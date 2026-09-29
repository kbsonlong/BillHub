use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Wechat,
    Alipay,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wechat => "wechat",
            Self::Alipay => "alipay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Payment,
    Refund,
    Transfer,
    TopUp,
    Withdrawal,
    Adjustment,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Payment => "payment",
            Self::Refund => "refund",
            Self::Transfer => "transfer",
            Self::TopUp => "top_up",
            Self::Withdrawal => "withdrawal",
            Self::Adjustment => "adjustment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CashFlow {
    Expense,
    Income,
    Neutral,
    Pending,
}

impl CashFlow {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Expense => "expense",
            Self::Income => "income",
            Self::Neutral => "neutral",
            Self::Pending => "pending",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Settled,
    Pending,
    Closed,
    Reversed,
    Unknown,
}

impl Lifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::Pending => "pending",
            Self::Closed => "closed",
            Self::Reversed => "reversed",
            Self::Unknown => "unknown",
        }
    }
}
