use super::{ALIPAY_PARSER_ID, PARSER_VERSION, StatementParser};
use crate::{
    Error, ImportFile, ParsedStatement, PreviewSummary, Provider, Result,
    amounts::parse_positive_cents,
    events::{CashFlow, EventKind, Lifecycle},
    normalize::{RawStatementRow, SourceRow},
};
use chrono::NaiveDateTime;
use csv::ReaderBuilder;
use encoding_rs::GB18030;
use std::collections::BTreeMap;

pub struct AlipayCsvV1;

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const REQUIRED_HEADERS: [&str; 5] = ["交易时间", "交易订单号", "交易状态", "金额", "收/支"];

fn decode(file: &ImportFile) -> (String, &'static str) {
    if file.data.starts_with(&UTF8_BOM) {
        (
            String::from_utf8_lossy(&file.data[UTF8_BOM.len()..]).into_owned(),
            "UTF-8 BOM",
        )
    } else if let Ok(text) = std::str::from_utf8(&file.data) {
        (text.to_owned(), "UTF-8")
    } else {
        let (text, _, _) = GB18030.decode(&file.data);
        (text.into_owned(), "GB18030")
    }
}

impl StatementParser for AlipayCsvV1 {
    fn provider(&self) -> Provider {
        Provider::Alipay
    }
    fn parser_id(&self) -> &'static str {
        ALIPAY_PARSER_ID
    }

    fn preview(&self, file: &ImportFile) -> Result<PreviewSummary> {
        Ok(self.parse(file)?.summary)
    }

    fn parse(&self, file: &ImportFile) -> Result<ParsedStatement> {
        let (text, encoding) = decode(file);
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(text.as_bytes());
        let mut header_line = None;
        let mut headers = Vec::new();
        let mut records: Vec<Vec<String>> = Vec::new();
        for record in reader.records() {
            let record = record?;
            if header_line.is_some() {
                records.push(record.iter().map(|v| v.trim().to_owned()).collect());
                continue;
            }
            let values: Vec<String> = record.iter().map(str::trim).map(str::to_owned).collect();
            if header_line.is_none() {
                let first = values.first().map(String::as_str).unwrap_or("");
                if first == "交易时间" && values.iter().any(|v| v.contains("交易订单号")) {
                    if !REQUIRED_HEADERS
                        .iter()
                        .all(|h| values.iter().any(|v| v == h))
                    {
                        return Err(Error::HeaderNotFound);
                    }
                    header_line = Some(record.position().expect("csv position").line());
                    headers = values;
                }
            }
        }
        let header_line = header_line.ok_or(Error::HeaderNotFound)?;

        let mut rows = Vec::new();
        for (offset, values) in records.into_iter().enumerate() {
            if values.iter().all(|v| v.trim().is_empty()) {
                continue;
            }
            let mut fields = BTreeMap::new();
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
                row_number: header_line as usize + offset + 1,
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
        let summary = PreviewSummary {
            parser_id: self.parser_id().to_owned(),
            parser_version: PARSER_VERSION.to_owned(),
            record_count: rows.len(),
            accepted_count: accepted.len(),
            rejected_count: issues.len(),
            range_start: accepted.iter().map(|r| r.occurred_at).min(),
            range_end: accepted.iter().map(|r| r.occurred_at).max(),
            issues: issues.clone(),
            warnings: Vec::new(),
        };
        let mut parsed = ParsedStatement {
            provider: self.provider(),
            summary,
            rows: accepted,
            rejected_rows: issues,
        };
        parsed.summary.warnings.push(format!("encoding={encoding}"));
        Ok(parsed)
    }

    fn normalize(&self, row: &SourceRow) -> Result<RawStatementRow> {
        let amount_cents = parse_positive_cents(row.get("金额").unwrap_or(""), row.row_number)?;
        let provider_transaction_id = row
            .get("交易订单号")
            .ok_or(Error::MissingTransactionId {
                row: row.row_number,
            })?
            .trim_end_matches(['\t', '\r', '\n', ' '])
            .to_owned();
        if provider_transaction_id.is_empty() {
            return Err(Error::MissingTransactionId {
                row: row.row_number,
            });
        }
        let date_text = row.get("交易时间").unwrap_or("");
        let occurred_at = NaiveDateTime::parse_from_str(date_text, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| Error::DateParseFailed {
                row: row.row_number,
                value: date_text.to_owned(),
            })?
            .and_utc()
            .timestamp();
        let direction = row.get("收/支").unwrap_or("unknown");
        let status = row.get("交易状态").unwrap_or("");
        let (event_kind, cash_flow, lifecycle) = classify(direction, status);
        Ok(RawStatementRow {
            source_row_number: row.row_number,
            occurred_at,
            event_kind,
            cash_flow,
            lifecycle,
            amount_cents,
            counterparty: row.get("交易对方").map(str::to_owned),
            description: row.get("商品说明").map(str::to_owned),
            raw_category: row.get("交易分类").map(str::to_owned),
            funding_account: row
                .get("收/付款方式")
                .map(|v| v.split('&').next().unwrap_or(v).trim().to_owned())
                .filter(|v| !v.is_empty() && *v != "/"),
            provider_transaction_id,
            merchant_order_id: row
                .get("商家订单号")
                .map(|v| v.trim_end_matches(['\t', '\r', '\n', ' ']).to_owned())
                .filter(|v| !v.is_empty()),
            raw_json: row.raw_json(),
            source_row_hash: row.source_row_hash(),
        })
    }
}

fn classify(direction: &str, status: &str) -> (EventKind, CashFlow, Lifecycle) {
    if direction == "不计收支" {
        return (EventKind::Adjustment, CashFlow::Neutral, Lifecycle::Settled);
    }
    if status == "退款成功" {
        let flow = if direction == "支出" {
            CashFlow::Income
        } else {
            CashFlow::Expense
        };
        (EventKind::Refund, flow, Lifecycle::Settled)
    } else if status == "交易成功" {
        let flow = if direction == "支出" {
            CashFlow::Expense
        } else if direction == "收入" {
            CashFlow::Income
        } else {
            CashFlow::Pending
        };
        (EventKind::Payment, flow, Lifecycle::Settled)
    } else if status == "交易关闭" {
        (EventKind::Adjustment, CashFlow::Pending, Lifecycle::Closed)
    } else if status == "等待确认收货" {
        (EventKind::Adjustment, CashFlow::Pending, Lifecycle::Pending)
    } else {
        (EventKind::Adjustment, CashFlow::Pending, Lifecycle::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_normalize_fields() {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("交易时间".to_string(), "2026-09-28 12:36:42".to_string());
        fields.insert("交易分类".to_string(), "爱车养车".to_string());
        fields.insert("交易对方".to_string(), "高速官方ETC".to_string());
        fields.insert("商品说明".to_string(), "ETC通行费".to_string());
        fields.insert("收/支".to_string(), "支出".to_string());
        fields.insert("金额".to_string(), "6.33".to_string());
        fields.insert(
            "收/付款方式".to_string(),
            "招商银行信用卡(4679)".to_string(),
        );
        fields.insert("交易状态".to_string(), "交易成功".to_string());
        fields.insert(
            "交易订单号".to_string(),
            "2026092823001487661428537599\t".to_string(),
        );
        let row = crate::normalize::SourceRow {
            row_number: 25,
            fields,
        };
        let parsed = AlipayCsvV1.normalize(&row).unwrap();
        assert_eq!(parsed.cash_flow, crate::events::CashFlow::Expense);
        assert_eq!(parsed.event_kind, crate::events::EventKind::Payment);
        println!("{parsed:?}");
    }
}
