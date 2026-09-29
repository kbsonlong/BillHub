use crate::events::{CashFlow, EventKind, Lifecycle, Provider};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct SourceRow {
    pub row_number: usize,
    pub fields: BTreeMap<String, String>,
}

impl SourceRow {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .get(key)
            .filter(|v| !v.trim().is_empty())
            .map(String::as_str)
    }
    pub fn raw_json(&self) -> String {
        serde_json::to_string(&self.fields).expect("BTreeMap can serialize")
    }
    pub fn source_row_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.row_number.to_le_bytes());
        for (key, value) in &self.fields {
            hasher.update(key.as_bytes());
            hasher.update([0]);
            hasher.update(value.trim().as_bytes());
            hasher.update([1]);
        }
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStatementRow {
    pub source_row_number: usize,
    pub occurred_at: i64,
    pub event_kind: EventKind,
    pub cash_flow: CashFlow,
    pub lifecycle: Lifecycle,
    pub amount_cents: i64,
    pub counterparty: Option<String>,
    pub description: Option<String>,
    pub raw_category: Option<String>,
    pub funding_account: Option<String>,
    pub provider_transaction_id: String,
    pub merchant_order_id: Option<String>,
    pub raw_json: String,
    pub source_row_hash: String,
}

impl RawStatementRow {
    pub fn event_key(&self, provider: Provider) -> String {
        format!("{}:{}", provider.as_str(), self.provider_transaction_id)
    }
}
