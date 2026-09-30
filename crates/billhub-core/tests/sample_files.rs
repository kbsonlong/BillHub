use billhub_core::{ImportFile, ImportOptions, LedgerStore, preview};

fn synthetic_alipay_file() -> ImportFile {
    let csv = concat!(
        "交易时间,交易分类,交易对方,商品说明,收/支,金额,收/付款方式,交易状态,交易订单号,商家订单号\n",
        "2026-09-30 23:30:00,消费,商户甲,九月消费,支出,100.00,余额,交易成功,expense-1,merchant-1\n",
        "2026-10-01 00:15:00,退款,商户甲,十月退款,支出,20.00,余额,退款成功,refund-1,merchant-1\n",
        "2026-10-01 10:00:00,收入,客户甲,十月收入,收入,50.00,余额,交易成功,income-1,merchant-2\n",
        "2026-10-02 10:00:00,退款,客户甲,收入退款,收入,10.00,余额,退款成功,refund-2,merchant-2\n",
        "2026-10-03 10:00:00,转账,本人账户,中性资金,不计收支,999.00,余额,交易成功,neutral-1,merchant-3\n",
        "2026-10-04 10:00:00,消费,商户乙,待确认消费,支出,30.00,余额,等待确认收货,pending-1,merchant-4\n",
    );
    ImportFile {
        file_name: "synthetic-alipay.csv".to_owned(),
        data: csv.as_bytes().to_vec(),
    }
}

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

#[test]
fn period_summaries_use_china_time_and_net_refunds() {
    let file = synthetic_alipay_file();
    let store = LedgerStore::in_memory().unwrap();
    billhub_core::import(&store, &file, ImportOptions::default()).unwrap();

    let monthly = store.monthly_summary().unwrap();
    assert_eq!(monthly.len(), 2);
    assert_eq!(monthly[0].period, "2026-09");
    assert_eq!(monthly[0].income_cents, 0);
    assert_eq!(monthly[0].expense_cents, 10_000);
    assert_eq!(monthly[0].net_cents, -10_000);
    assert_eq!(monthly[0].transaction_count, 1);
    assert_eq!(monthly[1].period, "2026-10");
    assert_eq!(monthly[1].income_cents, 4_000);
    assert_eq!(monthly[1].expense_cents, -2_000);
    assert_eq!(monthly[1].net_cents, 6_000);
    assert_eq!(monthly[1].transaction_count, 3);

    let yearly = store.yearly_summary().unwrap();
    assert_eq!(yearly.len(), 1);
    assert_eq!(yearly[0].income_cents, 4_000);
    assert_eq!(yearly[0].expense_cents, 8_000);
    assert_eq!(yearly[0].net_cents, -4_000);
    assert_eq!(yearly[0].transaction_count, 4);

    let refund = store
        .events(true, true, 10)
        .unwrap()
        .into_iter()
        .find(|event| event.provider_transaction_id == "refund-1")
        .unwrap();
    let expected = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:15:00+08:00")
        .unwrap()
        .timestamp();
    assert_eq!(refund.occurred_at, expected);
}

#[test]
fn migrates_v1_timestamps_once() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("billhub.sqlite3");
    let file = synthetic_alipay_file();
    {
        let store = LedgerStore::open(&database_path).unwrap();
        billhub_core::import(&store, &file, ImportOptions::default()).unwrap();
    }
    {
        let connection = rusqlite::Connection::open(&database_path).unwrap();
        connection
            .execute(
                "UPDATE ledger_events SET occurred_at = occurred_at + 28800",
                [],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE import_batches
                    SET parser_version = '1',
                        range_start = range_start + 28800,
                        range_end = range_end + 28800",
                [],
            )
            .unwrap();
    }

    let expected = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:15:00+08:00")
        .unwrap()
        .timestamp();
    {
        let store = LedgerStore::open(&database_path).unwrap();
        let refund = store
            .events(true, true, 10)
            .unwrap()
            .into_iter()
            .find(|event| event.provider_transaction_id == "refund-1")
            .unwrap();
        assert_eq!(refund.occurred_at, expected);
    }
    {
        let connection = rusqlite::Connection::open(&database_path).unwrap();
        let parser_version: String = connection
            .query_row(
                "SELECT parser_version FROM import_batches LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parser_version, "2");
    }

    let store = LedgerStore::open(&database_path).unwrap();
    let refund = store
        .events(true, true, 10)
        .unwrap()
        .into_iter()
        .find(|event| event.provider_transaction_id == "refund-1")
        .unwrap();
    assert_eq!(refund.occurred_at, expected);
}
