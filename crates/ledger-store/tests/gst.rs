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
            classification: None,
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

fn draft(s: &LedgerStore, period: &str, frequency: &str) -> Value {
    serde_json::to_value(
        s.gst_return_draft(GstReturnQuery {
            period: period.into(),
            frequency: frequency.into(),
        })
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn return_drafts_classify_gross_thresholds_and_preserve_exact_csv_and_control_totals() {
    let (mut s, _, c, _) = books();
    let mut registered = invoice("sale", c.id, "b2b", "29");
    registered.gst.as_mut().unwrap().counterparty_gstin = Some("29ABCDE1234F1Z5".into());
    s.post_document(registered).unwrap();
    // Exact gross threshold is B2CS; one paise above it is B2CL.
    let mut at = invoice("sale", c.id, "at-limit", "27");
    at.lines[0].unit_price = "84745.76".into(); // +18% = 100000.00.
    assert_eq!(s.post_document(at).unwrap().total_minor, "10000000");
    let mut over = invoice("sale", c.id, "over-limit", "27");
    over.lines[0].unit_price = "84745.77".into();
    assert_eq!(s.post_document(over).unwrap().total_minor, "10000001");
    let r = draft(&s, "2026-10", "monthly");
    assert_eq!(r["gstr1_rows"][0]["section"], "b2b");
    assert_eq!(r["gstr1_rows"][1]["section"], "b2cs");
    assert_eq!(r["gstr1_rows"][2]["section"], "b2cl");
    assert_eq!(
        r["gstr3b_book_3_2_unregistered"]["27"]["taxable_minor"],
        "16949153"
    );
    assert_eq!(r["reconciled"], true);
    assert!(r["issues"].as_array().unwrap().is_empty());
    assert_eq!(r["filing_ready"], false);
    let csv = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["section"] == "b2cl")
        .unwrap()["contents"]
        .as_str()
        .unwrap();
    assert!(csv.starts_with("\"Invoice Number\",\"Invoice date\",\"Invoice Value\""));
    assert!(csv.contains("\"100000.01\""));
    assert!(csv.contains("\"01-Oct-26\""));
    assert!(!csv.contains("e+"));
    assert_eq!(s.snapshot().unwrap().journal_count, 3); // Preparation is read-only.
}
#[test]
fn return_drafts_route_credits_using_original_invoice_and_keep_small_consumer_notes_net() {
    let (mut s, _, c, _) = books();
    for (i, price) in ["1000", "100000"].iter().enumerate() {
        let mut d = invoice("sale", c.id, &format!("consumer-{i}"), "27");
        d.date = "2026-09-01".into();
        d.lines[0].quantity = "10".into();
        d.lines[0].unit_price = (*price).into();
        let original = s.post_document(d).unwrap();
        let note: DocumentInput = serde_json::from_value(json!({
            "kind":"credit_note","date":"2026-10-01","due_date":null,"party_id":c.id,
            "invoice_id":original.id,"money_account_id":null,"amount":null,"note":"Partial credit",
            "lines":[],"return_lines":[{"line":0,"quantity":"0.1"}],"gst_adjustment_reviewed":true,
            "request_id":format!("note-{i}")
        }))
        .unwrap();
        s.post_document(note).unwrap();
    }
    let r = draft(&s, "2026-10", "monthly");
    assert_eq!(r["gstr1_rows"][0]["section"], "b2cs");
    assert_eq!(r["gstr1_rows"][0]["amounts"]["taxable_minor"], "-10000");
    assert_eq!(r["gstr1_rows"][1]["section"], "cdnur");
    assert_eq!(r["gstr1_rows"][1]["original_invoice"], "INV-000002");
    assert_eq!(r["gstr3b_book_3_1_a"]["igst_minor"], "-181800");
    assert_eq!(r["reconciled"], true);
    let b2cs = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["section"] == "b2cs")
        .unwrap();
    assert!(b2cs["contents"].as_str().unwrap().contains("\"-100.00\""));
    let cdnur = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["section"] == "cdnur")
        .unwrap();
    assert!(
        cdnur["contents"]
            .as_str()
            .unwrap()
            .contains("\"B2CL\",\"CN-000002\",\"01-Oct-26\",\"C\"")
    );
    assert_eq!(r["hsn_review"][0]["quantity_milli"], "-200");
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&s.export_backup().unwrap()).unwrap();
    assert_eq!(draft(&restored, "2026-10", "monthly"), r);
}
#[test]
fn draft_review_identifies_unclassified_documents_reversals_and_offsetting_manual_tax_entries() {
    let (mut s, p, c, _) = books();
    let d = s
        .post_document(invoice("sale", c.id, "reversed", "29"))
        .unwrap();
    s.reverse(
        d.journal_id,
        "2026-10-03",
        "Accounting correction",
        "reverse",
    )
    .unwrap();
    let mut plain = invoice("sale", c.id, "no-tax", "29");
    plain.gst = None;
    s.post_document(plain).unwrap();
    let j = s
        .post(ledger_core::JournalInput {
            date: "2026-10-03".into(),
            reference: "manual-tax".into(),
            description: "Tax adjustment requires classification".into(),
            request_id: "manual-tax".into(),
            lines: vec![
                ledger_core::PostingInput {
                    account_id: p.output.cgst,
                    side: ledger_core::Side::Debit,
                    amount: "1.00".into(),
                },
                ledger_core::PostingInput {
                    account_id: 8,
                    side: ledger_core::Side::Credit,
                    amount: "1.00".into(),
                },
            ],
        })
        .unwrap();
    let mismatch = draft(&s, "2026-10", "monthly");
    assert_eq!(mismatch["reconciled"], false);
    assert_eq!(mismatch["reconciliation"][0]["difference_minor"], "-100");
    s.reverse(j, "2026-10-04", "Undo manual adjustment", "undo-manual")
        .unwrap();
    let r = draft(&s, "2026-10", "monthly");
    assert_eq!(r["reconciled"], true);
    assert!(r["gstr1_rows"].as_array().unwrap().is_empty());
    let codes = r["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"accounting_reversal"));
    assert!(codes.contains(&"unclassified_document"));
    assert_eq!(
        codes
            .iter()
            .filter(|c| c.starts_with("manual_gst_journal_"))
            .count(),
        2
    );
}
#[test]
fn draft_period_validation_quarters_leap_year_and_historical_b2cl_thresholds() {
    let (mut s, _, c, _) = books();
    for (date, key) in [("2024-07-15", "july"), ("2024-08-15", "august")] {
        let mut d = invoice("sale", c.id, key, "27");
        d.date = date.into();
        d.due_date = Some(date.into());
        d.lines[0].unit_price = "100000".into();
        s.post_document(d).unwrap();
    }
    assert_eq!(
        draft(&s, "2024-07", "monthly")["gstr1_rows"][0]["section"],
        "b2cs"
    );
    assert_eq!(
        draft(&s, "2024-08", "monthly")["gstr1_rows"][0]["section"],
        "b2cl"
    );
    let q = draft(&s, "2024-09", "quarterly");
    assert_eq!(q["from"], "2024-07-01");
    assert_eq!(q["to"], "2024-09-30");
    assert_eq!(draft(&s, "2024-02", "monthly")["to"], "2024-02-29");
    for (period, frequency) in [
        ("2026-10", "quarterly"),
        ("2017-06", "monthly"),
        ("2026-2", "monthly"),
        ("2026-13", "monthly"),
        ("💸-01", "monthly"),
        ("2026-10", "daily"),
    ] {
        assert!(
            s.gst_return_draft(GstReturnQuery {
                period: period.into(),
                frequency: frequency.into()
            })
            .is_err()
        );
    }
}

#[test]
fn draft_csv_keeps_large_decimals_mixed_rates_and_registered_credit_signs() {
    let (mut s, _, c, _) = books();
    let mut input = invoice("sale", c.id, "large-exact", "29");
    input.lines[0].unit_price = "90071992547409.93".into();
    input.gst.as_mut().unwrap().counterparty_gstin = Some("29ABCDE1234F1Z5".into());
    input.lines.push(DocumentLine {
        description: "Second rate".into(),
        quantity: "1".into(),
        unit_price: "1".into(),
        account_id: 7,
        item_id: None,
    });
    input.gst.as_mut().unwrap().rates_bps.push(500);
    input.gst.as_mut().unwrap().hsn_sac.push("998314".into());
    let original = s.post_document(input).unwrap();
    let d: DocumentInput = serde_json::from_value(json!({
        "kind":"credit_note","date":"2026-10-02","due_date":null,"party_id":c.id,
        "invoice_id":original.id,"money_account_id":null,"amount":null,"note":"Full return",
        "lines":[],"return_lines":[{"line":0,"quantity":"1"},{"line":1,"quantity":"1"}],"gst_adjustment_reviewed":true,
        "request_id":"large-credit"
    })).unwrap();
    s.post_document(d).unwrap();
    let r = draft(&s, "2026-10", "monthly");
    assert_eq!(
        r["gstr1_sections"]["b2b"]["taxable_minor"],
        "9007199254741093"
    );
    assert_eq!(
        r["gstr1_sections"]["cdnr"]["taxable_minor"],
        "-9007199254741093"
    );
    assert_eq!(r["gstr3b_book_3_1_a"]["taxable_minor"], "0");
    let files = r["files"].as_array().unwrap();
    let b2b = files.iter().find(|f| f["section"] == "b2b").unwrap();
    let cdnr = files.iter().find(|f| f["section"] == "cdnr").unwrap();
    assert_eq!(b2b["row_count"], 2);
    assert_eq!(cdnr["row_count"], 2);
    assert!(
        b2b["contents"]
            .as_str()
            .unwrap()
            .contains("\"90071992547409.93\"")
    );
    assert!(cdnr["contents"].as_str().unwrap().contains("\"C\""));
    assert!(
        cdnr["contents"]
            .as_str()
            .unwrap()
            .contains("\"90071992547409.93\"")
    ); // Positive value; C supplies the note direction.
}
#[test]
fn draft_flags_group_rounding_and_manual_income_instead_of_silently_omitting_them() {
    let (mut s, _, c, _) = books();
    let mut input = invoice("sale", c.id, "rounding", "29");
    input.lines[0].unit_price = "0.03".into();
    input.lines.push(input.lines[0].clone());
    input.gst.as_mut().unwrap().rates_bps.push(1800);
    input.gst.as_mut().unwrap().hsn_sac.push("998314".into());
    s.post_document(input).unwrap(); // Each 3-paise line has zero half-component tax; six paise aggregated has a one-paise half component.
    s.post(ledger_core::JournalInput {
        date: "2026-10-01".into(),
        reference: "manual-sale".into(),
        description: "Unclassified sale".into(),
        request_id: "manual-sale".into(),
        lines: vec![
            ledger_core::PostingInput {
                account_id: 2,
                side: ledger_core::Side::Debit,
                amount: "100".into(),
            },
            ledger_core::PostingInput {
                account_id: 7,
                side: ledger_core::Side::Credit,
                amount: "100".into(),
            },
        ],
    })
    .unwrap();
    let r = draft(&s, "2026-10", "monthly");
    let issues = r["issues"].as_array().unwrap();
    assert!(
        issues
            .iter()
            .any(|i| i["code"] == "offline_rounding_difference")
    );
    assert!(issues.iter().any(|i| i["reference"] == "manual-sale"));
    assert_eq!(r["reconciled"], true); // Matching ledger alone cannot certify a return.
}

#[test]
fn classified_zero_tax_supplies_post_without_tax_and_flow_to_separate_draft_sections() {
    let (mut s, p, c, _) = books();
    let mut docs = vec![];
    for (i, kind) in ["nil_rated", "exempt", "non_gst"].into_iter().enumerate() {
        let mut d = invoice("sale", c.id, kind, if i == 1 { "27" } else { "29" });
        let g = d.gst.as_mut().unwrap();
        g.classification = Some(SupplyClassification {
            supplier_state: None,
            kind: kind.into(),
            reason: "Synthetic independently reviewed classification basis".into(),
        });
        g.rates_bps = vec![0];
        if i == 1 {
            g.counterparty_gstin = Some("27ABCDE1234F1Z5".into());
        }
        let posted = s.post_document(d.clone()).unwrap();
        assert_eq!(posted.total_minor, "100000");
        assert_eq!(posted.gst.as_ref().unwrap().cgst_minor, "0");
        assert_eq!(s.post_document(d).unwrap().id, posted.id);
        docs.push(posted);
    }
    assert_eq!(
        s.ledger(p.output.cgst, None, None).unwrap().closing_minor,
        "0"
    );
    let note: DocumentInput = serde_json::from_value(json!({"kind":"credit_note","date":"2026-10-02","due_date":null,"party_id":c.id,"invoice_id":docs[0].id,"money_account_id":null,"amount":null,"note":"Return half of nil-rated supply","lines":[],"return_lines":[{"line":0,"quantity":"0.5"}],"request_id":"nil-return"})).unwrap();
    s.post_document(note).unwrap();
    let r = draft(&s, "2026-10", "monthly");
    assert_eq!(r["gstr3b_book_3_1_a"]["taxable_minor"], "0");
    assert_eq!(r["gstr3b_book_3_1_c"]["taxable_minor"], "150000");
    assert_eq!(r["gstr3b_book_3_1_e"]["taxable_minor"], "100000");
    assert!(
        r["gstr3b_book_3_2_unregistered"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert_eq!(r["gstr1_sections"]["exemp"]["taxable_minor"], "250000");
    assert_eq!(r["reconciled"], true);
    let file = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["section"] == "exemp")
        .unwrap();
    let csv = file["contents"].as_str().unwrap();
    assert!(csv.contains(
        "\"Intra-State supplies to unregistered persons\",\"500.00\",\"0.00\",\"1000.00\""
    ));
    assert!(
        csv.contains(
            "\"Inter-State supplies to registered persons\",\"0.00\",\"1000.00\",\"0.00\""
        )
    );
    assert!(r["issues"].as_array().unwrap().is_empty());
    let backup = s.export_backup().unwrap();
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&backup).unwrap();
    assert_eq!(draft(&restored, "2026-10", "monthly"), r);
    let mut old: Value = serde_json::from_str(&backup).unwrap();
    old["version"] = json!(4);
    assert!(restored.restore(&old.to_string()).is_err());
    let mut bad: Value = serde_json::from_str(&backup).unwrap();
    bad["business"]["documents"][0]["input"]["gst"]["classification"]["kind"] = json!("regular");
    assert!(restored.restore(&bad.to_string()).is_err());
}
#[test]
fn zero_tax_classifications_reject_tax_rates_missing_review_and_purchase_misuse_atomically() {
    let (mut s, _, c, supplier) = books();
    let mut base = invoice("sale", c.id, "nil", "29");
    base.gst.as_mut().unwrap().classification = Some(SupplyClassification {
        supplier_state: None,
        kind: "nil_rated".into(),
        reason: "Reviewed reference".into(),
    });
    base.gst.as_mut().unwrap().rates_bps = vec![0];
    for mutation in 0..5 {
        let mut input = base.clone();
        match mutation {
            0 => input.gst.as_mut().unwrap().rates_bps[0] = 1800,
            1 => input
                .gst
                .as_mut()
                .unwrap()
                .classification
                .as_mut()
                .unwrap()
                .reason
                .clear(),
            2 => {
                input
                    .gst
                    .as_mut()
                    .unwrap()
                    .classification
                    .as_mut()
                    .unwrap()
                    .kind = "zero_rated_export".into()
            }
            3 => {
                input.kind = "purchase".into();
                input.party_id = Some(supplier.id);
                input.lines[0].account_id = 8;
            }
            _ => input.gst.as_mut().unwrap().classification = None,
        }
        assert!(s.post_document(input).is_err());
    }
    assert_eq!(s.snapshot().unwrap().journal_count, 0);
}
#[test]
fn v4_backup_and_database_migration_preserve_regular_gst_retry_identity() {
    let (mut s, _, c, _) = books();
    let input = invoice("sale", c.id, "legacy-regular", "29");
    let posted = s.post_document(input.clone()).unwrap();
    let mut old: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    old["version"] = json!(4);
    assert!(
        old["business"]["documents"][0]["input"]["gst"]
            .get("classification")
            .is_none()
    );
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&old.to_string()).unwrap();
    assert_eq!(restored.post_document(input).unwrap().id, posted.id);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-v4.sqlite");
    let mut disk = LedgerStore::open(&path).unwrap();
    disk.restore(&old.to_string()).unwrap();
    drop(disk);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.pragma_update(None, "user_version", 4).unwrap();
    drop(conn);
    let migrated = LedgerStore::open(&path).unwrap();
    assert_eq!(migrated.snapshot().unwrap().journal_count, 1);
    assert_eq!(
        migrated.gst_profile().unwrap().unwrap().gstin,
        "29ABCDE1234F1Z5"
    );
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
}

