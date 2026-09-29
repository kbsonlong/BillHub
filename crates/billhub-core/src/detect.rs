use crate::events::Provider;
use crate::normalize::RawStatementRow;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub type ImportIssue = (usize, String);

#[derive(Debug, Clone)]
pub struct ImportFile {
    pub file_name: String,
    pub data: Vec<u8>,
}

impl ImportFile {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            file_name: path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("statement")
                .to_owned(),
            data: std::fs::read(path)?,
        })
    }

    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewSummary {
    pub parser_id: String,
    pub parser_version: String,
    pub record_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub range_start: Option<i64>,
    pub range_end: Option<i64>,
    pub warnings: Vec<String>,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone)]
pub struct ParsedStatement {
    pub provider: Provider,
    pub summary: PreviewSummary,
    pub rows: Vec<RawStatementRow>,
    pub rejected_rows: Vec<ImportIssue>,
}

pub fn detect_import_file(
    file: &ImportFile,
) -> crate::Result<Box<dyn crate::parsers::StatementParser + Send>> {
    crate::parsers::make_parser(file)
}
