use ledger_core::*;
use ledger_store::*;
use serde_json::{Value, json};

fn books() -> LedgerStore {
    let mut s = LedgerStore::memory().unwrap();
    s.create_company(CompanyInput {
        name: "Worked examples".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-01-01".into(),
    })
    .unwrap();
    s
}
fn party(s: &mut LedgerStore, kind: &str) -> Party {
    s.create_party(PartyInput {
        name: format!("Test {kind}"),
        kind: kind.into(),
        contact: "example@example.test".into(),
        account_id: if kind == "customer" { 3 } else { 5 },
    })
    .unwrap()
}
fn invoice(kind: &str, pid: i64, key: &str, amount: &str) -> DocumentInput {
    DocumentInput {
        kind: kind.into(),
        date: "2026-10-01".into(),
        due_date: Some("2026-10-14".into()),
        party_id: Some(pid),
        invoice_id: None,
        money_account_id: None,
        amount: None,
        note: "Worked example".into(),
        lines: vec![DocumentLine {
            description: "Consulting".into(),
            quantity: "1".into(),
            unit_price: amount.into(),
            account_id: if kind == "sale" { 7 } else { 8 },
            item_id: None,
        }],
        request_id: key.into(),
    }
}
fn receipt(invoice: &Document, key: &str, amount: &str) -> DocumentInput {
    DocumentInput {
        kind: if invoice.input.kind == "sale" {
            "receipt"
        } else {
            "payment"
        }
        .into(),
        date: "2026-10-02".into(),
        due_date: None,
        party_id: invoice.input.party_id,
        invoice_id: Some(invoice.id),
        money_account_id: Some(2),
        amount: Some(amount.into()),
        note: "Settlement".into(),
        lines: vec![],
        request_id: key.into(),
    }
}
fn entry(key: &str, date: &str, debit: i64, credit: i64, amount: &str) -> JournalInput {
    JournalInput {
        date: date.into(),
        reference: String::new(),
        description: "Worked example".into(),
        request_id: key.into(),
        lines: vec![
            PostingInput {
                account_id: debit,
                side: Side::Debit,
                amount: amount.into(),
            },
            PostingInput {
                account_id: credit,
                side: Side::Credit,
                amount: amount.into(),
            },
        ],
    }
}
fn restore(s: &LedgerStore) -> LedgerStore {
    let backup = s.export_backup().unwrap();
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&backup).unwrap();
    restored
}
#[test]
fn sales_partial_receipts_reversals_and_asof_outstandings_agree() {
    let mut s = books();
    let p = party(&mut s, "customer");
    let input = invoice("sale", p.id, "sale", "1000");
    let d = s.post_document(input.clone()).unwrap();
    assert_eq!(d.total_minor, "100000");
    assert_eq!(d.number, "INV-000001");
    assert_eq!(s.post_document(input.clone()).unwrap().id, d.id);
    let mut different = input.clone();
    different.lines[0].unit_price = "1001".into();
    assert!(s.post_document(different).is_err());
    let r = s.post_document(receipt(&d, "receipt", "400")).unwrap();
    let b = s.business_state(0, 50, None).unwrap();
    let sale = b.documents.iter().find(|x| x.document.id == d.id).unwrap();
    assert_eq!(sale.outstanding_minor, "60000");
    assert_eq!(
        s.business_state(0, 50, Some("2026-10-01"))
            .unwrap()
            .documents[0]
            .outstanding_minor,
        "100000"
    );
    assert_eq!(s.ledger(3, None, None).unwrap().closing_minor, "60000");
    assert_eq!(s.ledger(2, None, None).unwrap().closing_minor, "40000");
    assert!(
        s.reverse(d.journal_id, "2026-10-03", "Mistake", "bad-rev")
            .is_err()
    );
    assert!(s.post_document(receipt(&d, "too-much", "601")).is_err());
    assert_eq!(s.snapshot().unwrap().journal_count, 2);
    s.reverse(
        r.journal_id,
        "2026-10-03",
        "Receipt correction",
        "reverse-receipt",
    )
    .unwrap();
    assert_eq!(
        s.business_state(0, 50, None)
            .unwrap()
            .documents
            .iter()
            .find(|x| x.document.id == d.id)
            .unwrap()
            .outstanding_minor,
        "100000"
    );
    s.reverse(
        d.journal_id,
        "2026-10-04",
        "Invoice correction",
        "reverse-invoice",
    )
    .unwrap();
    assert_eq!(
        s.business_state(0, 50, None)
            .unwrap()
            .documents
            .iter()
            .find(|x| x.document.id == d.id)
            .unwrap()
            .outstanding_minor,
        "0"
    );
    let t = restore(&s);
    assert_eq!(t.ledger(3, None, None).unwrap().closing_minor, "0");
    assert_eq!(t.business_backup().unwrap().documents.len(), 2);
}
#[test]
fn purchases_and_payments_use_payables_without_inverting_signs() {
    let mut s = books();
    let p = party(&mut s, "supplier");
    let d = s
        .post_document(invoice("purchase", p.id, "bill", "700"))
        .unwrap();
    let payment = s.post_document(receipt(&d, "pay", "250")).unwrap();
    assert_eq!(s.ledger(5, None, None).unwrap().closing_minor, "-45000");
    assert_eq!(s.ledger(8, None, None).unwrap().closing_minor, "70000");
    assert_eq!(s.ledger(2, None, None).unwrap().closing_minor, "-25000");
    let t = restore(&s);
    assert_eq!(
        t.business_state(0, 50, None)
            .unwrap()
            .documents
            .iter()
            .find(|x| x.document.id == d.id)
            .unwrap()
            .outstanding_minor,
        "45000"
    );
    assert_eq!(payment.input.kind, "payment");
}
fn inventory(s: &mut LedgerStore) -> Item {
    let asset = s
        .create_account(AccountInput {
            code: "1300".into(),
            name: "Inventory".into(),
            kind: AccountKind::Asset,
        })
        .unwrap();
    let expense = s
        .create_account(AccountInput {
            code: "5100".into(),
            name: "Cost of sales".into(),
            kind: AccountKind::Expense,
        })
        .unwrap();
    s.create_item(ItemInput {
        code: "WIDGET".into(),
        name: "Widget".into(),
        unit: "each".into(),
        asset_account_id: asset.id,
        expense_account_id: expense.id,
    })
    .unwrap()
}
#[test]
fn perpetual_inventory_and_profit_match_independent_weighted_average_example() {
    let mut s = books();
    let supplier = party(&mut s, "supplier");
    let customer = party(&mut s, "customer");
    let item = inventory(&mut s);
    let mut buy = invoice("purchase", supplier.id, "stock-buy", "10");
    buy.lines[0].quantity = "10".into();
    buy.lines[0].item_id = Some(item.id);
    buy.lines[0].account_id = item.input.asset_account_id;
    s.post_document(buy.clone()).unwrap();
    buy.request_id = "stock-buy-2".into();
    buy.lines[0].unit_price = "20".into();
    buy.date = "2026-10-02".into();
    s.post_document(buy).unwrap();
    // 20 units at weighted cost 15, sell 4 at 25 => revenue 100, cost 60, profit 40; remaining value 240.
    let mut sale = invoice("sale", customer.id, "stock-sale", "25");
    sale.date = "2026-10-03".into();
    sale.lines[0].quantity = "4".into();
    sale.lines[0].item_id = Some(item.id);
    let sold = s.post_document(sale.clone()).unwrap();
    let b = s.business_state(0, 50, None).unwrap();
    assert_eq!(b.stock[0].quantity_milli, "16000");
    assert_eq!(b.stock[0].value_minor, "24000");
    let f = s.financial_statements(None, None).unwrap();
    assert_eq!(f.income_minor, "10000");
    assert_eq!(f.expense_minor, "6000");
    assert_eq!(f.profit_minor, "4000");
    assert!(f.balanced);
    assert_eq!(
        s.ledger(item.input.asset_account_id, None, None)
            .unwrap()
            .closing_minor,
        "24000"
    );
    assert!(
        s.reverse(sold.journal_id, "2026-10-04", "Stock correction", "bad")
            .is_err()
    );
    assert!(
        s.post(entry(
            "manual-stock",
            "2026-10-04",
            item.input.asset_account_id,
            6,
            "1"
        ))
        .is_err()
    );
    sale.request_id = "too-many".into();
    sale.lines[0].quantity = "17".into();
    assert!(s.post_document(sale).is_err());
    assert_eq!(s.snapshot().unwrap().journal_count, 3);
    let mut restored = restore(&s);
    assert_eq!(
        restored.business_state(0, 50, None).unwrap().stock[0].value_minor,
        "24000"
    );
    // Exhaust the stock: no fractional-cent value remains.
    let mut last = invoice("sale", customer.id, "last", "25");
    last.date = "2026-10-04".into();
    last.lines[0].quantity = "16".into();
    last.lines[0].item_id = Some(item.id);
    restored.post_document(last).unwrap();
    assert_eq!(
        restored.business_state(0, 50, None).unwrap().stock[0].value_minor,
        "0"
    );
}
#[test]
fn fractional_quantities_round_extensions_half_up_and_reject_backdated_stock() {
    let mut s = books();
    let supplier = party(&mut s, "supplier");
    let item = inventory(&mut s);
    let mut buy = invoice("purchase", supplier.id, "fraction", "10.01");
    buy.lines[0].quantity = "0.500".into();
    buy.lines[0].item_id = Some(item.id);
    buy.lines[0].account_id = item.input.asset_account_id;
    assert_eq!(s.post_document(buy.clone()).unwrap().total_minor, "501");
    buy.request_id = "backdated".into();
    buy.date = "2026-09-30".into();
    assert!(s.post_document(buy).is_err());
    for invalid in ["0", "-1", "1.0001", "1e3", "9223372036854775807"] {
        assert!(parse_quantity(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        restore(&s).business_state(0, 50, None).unwrap().stock[0].quantity_milli,
        "500"
    );
}
#[test]
fn closing_preserves_profit_reports_carries_balances_and_locks_dates() {
    let mut s = books();
    s.post(entry("capital", "2026-01-01", 2, 6, "1000"))
        .unwrap();
    s.post(entry("income", "2026-10-01", 2, 7, "300")).unwrap();
    s.post(entry("expense", "2026-10-02", 8, 2, "100")).unwrap();
    let p = s.close_period("2026-01-01", "2026-10-03", 6).unwrap();
    assert!(p.journal_id.is_some());
    assert_eq!(s.ledger(7, None, None).unwrap().closing_minor, "0");
    assert_eq!(s.ledger(8, None, None).unwrap().closing_minor, "0");
    assert_eq!(s.ledger(6, None, None).unwrap().closing_minor, "-120000");
    assert!(s.post(entry("closed", "2026-10-03", 2, 7, "1")).is_err());
    assert!(
        s.reverse(p.journal_id.unwrap(), "2026-10-04", "Undo", "undo-close")
            .is_err()
    );
    let f = s
        .financial_statements(Some("2026-01-01"), Some("2026-10-03"))
        .unwrap();
    assert_eq!(f.profit_minor, "20000");
    assert_eq!(f.current_earnings_minor, "0");
    assert!(f.balanced);
    s.post(entry("next-income", "2026-10-04", 2, 7, "25"))
        .unwrap();
    assert_eq!(
        s.ledger(2, Some("2026-10-04"), None).unwrap().opening_minor,
        "120000"
    );
    assert!(s.close_period("2026-10-05", "2026-10-06", 6).is_err());
    s.close_period("2026-10-04", "2026-10-06", 6).unwrap();
    let t = restore(&s);
    assert_eq!(
        t.financial_statements(None, None).unwrap().profit_minor,
        "22500"
    );
    assert_eq!(t.business_backup().unwrap().periods.len(), 2);
}
#[test]
fn reconciliations_accept_earlier_outstanding_lines_and_reject_duplicates_or_mismatches() {
    let mut s = books();
    s.post(entry("deposit", "2026-09-30", 2, 6, "1000"))
        .unwrap();
    s.post(entry("withdrawal", "2026-10-02", 8, 2, "100"))
        .unwrap();
    let lines = s.bank_lines(2, None).unwrap();
    let input = ReconcileInput {
        account_id: 2,
        start: "2026-10-01".into(),
        end: "2026-10-03".into(),
        opening: "0.00".into(),
        closing: "1000".into(),
        reference: "October statement".into(),
        line_ids: vec![lines[0].id],
        request_id: "reconcile-1".into(),
    };
    let mut bad = input.clone();
    bad.closing = "999".into();
    assert!(s.reconcile(bad).is_err());
    assert!(s.business_backup().unwrap().reconciliations.is_empty());
    let r = s.reconcile(input.clone()).unwrap();
    assert_eq!(s.reconcile(input.clone()).unwrap().id, r.id);
    let mut next = input;
    next.start = "2026-10-04".into();
    next.end = "2026-10-05".into();
    next.opening = "1000".into();
    next.closing = "900".into();
    next.line_ids = vec![lines[1].id];
    next.request_id = "next".into();
    s.reconcile(next).unwrap();
    let t = restore(&s);
    assert!(
        t.bank_lines(2, None)
            .unwrap()
            .iter()
            .all(|l| l.reconciliation_id.is_some())
    );
}
#[test]
fn tampered_business_backups_never_replace_books() {
    let mut s = books();
    let p = party(&mut s, "customer");
    s.post_document(invoice("sale", p.id, "sale", "1000"))
        .unwrap();
    let before = s.snapshot().unwrap().journal_count;
    let base: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    for field in ["total_minor", "number"] {
        let mut bad = base.clone();
        bad["business"]["documents"][0][field] = json!("forged");
        assert!(s.restore(&bad.to_string()).is_err());
    }
    let mut bad = base.clone();
    bad["business"]["parties"][0]["input"]["kind"] = json!("supplier");
    assert!(s.restore(&bad.to_string()).is_err());
    assert_eq!(s.snapshot().unwrap().journal_count, before);
}
#[test]
fn numbering_collisions_rollback_sequence_and_posting() {
    let mut s = books();
    let p = party(&mut s, "customer");
    s.post_document(invoice("sale", p.id, "sale-1", "1"))
        .unwrap();
    s.set_numbering(Numbering {
        kind: "sale".into(),
        prefix: "INV-".into(),
        next: 1,
    })
    .unwrap();
    assert!(
        s.post_document(invoice("sale", p.id, "sale-2", "2"))
            .is_err()
    );
    assert_eq!(s.snapshot().unwrap().journal_count, 1);
    assert_eq!(
        s.business_backup()
            .unwrap()
            .numbering
            .iter()
            .find(|n| n.kind == "sale")
            .unwrap()
            .next,
        1
    );
}
#[test]
fn v1_database_and_backup_migrate_without_losing_original_idempotency() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    drop(conn);
    let mut s = LedgerStore::open(&path).unwrap();
    s.create_company(CompanyInput {
        name: "Legacy".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-01-01".into(),
    })
    .unwrap();
    let input = entry("old-id", "2026-01-01", 2, 6, "90071992547409.91");
    let id = s.post(input.clone()).unwrap();
    let mut backup: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    backup["version"] = json!(1);
    backup.as_object_mut().unwrap().remove("business");
    for j in backup["journals"].as_array_mut().unwrap() {
        for l in j["lines"].as_array_mut().unwrap() {
            l.as_object_mut().unwrap().remove("id");
        }
    }
    let mut restored = books();
    restored.restore(&backup.to_string()).unwrap();
    assert_eq!(restored.post(input).unwrap(), id);
    assert_eq!(
        restored.ledger(2, None, None).unwrap().closing_minor,
        "9007199254740991"
    );
}
#[test]
fn separate_company_switching_survives_restart_and_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("default.sqlite");
    let service = LedgerService::open(&path).unwrap();
    service
        .request(
            "create_company",
            json!({"name":"Original","currency":"INR","fiscal_year_start":"2026-01-01"}),
        )
        .unwrap();
    service
        .request(
            "new_books",
            json!({"name":"Second","currency":"USD","fiscal_year_start":"2026-01-01"}),
        )
        .unwrap();
    let active = service.request("books", json!({})).unwrap()["active"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        service
            .request("switch_books", json!({"file":"../../private.sqlite"}))
            .is_err()
    );
    drop(service);
    let service = LedgerService::open(&path).unwrap();
    assert_eq!(
        service.request("state", json!({})).unwrap()["company"]["name"],
        "Second"
    );
    service
        .request("switch_books", json!({"file":"default.sqlite"}))
        .unwrap();
    assert_eq!(
        service.request("state", json!({})).unwrap()["company"]["name"],
        "Original"
    );
    service
        .request("switch_books", json!({"file":active}))
        .unwrap();
}
#[test]
fn batch_import_is_previewable_atomic_and_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let service = LedgerService::open(&dir.path().join("import.sqlite")).unwrap();
    service
        .request(
            "create_company",
            json!({"name":"Import","currency":"INR","fiscal_year_start":"2026-01-01"}),
        )
        .unwrap();
    let input = json!({"format":"open-ledger-journals","version":1,"preview":true,"journals":[entry("one","2026-10-01",2,6,"10"),entry("two","2026-10-02",8,2,"2")]});
    assert_eq!(
        service.request("import_journals", input.clone()).unwrap()["count"],
        2
    );
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        0
    );
    let mut bad = input.clone();
    bad["preview"] = json!(false);
    bad["journals"][1]["lines"][0]["account_id"] = json!(999);
    assert!(service.request("import_journals", bad).is_err());
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        0
    );
    let mut good = input;
    good["preview"] = json!(false);
    service.request("import_journals", good.clone()).unwrap();
    service.request("import_journals", good).unwrap();
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        2
    );
}
#[test]
fn backdated_replacement_receipts_cannot_overpay_historical_balances() {
    let mut s = books();
    let p = party(&mut s, "customer");
    let d = s
        .post_document(invoice("sale", p.id, "sale", "100"))
        .unwrap();
    let r = s.post_document(receipt(&d, "paid", "100")).unwrap();
    s.reverse(r.journal_id, "2026-10-05", "Correction", "undo")
        .unwrap();
    assert!(s.post_document(receipt(&d, "backdated", "100")).is_err());
    assert!(
        s.reverse(d.journal_id, "2026-10-03", "Correction", "backdated-undo")
            .is_err()
    );
    let mut replacement = receipt(&d, "replacement", "100");
    replacement.date = "2026-10-06".into();
    s.post_document(replacement).unwrap();
    assert_eq!(
        s.business_state(0, 50, Some("2026-10-04"))
            .unwrap()
            .documents
            .iter()
            .find(|v| v.document.id == d.id)
            .unwrap()
            .outstanding_minor,
        "0"
    );
    restore(&s);
}
#[test]
fn stock_adjustments_are_balanced_and_forged_valuations_are_rejected() {
    let mut s = books();
    let item = inventory(&mut s);
    let input = DocumentInput {
        kind: "stock".into(),
        date: "2026-10-01".into(),
        due_date: None,
        party_id: None,
        invoice_id: None,
        money_account_id: None,
        amount: None,
        note: "Opening stock".into(),
        request_id: "opening-stock".into(),
        lines: vec![DocumentLine {
            description: "Widget".into(),
            quantity: "5".into(),
            unit_price: "10".into(),
            account_id: 6,
            item_id: Some(item.id),
        }],
    };
    s.post_document(input.clone()).unwrap();
    let mut issue = input;
    issue.request_id = "issue".into();
    issue.date = "2026-10-02".into();
    issue.lines[0].quantity = "2".into();
    issue.lines[0].account_id = item.input.asset_account_id;
    s.post_document(issue).unwrap();
    assert_eq!(
        s.ledger(item.input.asset_account_id, None, None)
            .unwrap()
            .closing_minor,
        "3000"
    );
    assert_eq!(
        s.ledger(item.input.expense_account_id, None, None)
            .unwrap()
            .closing_minor,
        "2000"
    );
    assert_eq!(s.stock_history(0, 50).unwrap()["count"], 2);
    let mut forged: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    forged["business"]["stock_moves"][0]["value_minor"] = json!(5001);
    assert!(s.restore(&forged.to_string()).is_err());
    restore(&s);
    let mut missing: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    missing["business"]["documents"] = json!([]);
    missing["business"]["stock_moves"] = json!([]);
    assert!(s.restore(&missing.to_string()).is_err());
}
#[test]
fn large_books_are_paged_without_losing_full_report_totals() {
    let mut s = books();
    let p = party(&mut s, "customer");
    let began = std::time::Instant::now();
    for i in 0..2000 {
        s.post_document(invoice("sale", p.id, &format!("volume-{i}"), "0.01"))
            .unwrap();
    }
    let page = s.business_state(1950, 50, None).unwrap();
    assert_eq!(page.document_count, 2000);
    assert_eq!(page.documents.len(), 50);
    assert_eq!(s.journals_page(1950, 50).unwrap().len(), 50);
    assert_eq!(
        s.financial_statements(None, None).unwrap().income_minor,
        "2000"
    );
    assert_eq!(s.ledger(3, None, None).unwrap().closing_minor, "2000");
    eprintln!(
        "2,000 invoices, paging, ledger and statements: {:?}",
        began.elapsed()
    );
}