#[test]
fn classified_inward_supplies_use_reviewed_supplier_state_without_claiming_input_tax() {
    let (mut s, _, _, supplier) = books();
    for (i, (kind, state)) in [("nil_rated", "27"), ("exempt", "29"), ("non_gst", "27")]
        .into_iter()
        .enumerate()
    {
        let mut input = invoice("purchase", supplier.id, &format!("inward-{i}"), "29");
        let g = input.gst.as_mut().unwrap();
        g.classification = Some(SupplyClassification {
            kind: kind.into(),
            reason: "Reviewed domestic non-taxable inward supply".into(),
            supplier_state: Some(state.into()),
        });
        g.credit_reviewed = false;
        g.rates_bps[0] = 0;
        g.counterparty_gstin = None;
        let posted = s.post_document(input.clone()).unwrap();
        assert_eq!(posted.total_minor, "100000");
        if i == 0 {
            let note: DocumentInput = serde_json::from_value(json!({"kind":"debit_note","date":"2026-10-02","due_date":null,"party_id":supplier.id,"invoice_id":posted.id,"money_account_id":null,"amount":null,"note":"Partial inward return","lines":[],"return_lines":[{"line":0,"quantity":"0.25"}],"request_id":"nil-purchase-return"})).unwrap();
            s.post_document(note).unwrap();
        }
        let mut wrong = input.clone();
        wrong.request_id = format!("bad-state-{i}");
        wrong.gst.as_mut().unwrap().counterparty_gstin = Some(
            if state == "27" {
                "29ABCDE1234F1Z5"
            } else {
                "27ABCDE1234F1Z5"
            }
            .into(),
        );
        assert!(s.post_document(wrong).is_err());
        let mut wrong = input;
        wrong.request_id = format!("bad-credit-{i}");
        wrong.gst.as_mut().unwrap().credit_reviewed = true;
        assert!(s.post_document(wrong).is_err());
    }
    let r = draft(&s, "2026-10", "monthly");
    assert!(r["gstr1_rows"].as_array().unwrap().is_empty());
    assert_eq!(r["gstr3b_book_5"][0]["inter_state_minor"], "75000");
    assert_eq!(r["gstr3b_book_5"][0]["intra_state_minor"], "100000");
    assert_eq!(r["gstr3b_book_5"][1]["inter_state_minor"], "100000");
    assert_eq!(r["gstr3b_book_4_a_5"]["igst_minor"], "0");
    assert_eq!(r["reconciled"], true);
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&s.export_backup().unwrap()).unwrap();
    assert_eq!(draft(&restored, "2026-10", "monthly"), r);
}
