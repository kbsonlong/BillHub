use crate::detect::{ImportFile, ParsedStatement};
use crate::{Error, ImportOptions, Result, normalize::RawStatementRow};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use uuid::Uuid;

pub struct LedgerStore {
    connection: Connection,
}

#[derive(Debug, Clone)]
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
    pub neutral_count: i64,
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
        connection.execute_batch(r"
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
        ")?;
        Ok(())
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
            let (kind, flow, lifecycle, count, amount) = row?;
            match (kind.as_str(), flow.as_str()) {
                ("payment", "expense") => summary.settled_expense_cents += amount.round() as i64,
                ("payment", "income") => summary.settled_income_cents += amount.round() as i64,
                ("refund", "income") => summary.refund_income_cents += amount.round() as i64,
                ("refund", "expense") => summary.refund_expense_cents += amount.round() as i64,
                _ => {}
            }
            if lifecycle == "pending" || lifecycle == "unknown" {
                summary.pending_count += count;
            }
            if flow == "neutral" {
                summary.neutral_count += count;
            }
        }
        Ok(summary)
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
