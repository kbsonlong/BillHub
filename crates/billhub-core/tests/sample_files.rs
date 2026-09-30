use billhub_core::{ImportFile, ImportOptions, LedgerStore, preview};

fn sample_path(suffix: &str) -> Option<std::path::PathBuf> {
    let root = std::env::current_dir()
        .ok()?
        .ancestors()
        .nth(2)?
        .to_path_buf();
    std::fs::read_dir(root).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        let name = path.file_name()?.to_str()?;
        (name.ends_with(suffix)).then_some(path)
    })
}

#[test]
fn parses_wechat_and_alipay_sample_headers_and_rows() {
    let Some(wechat_path) = sample_path(".xlsx") else {
        eprintln!("skipping: local WeChat XLSX is unavailable");
        return;
    };
    let Some(alipay_path) = sample_path(".csv") else {
        eprintln!("skipping: local Alipay CSV is unavailable");
        return;
    };
    let store = LedgerStore::in_memory().unwrap();
    let wechat = preview(&store, &ImportFile::from_path(&wechat_path).unwrap()).unwrap();
    assert_eq!(wechat.summary.parser_id, "wechat_xlsx_v1");
    assert_eq!(wechat.summary.record_count, 1528);
    assert_eq!(wechat.summary.accepted_count, 1528);
    assert_eq!(wechat.summary.rejected_count, 0);

    let alipay = preview(&store, &ImportFile::from_path(&alipay_path).unwrap()).unwrap();
    assert_eq!(alipay.summary.parser_id, "alipay_csv_v1");
    assert_eq!(alipay.summary.record_count, 514);
    assert_eq!(alipay.summary.accepted_count, 514);
    assert_eq!(alipay.summary.rejected_count, 0);
    assert!(
        alipay
            .summary
            .warnings
            .iter()
            .any(|w| w.contains("GB18030"))
    );
}

#[test]
fn imports_blocks_duplicates_and_deletes_batch_atomically() {
    let Some(path) = sample_path(".csv") else {
        eprintln!("skipping: local Alipay CSV is unavailable");
        return;
    };
    let store = LedgerStore::in_memory().unwrap();
    let file = ImportFile::from_path(&path).unwrap();
    let batch = billhub_core::import(&store, &file, ImportOptions::default()).unwrap();
    assert_eq!(batch.accepted_count, 514);
    assert!(billhub_core::import(&store, &file, ImportOptions::default()).is_err());
    assert_eq!(store.summary(false, false).unwrap().pending_count, 0);
    assert!(store.delete_batch(&batch.id).unwrap());
    assert!(store.batches().unwrap().is_empty());
    assert!(store.events(false, false, 10).unwrap().is_empty());
    let again = billhub_core::import(&store, &file, ImportOptions::default()).unwrap();
    assert_eq!(store.events(false, false, 10).unwrap().len(), 10);
    assert!(store.delete_batch(&again.id).unwrap());
}

#[test]
fn groups_imported_cash_flow_by_month_and_year() {
    let Some(path) = sample_path(".csv") else {
        eprintln!("skipping: local Alipay CSV is unavailable");
        return;
    };
    let store = LedgerStore::in_memory().unwrap();
    let file = ImportFile::from_path(&path).unwrap();
    billhub_core::import(&store, &file, ImportOptions::default()).unwrap();

    let monthly = store.monthly_summary().unwrap();
    let yearly = store.yearly_summary().unwrap();
    let monthly_income: i64 = monthly.iter().map(|period| period.income_cents).sum();
    let monthly_expense: i64 = monthly.iter().map(|period| period.expense_cents).sum();
    let yearly_income: i64 = yearly.iter().map(|period| period.income_cents).sum();
    let yearly_expense: i64 = yearly.iter().map(|period| period.expense_cents).sum();
    let summary = store.summary(false, false).unwrap();

    assert!(!monthly.is_empty());
    assert!(!yearly.is_empty());
    assert_eq!(
        monthly_income,
        summary.settled_income_cents - summary.refund_expense_cents
    );
    assert_eq!(
        monthly_expense,
        summary.settled_expense_cents - summary.refund_income_cents
    );
    assert_eq!(yearly_income, monthly_income);
    assert_eq!(yearly_expense, monthly_expense);
    assert!(
        monthly
            .windows(2)
            .all(|periods| periods[0].period < periods[1].period)
    );
    assert!(
        yearly
            .windows(2)
            .all(|periods| periods[0].period < periods[1].period)
    );
}
