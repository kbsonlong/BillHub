use crate::detect::{ImportFile, ParsedStatement};
use crate::{Error, ImportOptions, Result, normalize::RawStatementRow};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use uuid::Uuid;

pub struct LedgerStore {
    connection: Connection,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchRecord {
    pub id: String,
    pub provider: String,
    pub file_sha256: String,
    pub record_count: i64,
    pub accepted_count: i64,
    pub rejected_count: i64,
    pub imported_at: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LedgerSummary {
    pub settled_expense_cents: i64,
    pub settled_income_cents: i64,
    pub refund_income_cents: i64,
    pub refund_expense_cents: i64,
    pub pending_count: i64,
    pub unknown_count: i64,
    pub neutral_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PeriodSummary {
    pub period: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryTotal {
    pub category: String,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyExpense {
    pub day: u32,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonthDashboard {
    pub period: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
    pub transaction_count: i64,
    pub pending_count: i64,
    pub categories: Vec<CategoryTotal>,
    pub daily_expenses: Vec<DailyExpense>,
    pub recent_events: Vec<LedgerEventRecord>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LedgerEventRecord {
    pub id: String,
    pub batch_id: String,
    pub provider: String,
    pub occurred_at: i64,
    pub event_kind: String,
    pub cash_flow: String,
    pub lifecycle: String,
    pub amount_cents: i64,
    pub counterparty: Option<String>,
    pub description: Option<String>,
    pub raw_category: Option<String>,
    pub funding_account: Option<String>,
    pub provider_transaction_id: String,
    pub merchant_order_id: Option<String>,
    pub raw_json: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EventPage {
    pub items: Vec<LedgerEventRecord>,
    pub total_count: i64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

impl LedgerStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::with_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    fn with_connection(connection: Connection) -> Result<Self> {
        let connection = connection;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Self::migrate(&connection)?;
        Ok(Self { connection })
    }

    fn migrate(connection: &Connection) -> Result<()> {
        let result = connection.execute_batch(r"
            BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS import_batches (
              id TEXT PRIMARY KEY,
              provider TEXT NOT NULL,
              parser_id TEXT NOT NULL,
              parser_version TEXT NOT NULL,
              original_filename TEXT NOT NULL,
              file_sha256 TEXT NOT NULL UNIQUE,
              imported_at INTEGER NOT NULL,
              range_start INTEGER,
              range_end INTEGER,
              record_count INTEGER NOT NULL,
              accepted_count INTEGER NOT NULL,
              rejected_count INTEGER NOT NULL,
              warnings_json TEXT NOT NULL,
              source_path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ledger_events (
              id TEXT PRIMARY KEY,
              batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
              provider TEXT NOT NULL,
              occurred_at INTEGER NOT NULL,
              settled_at INTEGER,
              event_kind TEXT NOT NULL,
              cash_flow TEXT NOT NULL,
              lifecycle TEXT NOT NULL,
              amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
              currency TEXT NOT NULL DEFAULT 'CNY',
              counterparty TEXT,
              description TEXT,
              category_id TEXT,
              funding_account_id TEXT,
              raw_category TEXT,
              provider_transaction_id TEXT NOT NULL,
              merchant_order_id TEXT,
              source_row_hash TEXT NOT NULL,
              raw_json TEXT NOT NULL,
              UNIQUE(provider, provider_transaction_id)
            );
            CREATE INDEX IF NOT EXISTS idx_ledger_occurred_at ON ledger_events(occurred_at);
            CREATE INDEX IF NOT EXISTS idx_ledger_filters ON ledger_events(cash_flow, event_kind, lifecycle, provider);
            CREATE TABLE IF NOT EXISTS import_issues (
              id TEXT PRIMARY KEY,
              batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
              source_row_number INTEGER NOT NULL,
              severity TEXT NOT NULL,
              code TEXT NOT NULL,
              message TEXT NOT NULL
            );
            UPDATE ledger_events
               SET occurred_at = occurred_at - 28800,
                   settled_at = CASE WHEN settled_at IS NULL THEN NULL ELSE settled_at - 28800 END
             WHERE batch_id IN (
               SELECT id FROM import_batches WHERE parser_version = '1'
             );
            UPDATE import_batches
               SET parser_version = '2',
                   range_start = CASE WHEN range_start IS NULL THEN NULL ELSE range_start - 28800 END,
                   range_end = CASE WHEN range_end IS NULL THEN NULL ELSE range_end - 28800 END
             WHERE parser_version = '1';
            COMMIT;
        ");
        if result.is_err() {
            let _ = connection.execute_batch("ROLLBACK");
        }
        result?;
        Self::migrate_cash_flow_classification(connection)?;
        Ok(())
    }

    fn migrate_cash_flow_classification(connection: &Connection) -> Result<()> {
        let rows = {
            let mut statement = connection.prepare(
                "SELECT ledger_events.id, ledger_events.provider, ledger_events.raw_json
                   FROM ledger_events
                   JOIN import_batches ON import_batches.id = ledger_events.batch_id
                  WHERE ledger_events.provider IN ('wechat', 'alipay')
                    AND import_batches.parser_version <> '3'",
            )?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        if rows.is_empty() {
            return Ok(());
        }

        let result = (|| -> Result<()> {
            connection.execute_batch("BEGIN IMMEDIATE;")?;
            for (id, provider, raw_json) in rows {
                let fields: std::collections::BTreeMap<String, String> =
                    serde_json::from_str(&raw_json)?;
                let direction = fields.get("收/支").map(String::as_str).unwrap_or("");
                let status = match provider.as_str() {
                    "wechat" => fields.get("当前状态").map(String::as_str).unwrap_or(""),
                    "alipay" => fields.get("交易状态").map(String::as_str).unwrap_or(""),
                    _ => continue,
                };
                let (kind, flow, lifecycle) = match provider.as_str() {
                    "wechat" => crate::parsers::wechat::classify(direction, status),
                    "alipay" => crate::parsers::alipay::classify(direction, status),
                    _ => continue,
                };
                connection.execute(
                    "UPDATE ledger_events
                        SET event_kind = ?1, cash_flow = ?2, lifecycle = ?3
                      WHERE id = ?4",
                    params![kind.as_str(), flow.as_str(), lifecycle.as_str(), id],
                )?;
            }
            connection.execute(
                "UPDATE import_batches
                    SET parser_version = '3'
                  WHERE provider IN ('wechat', 'alipay') AND parser_version <> '3'",
                [],
            )?;
            connection.execute_batch("COMMIT;")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = connection.execute_batch("ROLLBACK");
        }
        result
    }

    pub fn file_status(&self, sha256: &str) -> Result<Option<BatchRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id,provider,file_sha256,record_count,accepted_count,rejected_count,imported_at FROM import_batches WHERE file_sha256=?1"
        )?;
        let record = statement
            .query_row(params![sha256], |row| {
                Ok(BatchRecord {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    file_sha256: row.get(2)?,
                    record_count: row.get(3)?,
                    accepted_count: row.get(4)?,
                    rejected_count: row.get(5)?,
                    imported_at: row.get(6)?,
                })
            })
            .optional()?;
        Ok(record)
    }

    pub fn import(
        &self,
        file: &ImportFile,
        parsed: ParsedStatement,
        options: ImportOptions,
    ) -> Result<BatchRecord> {
        let sha256 = file.sha256();
        if !options.replace && self.file_status(&sha256)?.is_some() {
            return Err(Error::DuplicateFile(sha256));
        }
        let batch_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        let source_path = options
            .source_path
            .clone()
            .unwrap_or_else(|| file.file_name.clone());
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<BatchRecord> {
            if options.replace {
                self.connection.execute(
                    "DELETE FROM import_batches WHERE file_sha256=?1",
                    params![sha256],
                )?;
            }
            self.connection.execute(
                r"INSERT INTO import_batches
                  (id,provider,parser_id,parser_version,original_filename,file_sha256,imported_at,range_start,range_end,record_count,accepted_count,rejected_count,warnings_json,source_path)
                  VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    batch_id,
                    parsed.provider.as_str(),
                    parsed.summary.parser_id,
                    parsed.summary.parser_version,
                    file.file_name,
                    sha256,
                    now,
                    parsed.summary.range_start,
                    parsed.summary.range_end,
                    parsed.summary.record_count as i64,
                    parsed.rows.len() as i64,
                    parsed.summary.rejected_count as i64,
                    serde_json::to_string(&parsed.summary.warnings)?,
                    source_path,
                ],
            )?;
            for row in &parsed.rows {
                self.insert_event(&batch_id, parsed.provider, row)?;
            }
            let issue_id_prefix = batch_id.clone();
            for (row_number, message) in &parsed.rejected_rows {
                self.connection.execute(
                    "INSERT INTO import_issues(id,batch_id,source_row_number,severity,code,message) VALUES(?1,?2,?3,?4,?5,?6)",
                    params![format!("{}-{}", issue_id_prefix, row_number), batch_id, *row_number as i64, "error", "PARSE_FAILED", message],
                )?;
            }
            Ok(BatchRecord {
                id: batch_id,
                provider: parsed.provider.as_str().to_owned(),
                file_sha256: sha256,
                record_count: parsed.summary.record_count as i64,
                accepted_count: parsed.rows.len() as i64,
                rejected_count: parsed.summary.rejected_count as i64,
                imported_at: now,
            })
        })();
        match result {
            Ok(record) => {
                self.connection.execute_batch("COMMIT")?;
                Ok(record)
            }
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    fn insert_event(
        &self,
        batch_id: &str,
        provider: crate::events::Provider,
        row: &RawStatementRow,
    ) -> Result<()> {
        self.connection.execute(
            r"INSERT INTO ledger_events
              (id,batch_id,provider,occurred_at,event_kind,cash_flow,lifecycle,amount_cents,
               counterparty,description,raw_category,funding_account_id,provider_transaction_id,merchant_order_id,source_row_hash,raw_json)
              VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                row.event_key(provider),
                batch_id,
                provider.as_str(),
                row.occurred_at,
                row.event_kind.as_str(),
                row.cash_flow.as_str(),
                row.lifecycle.as_str(),
                row.amount_cents,
                row.counterparty,
                row.description,
                row.raw_category,
                row.funding_account,
                row.provider_transaction_id,
                row.merchant_order_id,
                row.source_row_hash,
                row.raw_json,
            ],
        )?;
        Ok(())
    }

    pub fn delete_batch(&self, batch_id: &str) -> Result<bool> {
        let affected = self
            .connection
            .execute("DELETE FROM import_batches WHERE id=?1", params![batch_id])?;
        Ok(affected > 0)
    }

    pub fn batches(&self) -> Result<Vec<BatchRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id,provider,file_sha256,record_count,accepted_count,rejected_count,imported_at FROM import_batches ORDER BY imported_at DESC"
        )?;
        let records = statement
            .query_map([], |row| {
                Ok(BatchRecord {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    file_sha256: row.get(2)?,
                    record_count: row.get(3)?,
                    accepted_count: row.get(4)?,
                    rejected_count: row.get(5)?,
                    imported_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn events(
        &self,
        include_neutral: bool,
        include_pending: bool,
        limit: usize,
    ) -> Result<Vec<LedgerEventRecord>> {
        let filter = match (include_neutral, include_pending) {
            (true, true) => "1=1",
            (true, false) => "lifecycle='settled'",
            (false, true) => "cash_flow!='neutral'",
            (false, false) => "cash_flow!='neutral' AND lifecycle='settled'",
        };
        let sql = format!(
            "SELECT id,batch_id,provider,occurred_at,event_kind,cash_flow,lifecycle,amount_cents,counterparty,description,raw_category,funding_account_id,provider_transaction_id,merchant_order_id,raw_json FROM ledger_events WHERE {filter} ORDER BY occurred_at DESC LIMIT ?1"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let records = statement
            .query_map(params![limit as i64], map_event)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn event_page(
        &self,
        page: usize,
        page_size: usize,
        provider: Option<String>,
        cash_flow: Option<String>,
        lifecycle: Option<String>,
    ) -> Result<EventPage> {
        self.event_page_filtered(page, page_size, provider, cash_flow, lifecycle, None)
    }

    pub fn event_page_filtered(
        &self,
        page: usize,
        page_size: usize,
        provider: Option<String>,
        cash_flow: Option<String>,
        lifecycle: Option<String>,
        period: Option<String>,
    ) -> Result<EventPage> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let provider = provider.filter(|value| matches!(value.as_str(), "wechat" | "alipay"));
        let cash_flow = cash_flow.filter(|value| {
            matches!(
                value.as_str(),
                "expense" | "income" | "refund" | "neutral" | "pending"
            )
        });
        let lifecycle = lifecycle.filter(|value| {
            matches!(
                value.as_str(),
                "settled" | "pending" | "closed" | "reversed" | "unknown"
            )
        });
        let period = period.filter(|value| is_period(value));
        let filters = "(?1 IS NULL OR provider = ?1)
                       AND (?2 IS NULL
                            OR (?2 = 'expense' AND event_kind = 'payment' AND cash_flow = 'expense')
                            OR (?2 = 'income' AND event_kind = 'payment' AND cash_flow = 'income')
                            OR (?2 = 'refund' AND event_kind = 'refund')
                            OR (?2 = 'neutral' AND event_kind != 'refund' AND cash_flow = 'neutral')
                            OR (?2 = 'pending' AND cash_flow = 'pending'))
                       AND (?3 IS NULL OR lifecycle = ?3)
                       AND (?4 IS NULL OR substr(strftime('%Y-%m', occurred_at, 'unixepoch', '+8 hours'), 1, length(?4)) = ?4)";
        let total_count = self.connection.query_row(
            &format!("SELECT COUNT(*) FROM ledger_events WHERE {filters}"),
            params![
                provider.as_deref(),
                cash_flow.as_deref(),
                lifecycle.as_deref(),
                period.as_deref()
            ],
            |row| row.get(0),
        )?;
        let offset = ((page - 1) * page_size) as i64;
        let sql = format!(
            "SELECT id,batch_id,provider,occurred_at,event_kind,cash_flow,lifecycle,amount_cents,counterparty,description,raw_category,funding_account_id,provider_transaction_id,merchant_order_id,raw_json
               FROM ledger_events
              WHERE {filters}
              ORDER BY occurred_at DESC
              LIMIT ?5 OFFSET ?6"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let items = statement
            .query_map(
                params![
                    provider.as_deref(),
                    cash_flow.as_deref(),
                    lifecycle.as_deref(),
                    period.as_deref(),
                    page_size as i64,
                    offset
                ],
                map_event,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let total_pages = ((total_count as usize) + page_size - 1) / page_size;
        Ok(EventPage {
            items,
            total_count,
            page,
            page_size,
            total_pages,
        })
    }

    pub fn summary(&self, include_neutral: bool, include_pending: bool) -> Result<LedgerSummary> {
        let mut filter_clauses = Vec::new();
        if !include_neutral {
            filter_clauses.push("cash_flow != 'neutral'".to_owned());
        }
        if !include_pending {
            filter_clauses.push("lifecycle = 'settled'".to_owned());
        }
        let filter = if filter_clauses.is_empty() {
            "1=1"
        } else {
            &filter_clauses.join(" AND ")
        };
        let mut statement = self.connection.prepare(&format!(
            "SELECT event_kind,cash_flow,lifecycle,COUNT(*),TOTAL(amount_cents) FROM ledger_events WHERE {filter} GROUP BY event_kind,cash_flow,lifecycle"
        ))?;
        let mut summary = LedgerSummary::default();
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })?;
        for row in rows {
            let (kind, flow, _, _, amount) = row?;
            match (kind.as_str(), flow.as_str()) {
                ("payment", "expense") => summary.settled_expense_cents += amount.round() as i64,
                ("payment", "income") => summary.settled_income_cents += amount.round() as i64,
                ("refund", "income") => summary.refund_income_cents += amount.round() as i64,
                ("refund", "expense") => summary.refund_expense_cents += amount.round() as i64,
                _ => {}
            }
        }
        let (pending_count, unknown_count, neutral_count) = self.connection.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN lifecycle = 'pending' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN lifecycle = 'unknown' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN cash_flow = 'neutral' THEN 1 ELSE 0 END), 0)
             FROM ledger_events",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        summary.pending_count = pending_count;
        summary.unknown_count = unknown_count;
        summary.neutral_count = neutral_count;
        Ok(summary)
    }

    pub fn update_event_classification(
        &self,
        event_id: &str,
        event_kind: &str,
        lifecycle: &str,
    ) -> Result<bool> {
        Ok(self.update_events_classification(
            &[event_id.to_owned()],
            Some(event_kind),
            Some(lifecycle),
        )? > 0)
    }

    pub fn update_events_classification(
        &self,
        event_ids: &[String],
        event_kind: Option<&str>,
        lifecycle: Option<&str>,
    ) -> Result<usize> {
        if event_kind.is_none() && lifecycle.is_none() {
            return Err(Error::NoEventClassificationChange);
        }
        if event_kind.is_some_and(|value| !is_event_kind(value))
            || lifecycle.is_some_and(|value| !is_lifecycle(value))
        {
            return Err(Error::InvalidEventClassification);
        }
        if event_ids.is_empty() {
            return Ok(0);
        }

        self.connection.execute_batch("BEGIN IMMEDIATE;")?;
        let result = (|| -> Result<usize> {
            let mut updated_count = 0;
            for event_id in event_ids {
                let event = self
                    .connection
                    .query_row(
                        "SELECT event_kind, lifecycle, raw_json FROM ledger_events WHERE id = ?1",
                        params![event_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                            ))
                        },
                    )
                    .optional()?;
                let Some((current_kind, current_lifecycle, raw_json)) = event else {
                    continue;
                };
                let event_kind = event_kind.unwrap_or(&current_kind);
                let lifecycle = lifecycle.unwrap_or(&current_lifecycle);
                let fields: std::collections::BTreeMap<String, String> =
                    serde_json::from_str(&raw_json)?;
                let cash_flow = manual_cash_flow(
                    event_kind,
                    lifecycle,
                    fields.get("收/支").map(String::as_str),
                );
                updated_count += self.connection.execute(
                    "UPDATE ledger_events
                        SET event_kind = ?1, cash_flow = ?2, lifecycle = ?3
                      WHERE id = ?4",
                    params![event_kind, cash_flow, lifecycle, event_id],
                )?;
            }
            self.connection.execute_batch("COMMIT;")?;
            Ok(updated_count)
        })();
        if result.is_err() {
            let _ = self.connection.execute_batch("ROLLBACK;");
        }
        result
    }

    pub fn monthly_summary(&self) -> Result<Vec<PeriodSummary>> {
        self.period_summary("%Y-%m")
    }

    pub fn yearly_summary(&self) -> Result<Vec<PeriodSummary>> {
        self.period_summary("%Y")
    }

    pub fn month_dashboard(&self, period: &str) -> Result<Option<MonthDashboard>> {
        if !is_period(period) || period.len() != 7 {
            return Ok(None);
        }
        let Ok(start_date) = chrono::NaiveDate::parse_from_str(&format!("{period}-01"), "%Y-%m-%d") else {
            return Ok(None);
        };
        let Some(end_date) = start_date.checked_add_months(chrono::Months::new(1)) else {
            return Ok(None);
        };
        let to_epoch = |date: chrono::NaiveDate| {
            date.and_hms_opt(0, 0, 0)
                .expect("midnight is valid")
                .and_utc()
                .timestamp()
                - 28_800
        };
        let start = to_epoch(start_date);
        let end = to_epoch(end_date);
        let (income_cents, expense_cents, transaction_count, pending_count) = self.connection.query_row(
            "SELECT
                COALESCE(SUM(CASE
                    WHEN event_kind='payment' AND cash_flow='income' THEN amount_cents
                    WHEN event_kind='refund' AND cash_flow='expense' THEN -amount_cents
                    ELSE 0 END), 0),
                COALESCE(SUM(CASE
                    WHEN event_kind='payment' AND cash_flow='expense' THEN amount_cents
                    WHEN event_kind='refund' AND cash_flow='income' THEN -amount_cents
                    ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN lifecycle='settled' AND event_kind IN ('payment','refund')
                    AND cash_flow IN ('income','expense') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN lifecycle IN ('pending','unknown') THEN 1 ELSE 0 END), 0)
             FROM ledger_events WHERE occurred_at >= ?1 AND occurred_at < ?2",
            params![start, end],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?),),
        )?;

        let mut category_statement = self.connection.prepare(
            "SELECT COALESCE(NULLIF(TRIM(raw_category), ''), '其他') AS category,
                    SUM(CASE
                        WHEN event_kind='payment' AND cash_flow='expense' THEN amount_cents
                        WHEN event_kind='refund' AND cash_flow='income' THEN -amount_cents
                        ELSE 0 END) AS amount_cents
               FROM ledger_events
              WHERE occurred_at >= ?1 AND occurred_at < ?2 AND lifecycle='settled'
                AND ((event_kind='payment' AND cash_flow='expense') OR (event_kind='refund' AND cash_flow='income'))
              GROUP BY category HAVING amount_cents > 0 ORDER BY amount_cents DESC",
        )?;
        let categories = category_statement
            .query_map(params![start, end], |row| {
                Ok(CategoryTotal { category: row.get(0)?, amount_cents: row.get(1)? })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut daily_statement = self.connection.prepare(
            "SELECT CAST(strftime('%d', occurred_at, 'unixepoch', '+8 hours') AS INTEGER) AS day,
                    SUM(CASE
                        WHEN event_kind='payment' AND cash_flow='expense' THEN amount_cents
                        WHEN event_kind='refund' AND cash_flow='income' THEN -amount_cents
                        ELSE 0 END) AS amount_cents
               FROM ledger_events
              WHERE occurred_at >= ?1 AND occurred_at < ?2 AND lifecycle='settled'
                AND ((event_kind='payment' AND cash_flow='expense') OR (event_kind='refund' AND cash_flow='income'))
              GROUP BY day ORDER BY day",
        )?;
        let daily_expenses = daily_statement
            .query_map(params![start, end], |row| {
                Ok(DailyExpense { day: row.get(0)?, amount_cents: row.get(1)? })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut recent_statement = self.connection.prepare(
            "SELECT id,batch_id,provider,occurred_at,event_kind,cash_flow,lifecycle,amount_cents,counterparty,description,raw_category,funding_account_id,provider_transaction_id,merchant_order_id,raw_json
               FROM ledger_events WHERE occurred_at >= ?1 AND occurred_at < ?2
              ORDER BY occurred_at DESC LIMIT 5",
        )?;
        let recent_events = recent_statement
            .query_map(params![start, end], map_event)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let net_cents = income_cents - expense_cents;
        Ok(Some(MonthDashboard {
            period: period.to_owned(), income_cents, expense_cents, net_cents,
            transaction_count, pending_count, categories, daily_expenses, recent_events,
        }))
    }

    fn period_summary(&self, format: &str) -> Result<Vec<PeriodSummary>> {
        let mut statement = self.connection.prepare(
            "SELECT strftime(?1, occurred_at, 'unixepoch', '+8 hours'),
                    COALESCE(SUM(CASE
                        WHEN event_kind='payment' AND cash_flow='income' THEN amount_cents
                        WHEN event_kind='refund' AND cash_flow='expense' THEN -amount_cents
                        ELSE 0
                    END), 0),
                    COALESCE(SUM(CASE
                        WHEN event_kind='payment' AND cash_flow='expense' THEN amount_cents
                        WHEN event_kind='refund' AND cash_flow='income' THEN -amount_cents
                        ELSE 0
                    END), 0),
                    COUNT(*)
             FROM ledger_events
             WHERE lifecycle='settled'
               AND event_kind IN ('payment', 'refund')
               AND cash_flow IN ('income', 'expense')
             GROUP BY strftime(?1, occurred_at, 'unixepoch', '+8 hours')
             ORDER BY strftime(?1, occurred_at, 'unixepoch', '+8 hours') ASC",
        )?;
        let rows = statement.query_map(params![format], |row| {
            let period = row.get::<_, String>(0)?;
            let income_cents = row.get::<_, i64>(1)?;
            let expense_cents = row.get::<_, i64>(2)?;
            Ok(PeriodSummary {
                period,
                income_cents,
                expense_cents,
                net_cents: income_cents - expense_cents,
                transaction_count: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn is_event_kind(value: &str) -> bool {
    matches!(
        value,
        "payment" | "refund" | "transfer" | "top_up" | "withdrawal" | "adjustment"
    )
}

fn is_lifecycle(value: &str) -> bool {
    matches!(
        value,
        "settled" | "pending" | "closed" | "reversed" | "unknown"
    )
}

fn is_period(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() == 4 {
        return bytes.iter().all(u8::is_ascii_digit);
    }
    if bytes.len() != 7 || bytes[4] != b'-' || !bytes[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let month = std::str::from_utf8(&bytes[5..])
        .ok()
        .and_then(|value| value.parse::<u8>().ok());
    matches!(month, Some(1..=12))
}

fn manual_cash_flow(event_kind: &str, lifecycle: &str, direction: Option<&str>) -> &'static str {
    if lifecycle != "settled" {
        return "pending";
    }
    if matches!(
        event_kind,
        "transfer" | "top_up" | "withdrawal" | "adjustment"
    ) {
        return "neutral";
    }
    match direction {
        Some("支出") => "expense",
        Some("收入") => "income",
        _ => "neutral",
    }
}

fn map_event(row: &rusqlite::Row) -> rusqlite::Result<LedgerEventRecord> {
    Ok(LedgerEventRecord {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        provider: row.get(2)?,
        occurred_at: row.get(3)?,
        event_kind: row.get(4)?,
        cash_flow: row.get(5)?,
        lifecycle: row.get(6)?,
        amount_cents: row.get(7)?,
        counterparty: row.get(8)?,
        description: row.get(9)?,
        raw_category: row.get(10)?,
        funding_account: row.get(11)?,
        provider_transaction_id: row.get(12)?,
        merchant_order_id: row.get(13)?,
        raw_json: row.get(14)?,
    })
}
