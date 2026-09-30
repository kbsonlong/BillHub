use super::{StatementParser, WECHAT_PARSER_ID};
use crate::{
    Error, ImportFile, ParsedStatement, PreviewSummary, Provider, Result,
    amounts::parse_positive_cents,
    events::{CashFlow, EventKind, Lifecycle},
    normalize::{RawStatementRow, SourceRow},
};
use calamine::{Data, Reader, Xlsx};
use chrono::{Duration, FixedOffset, NaiveDate, NaiveDateTime};
use std::io::Cursor;

pub struct WechatXlsxV1;

const REQUIRED_HEADERS: [&str; 5] = ["交易时间", "交易单号", "当前状态", "金额(元)", "收/支"];

impl StatementParser for WechatXlsxV1 {
    fn provider(&self) -> Provider {
        Provider::Wechat
    }
    fn parser_id(&self) -> &'static str {
        WECHAT_PARSER_ID
    }

    fn preview(&self, file: &ImportFile) -> Result<PreviewSummary> {
        let statement = self.parse(file)?;
        Ok(statement.summary)
    }

    fn parse(&self, file: &ImportFile) -> Result<ParsedStatement> {
        let cursor = Cursor::new(&file.data);
        let mut workbook = Xlsx::new(cursor)?;
        let range = workbook
            .worksheet_range_at(0)
            .ok_or(Error::HeaderNotFound)??;
        let mut header_index = None;
        for (index, row) in range.rows().enumerate() {
            let values: Vec<String> = row.iter().map(cell_text).collect();
            if REQUIRED_HEADERS
                .iter()
                .all(|h| values.iter().any(|v| v == h))
            {
                header_index = Some((index, values));
                break;
            }
        }
        let (header_index, headers) = header_index.ok_or(Error::HeaderNotFound)?;
        let mut rows = Vec::new();
        for (offset, row) in range.rows().enumerate().skip(header_index + 1) {
            let values: Vec<String> = row.iter().map(cell_text).collect();
            if values.iter().all(|v| v.trim().is_empty()) {
                continue;
            }
            let mut fields = std::collections::BTreeMap::new();
            for (index, header) in headers.iter().enumerate() {
                if header.trim().is_empty() {
                    continue;
                }
                fields.insert(
                    header.clone(),
                    values.get(index).cloned().unwrap_or_default(),
                );
            }
            rows.push(SourceRow {
                row_number: header_index + offset + 2,
                fields,
            });
        }

        let mut accepted = Vec::new();
        let mut issues = Vec::new();
        for row in &rows {
            match self.normalize(row) {
                Ok(event) => accepted.push(event),
                Err(error) => issues.push((row.row_number, error.to_string())),
            }
        }
        let range_start = accepted.iter().map(|r| r.occurred_at).min();
        let range_end = accepted.iter().map(|r| r.occurred_at).max();
        let summary = PreviewSummary {
            parser_id: self.parser_id().to_owned(),
            parser_version: super::PARSER_VERSION.to_owned(),
            record_count: rows.len(),
            accepted_count: accepted.len(),
            rejected_count: issues.len(),
            range_start,
            range_end,
            issues: issues.clone(),
            warnings: Vec::new(),
        };
        Ok(ParsedStatement {
            provider: self.provider(),
            summary,
            rows: accepted,
            rejected_rows: issues,
        })
    }

    fn normalize(&self, row: &SourceRow) -> Result<RawStatementRow> {
        let amount_cents = parse_positive_cents(row.get("金额(元)").unwrap_or(""), row.row_number)?;
        let provider_transaction_id = row
            .get("交易单号")
            .ok_or(Error::MissingTransactionId {
                row: row.row_number,
            })?
            .to_owned();
        let direction = row.get("收/支").unwrap_or("unknown");
        let status = row.get("当前状态").unwrap_or("");
        let (event_kind, cash_flow, lifecycle) = classify(direction, status);
        Ok(RawStatementRow {
            source_row_number: row.row_number,
            occurred_at: parse_datetime(row.get("交易时间").unwrap_or("")).ok_or(
                Error::DateParseFailed {
                    row: row.row_number,
                    value: row.get("交易时间").unwrap_or("").to_owned(),
                },
            )?,
            event_kind,
            cash_flow,
            lifecycle,
            amount_cents,
            counterparty: row.get("交易对方").map(str::to_owned),
            description: row.get("商品").map(str::to_owned),
            raw_category: row.get("交易类型").map(str::to_owned),
            funding_account: row.get("支付方式").filter(|v| *v != "/").map(str::to_owned),
            provider_transaction_id,
            merchant_order_id: row.get("商户单号").map(str::to_owned),
            raw_json: row.raw_json(),
            source_row_hash: row.source_row_hash(),
        })
    }
}

fn classify(direction: &str, status: &str) -> (EventKind, CashFlow, Lifecycle) {
    if status.contains("退款") {
        let flow = if direction == "支出" {
            CashFlow::Income
        } else {
            CashFlow::Expense
        };
        (EventKind::Refund, flow, Lifecycle::Settled)
    } else if status == "支付成功" {
        let flow = if direction == "支出" {
            CashFlow::Expense
        } else {
            CashFlow::Income
        };
        (EventKind::Payment, flow, Lifecycle::Settled)
    } else {
        (EventKind::Adjustment, CashFlow::Pending, Lifecycle::Unknown)
    }
}

pub fn parse_datetime(value: &str) -> Option<i64> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return china_timestamp(dt);
    }
    let numeric: f64 = value.parse().ok()?;
    let serial = numeric.trunc() as i64;
    let seconds = ((numeric - numeric.trunc()) * 86_400f64).round() as i64;
    // Excel's serial 1 is 1900-01-01 and includes its historical 1900 leap-year bug.
    let date = NaiveDate::from_ymd_opt(1899, 12, 30)?.checked_add_signed(Duration::days(serial))?;
    let datetime = date
        .and_hms_opt(0, 0, 0)?
        .checked_add_signed(Duration::seconds(seconds))?;
    china_timestamp(datetime)
}

fn china_timestamp(datetime: NaiveDateTime) -> Option<i64> {
    let offset = FixedOffset::east_opt(8 * 60 * 60)?;
    datetime
        .and_local_timezone(offset)
        .single()
        .map(|datetime| datetime.timestamp())
}

fn cell_text(value: &Data) -> String {
    match value {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_owned(),
        Data::Float(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_excel_serial_dates_with_leap_year_bug() {
        let actual = parse_datetime("46292.52381944445").unwrap();
        assert_eq!(actual, 1_790_483_658);
    }

    #[test]
    fn classifies_partial_refunds_as_independent_events() {
        assert_eq!(
            classify("支出", "已退款(¥1.11)"),
            (EventKind::Refund, CashFlow::Income, Lifecycle::Settled)
        );
        assert_eq!(
            classify("收入", "已全额退款"),
            (EventKind::Refund, CashFlow::Expense, Lifecycle::Settled)
        );
    }
}
