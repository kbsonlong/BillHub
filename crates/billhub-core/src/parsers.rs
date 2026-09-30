pub mod alipay;
pub mod wechat;

use crate::{
    ImportFile, ParsedStatement, PreviewSummary, Result,
    events::Provider,
    normalize::{RawStatementRow, SourceRow},
};

pub const WECHAT_PARSER_ID: &str = "wechat_xlsx_v1";
pub const ALIPAY_PARSER_ID: &str = "alipay_csv_v1";
pub const PARSER_VERSION: &str = "2";

pub trait StatementParser {
    fn provider(&self) -> Provider;
    fn parser_id(&self) -> &'static str;
    fn preview(&self, file: &ImportFile) -> Result<PreviewSummary>;
    fn parse(&self, file: &ImportFile) -> Result<ParsedStatement>;
    fn normalize(&self, row: &SourceRow) -> Result<RawStatementRow>;
}

pub fn make_parser(file: &ImportFile) -> Result<Box<dyn StatementParser + Send>> {
    if file.file_name.to_ascii_lowercase().ends_with(".xlsx") {
        Ok(Box::new(wechat::WechatXlsxV1))
    } else {
        Ok(Box::new(alipay::AlipayCsvV1))
    }
}
