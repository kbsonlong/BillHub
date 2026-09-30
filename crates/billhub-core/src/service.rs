use crate::detect::{ImportFile, PreviewSummary, detect_import_file};
use crate::{LedgerStore, Result, db::BatchRecord};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub replace: bool,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    pub summary: PreviewSummary,
    pub file_sha256: String,
    pub existing_batch_id: Option<String>,
}

pub fn preview(store: &LedgerStore, file: &ImportFile) -> Result<ImportPreview> {
    let parser = detect_import_file(file)?;
    let summary = parser.preview(file)?;
    let sha256 = file.sha256();
    let existing_batch_id = store.file_status(&sha256)?.map(|batch| batch.id);
    Ok(ImportPreview {
        summary,
        file_sha256: sha256,
        existing_batch_id,
    })
}

pub fn import(
    store: &LedgerStore,
    file: &ImportFile,
    options: ImportOptions,
) -> Result<BatchRecord> {
    let parser = detect_import_file(file)?;
    let parsed = parser.parse(file)?;
    store.import(file, parsed, options)
}
