use ledger_store::*;
use serde_json::{Value, json};
fn books() -> (LedgerStore, GstProfile, Party, Party) {
    let mut s = LedgerStore::memory().unwrap();
    s.create_company(CompanyInput {
        name: "GST worked examples".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-04-01".into(),
    })
    .unwrap();
    let a = s.gst_control_accounts().unwrap();
    let p = GstProfile {
        gstin: "29ABCDE1234F1Z5".into(),
        state_code: "29".into(),
        state_name: "Karnataka".into(),
        address: "Synthetic registered office, Bengaluru".into(),
        output: serde_json::from_value(a["output"].clone()).unwrap(),
        input: serde_json::from_value(a["input"].clone()).unwrap(),
    };
    s.configure_gst(p.clone()).unwrap();
    let customer = s
        .create_party(PartyInput {
            name: "Synthetic buyer".into(),
            kind: "customer".into(),
            contact: "".into(),
            account_id: 3,
        })
        .unwrap();
    let supplier = s
        .create_party(PartyInput {
            name: "Synthetic supplier".into(),
            kind: "supplier".into(),
            contact: "".into(),
            account_id: 5,
        })
        .unwrap();
    (s, p, customer, supplier)
}
fn invoice(kind: &str, party: i64, key: &str, state: &str) -> DocumentInput {
    DocumentInput {
        gst_adjustment_reviewed: None,
        return_lines: None,
        reviewed_postings: None,
        gst: Some(GstInput {
            supply_state: state.into(),
            supply_state_name: if state == "29" {
                "Karnataka"
            } else {
                "Maharashtra"
            }
            .into(),
            counterparty_gstin: if kind == "purchase" {
                Some("29ABCDE1234F1Z5".into())
            } else {
                None
            },
            counterparty_address: "Synthetic buyer or supplier address".into(),
            delivery_address: "Synthetic delivery address".into(),
            supplier_reference: if kind == "purchase" {
                Some("SUP-001".into())
            } else {
                None
            },
            credit_reviewed: kind == "purchase",
            rates_bps: vec![1800],
            hsn_sac: vec!["998314".into()],
        }),
        kind: kind.into(),
        date: "2026-10-01".into(),
        due_date: Some("2026-10-14".into()),
        party_id: Some(party),
        invoice_id: None,
        money_account_id: None,
        amount: None,
        note: "Synthetic 18% rate example, not a product-rate recommendation".into(),
        lines: vec![DocumentLine {
            description: "Synthetic service".into(),
            quantity: "1".into(),
            unit_price: "1000".into(),
            account_id: if kind == "sale" { 7 } else { 8 },
            item_id: None,
        }],
        request_id: key.into(),
    }
}
#[test]
fn intrastate_sales_tax_is_a_liability_and_settlements_use_the_gross_amount() {
    let (mut s, p, c, _) = books();
    let input = invoice("sale", c.id, "gst-sale", "29");
    let d = s.post_document(input.clone()).unwrap();
    assert_eq!(d.total_minor, "118000");
    let g = d.gst.as_ref().unwrap();
    assert_eq!(g.cgst_minor, "9000");
    assert_eq!(g.sgst_minor, "9000");
    assert_eq!(g.igst_minor, "0");
    assert_eq!(
        s.ledger(p.output.cgst, None, None).unwrap().closing_minor,
        "-9000"
    );
    assert_eq!(
        s.financial_statements(None, None).unwrap().income_minor,
        "100000"
    );
    assert!(s.financial_statements(None, None).unwrap().balanced);
    let payment = DocumentInput {
        gst_adjustment_reviewed: None,
        return_lines: None,
        reviewed_postings: None,
        gst: None,
        kind: "receipt".into(),
        date: "2026-10-02".into(),
        due_date: None,
        party_id: Some(c.id),
        invoice_id: Some(d.id),
        money_account_id: Some(2),
        amount: Some("400".into()),
        note: "Partial receipt".into(),
        lines: vec![],
        request_id: "gst-receipt".into(),
    };
    let mut wrong_bank = payment.clone();
    wrong_bank.money_account_id = Some(p.input.cgst);
    wrong_bank.request_id = "bad-gst-bank".into();
    assert!(s.post_document(wrong_bank).is_err());
    s.post_document(payment).unwrap();
    assert_eq!(
        s.business_state(0, 50, None)
            .unwrap()
            .documents
            .iter()
            .find(|r| r.document.id == d.id)
            .unwrap()
            .outstanding_minor,
        "78000"
    );
    let mut t = LedgerStore::memory().unwrap();
    t.restore(&s.export_backup().unwrap()).unwrap();
    assert_eq!(t.post_document(input).unwrap().id, d.id);
    assert_eq!(t.gst_profile().unwrap(), Some(p));
}
#[test]
fn interstate_sales_and_reviewed_purchase_credits_have_independent_postings() {
    let (mut s, p, c, v) = books();
    let sale = s
        .post_document(invoice("sale", c.id, "interstate", "27"))
        .unwrap();
    assert_eq!(sale.gst.unwrap().igst_minor, "18000");
    assert_eq!(
        s.ledger(p.output.igst, None, None).unwrap().closing_minor,
        "-18000"
    );
    let purchase = s
        .post_document(invoice("purchase", v.id, "local-input", "29"))
        .unwrap();
    assert_eq!(purchase.total_minor, "118000");
    assert_eq!(
        s.ledger(p.input.cgst, None, None).unwrap().closing_minor,
        "9000"
    );
    assert_eq!(s.ledger(5, None, None).unwrap().closing_minor, "-118000");
    assert_eq!(
        s.financial_statements(None, None).unwrap().profit_minor,
        "0"
    );
    let report = s.gst_report("2026-10-01", "2026-10-14").unwrap();
    assert_eq!(report["totals"]["sale"]["igst_minor"], "18000");
    assert_eq!(report["totals"]["purchase"]["cgst_minor"], "9000");
    assert_eq!(report["filing_ready"], false);
    let mut t = LedgerStore::memory().unwrap();
    t.restore(&s.export_backup().unwrap()).unwrap();
    assert_eq!(
        t.gst_report("2026-10-01", "2026-10-14").unwrap()["totals"],
        report["totals"]
    );
}
#[test]
fn unsupported_or_unreviewed_gst_requests_fail_atomically() {
    let (mut s, p, c, v) = books();
    let base = invoice("purchase", v.id, "input", "29");
    for mutation in 0..6 {
        let mut d = base.clone();
        let g = d.gst.as_mut().unwrap();
        match mutation {
            0 => g.credit_reviewed = false,
            1 => g.supply_state = "27".into(),
            2 => g.counterparty_gstin = Some("☃☃☃☃☃".into()),
            3 => g.hsn_sac[0] = "bogus".into(),
            4 => g.rates_bps[0] = 0,
            _ => g.supplier_reference = None,
        };
        assert!(s.post_document(d).is_err());
    }
    assert_eq!(s.snapshot().unwrap().journal_count, 0);
    assert!(
        s.create_party(PartyInput {
            name: "Bad control mapping".into(),
            kind: "customer".into(),
            contact: "".into(),
            account_id: p.input.cgst
        })
        .is_err()
    );
    let mut sale = invoice("sale", c.id, "sale", "29");
    sale.lines[0].account_id = p.input.cgst;
    assert!(s.post_document(sale).is_err());
    assert!(s.configure_gst(p).is_err());
}
#[test]
fn gst_register_uses_reversal_dates_and_tampered_tax_backups_are_rejected() {
    let (mut s, _, c, _) = books();
    let d = s
        .post_document(invoice("sale", c.id, "sale", "29"))
        .unwrap();
    s.reverse(
        d.journal_id,
        "2026-10-05",
        "Accounting correction, review statutory note separately",
        "gst-reverse",
    )
    .unwrap();
    assert_eq!(
        s.gst_report("2026-10-01", "2026-10-04").unwrap()["totals"]["sale"]["cgst_minor"],
        "9000"
    );
    assert_eq!(
        s.gst_report("2026-10-05", "2026-10-05").unwrap()["totals"]["sale"]["cgst_minor"],
        "-9000"
    );
    assert_eq!(
        s.gst_report("2026-10-01", "2026-10-14").unwrap()["totals"]["sale"]["cgst_minor"],
        "0"
    );
    let mut bad: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    bad["business"]["documents"][0]["gst"]["cgst_minor"] = json!("8999");
    assert!(s.restore(&bad.to_string()).is_err());
    assert_eq!(s.snapshot().unwrap().journal_count, 2);
    let mut t = LedgerStore::memory().unwrap();
    t.restore(&s.export_backup().unwrap()).unwrap();
}
#[test]
fn schema_and_backup_v2_compatibility_survive_the_gst_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v2.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../src/schema.sql")).unwrap();
    c.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    drop(c);
    let mut s = LedgerStore::open(&path).unwrap();
    s.create_company(CompanyInput {
        name: "Old books".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-04-01".into(),
    })
    .unwrap();
    let p = s
        .create_party(PartyInput {
            name: "Old buyer".into(),
            kind: "customer".into(),
            contact: "".into(),
            account_id: 3,
        })
        .unwrap();
    let mut d = invoice("sale", p.id, "old", "29");
    d.gst = None;
    s.post_document(d.clone()).unwrap();
    let mut backup: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    backup["version"] = json!(2);
    backup["business"]["numbering"]
        .as_array_mut()
        .unwrap()
        .retain(|n| {
            ["sale", "purchase", "receipt", "payment", "stock", "close"]
                .contains(&n["kind"].as_str().unwrap())
        });
    let mut t = LedgerStore::memory().unwrap();
    t.restore(&backup.to_string()).unwrap();
    assert_eq!(t.post_document(d).unwrap().id, 1);
    assert!(t.gst_profile().unwrap().is_none());
}
#[test]
fn company_inventory_does_not_migrate_inactive_books() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir
        .path()
        .join("books-00000000-0000-4000-8000-000000000001.sqlite");
    let conn = rusqlite::Connection::open(&old).unwrap();
    conn.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    drop(conn);
    let service = LedgerService::open(&dir.path().join("active.sqlite")).unwrap();
    service.request("books", json!({})).unwrap();
    let conn = rusqlite::Connection::open(&old).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn linked_gst_returns_sum_to_original_components_and_register_uses_note_dates() {
    let (mut s, _, c, _) = books();
    let mut input = invoice("sale", c.id, "tiny-sale", "29");
    input.lines[0].quantity = "3".into();
    input.lines[0].unit_price = "0.03".into();
    let sale = s.post_document(input).unwrap();
    assert_eq!(sale.total_minor, "11"); // 9 taxable + 1 CGST + 1 SGST.
    let unreviewed = serde_json::json!({"kind":"credit_note","date":"2026-10-03","due_date":null,"party_id":c.id,"invoice_id":sale.id,"money_account_id":null,"amount":null,"note":"Requires review","lines":[],"return_lines":[{"line":0,"quantity":"1"}],"request_id":"unreviewed"});
    assert!(
        s.post_document(serde_json::from_value(unreviewed).unwrap())
            .is_err()
    );
    let mut totals = [0i64; 4];
    for i in 0..3 {
        let d = s
            .post_document(DocumentInput {
                gst_adjustment_reviewed: Some(true),
                return_lines: Some(vec![ReturnLine {
                    line: 0,
                    quantity: "1".into(),
                }]),
                reviewed_postings: None,
                gst: None,
                kind: "credit_note".into(),
                date: format!("2026-10-0{}", i + 3),
                due_date: None,
                party_id: Some(c.id),
                invoice_id: Some(sale.id),
                money_account_id: None,
                amount: None,
                note: "Cancellation of one unit".into(),
                lines: vec![],
                request_id: format!("note-{i}"),
            })
            .unwrap();
        let g = d.gst.unwrap();
        for (j, n) in [
            &g.taxable_minor,
            &g.cgst_minor,
            &g.sgst_minor,
            &g.igst_minor,
        ]
        .into_iter()
        .enumerate()
        {
            totals[j] += n.parse::<i64>().unwrap();
        }
    }
    assert_eq!(totals, [9, 1, 1, 0]);
    assert_eq!(
        s.gst_report("2026-10-01", "2026-10-05").unwrap()["totals"]["sale"]["taxable_minor"],
        "0"
    );
    let report = s.gst_report("2026-10-03", "2026-10-05").unwrap();
    assert_eq!(report["totals"]["sale"]["cgst_minor"], "-1");
    assert_eq!(report["rows"][0]["event"], "return_note");
    assert_eq!(report["rows"][0]["original_invoice"], sale.number);
    assert_eq!(report["filing_ready"], false);
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&s.export_backup().unwrap()).unwrap();
    assert_eq!(restored.ledger(3, None, None).unwrap().closing_minor, "0");
}
