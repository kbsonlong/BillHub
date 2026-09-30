pub mod amounts;
pub mod db;
pub mod detect;
pub mod events;
pub mod normalize;
pub mod parsers;
pub mod service;

pub use db::{
    BatchRecord, CategoryTotal, DailyExpense, EventPage, GamificationDay, GamificationSnapshot, GamificationTask, LedgerEventRecord, LedgerStore,
    LedgerSummary, MonthDashboard, PeriodSummary,
};
pub use detect::{ImportFile, ParsedStatement, PreviewSummary, detect_import_file};
pub use events::*;
pub use normalize::{RawStatementRow, SourceRow};
pub use parsers::make_parser;

pub use service::{ImportOptions, ImportPreview, import, preview};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HEADER_NOT_FOUND: 未找到可识别的账单表头")]
    HeaderNotFound,
    #[error("MISSING_TRANSACTION_ID: 行 {row} 缺少稳定交易编号")]
    MissingTransactionId { row: usize },
    #[error("UNKNOWN_STATUS: 行 {row} 未识别交易状态 {status:?}")]
    UnknownStatus { row: usize, status: String },
    #[error("AMOUNT_PARSE_FAILED: 行 {row} 金额无效 {amount:?}")]
    AmountParseFailed { row: usize, amount: String },
    #[error("DATE_PARSE_FAILED: 行 {row} 日期无效 {value:?}")]
    DateParseFailed { row: usize, value: String },
    #[error("DUPLICATE_FILE: 文件已导入为批次 {0}")]
    DuplicateFile(String),
    #[error("INVALID_EVENT_CLASSIFICATION: 不支持的交易类型或状态")]
    InvalidEventClassification,
    #[error("NO_EVENT_CLASSIFICATION_CHANGE: 请至少选择要修改的类型或状态")]
    NoEventClassificationChange,
    #[error("INVALID_MANUAL_ENTRY: 手工记账参数无效")]
    InvalidManualEntry,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Xlsx(#[from] calamine::XlsxError),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
