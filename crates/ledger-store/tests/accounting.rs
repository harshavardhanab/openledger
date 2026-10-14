use ledger_core::*;
use ledger_store::*;
use serde_json::{Value, json};

fn books() -> LedgerStore {
    let mut s = LedgerStore::memory().unwrap();
    s.create_company(CompanyInput {
        name: "Example".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-01-01".into(),
    })
    .unwrap();
    s
}
fn entry(key: &str, date: &str, debit: i64, credit: i64, amount: &str) -> JournalInput {
    JournalInput {
        date: date.into(),
        reference: "".into(),
        description: "Example transaction".into(),
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
#[test]
fn posting_reports_match_independently_calculated_books() {
    let mut s = books();
    for (key, date, d, c, amount) in [
        ("capital", "2026-10-01", 2, 6, "10000"),
        ("equipment", "2026-10-03", 4, 2, "1500"),
        ("sale", "2026-10-05", 2, 7, "2500"),
        ("rent", "2026-10-08", 8, 2, "400"),
    ] {
        s.post(entry(key, date, d, c, amount)).unwrap();
    }
    let bank = s.ledger(2, None, None).unwrap();
    assert_eq!(bank.closing_minor, "1060000");
    assert_eq!(bank.debit_minor, "1250000");
    assert_eq!(bank.credit_minor, "190000");
    let partial = s.ledger(2, Some("2026-10-05"), Some("2026-10-08")).unwrap();
    assert_eq!(partial.opening_minor, "850000");
    assert_eq!(partial.closing_minor, "1060000");
    assert_eq!(partial.lines.len(), 2);
    let trial = s.trial_balance(None).unwrap();
    assert!(trial.balanced);
    assert_eq!(trial.debit_minor, "1250000");
    assert_eq!(trial.credit_minor, "1250000");
    let old = s.trial_balance(Some("2026-10-02")).unwrap();
    assert_eq!(old.debit_minor, "1000000");
    assert!(old.balanced);
}
#[test]
fn failed_posting_is_atomic_and_idempotency_survives_restore() {
    let mut s = books();
    assert!(
        s.post(entry("unknown", "2026-10-01", 999, 6, "100"))
            .is_err()
    );
    assert_eq!(s.snapshot().unwrap().journal_count, 0);
    let input = entry("request-1", "2026-10-01", 2, 6, "100.01");
    let id = s.post(input.clone()).unwrap();
    assert_eq!(s.post(input.clone()).unwrap(), id);
    assert_eq!(s.snapshot().unwrap().journal_count, 1);
    assert!(
        s.post(entry("request-1", "2026-10-01", 2, 6, "100.02"))
            .is_err()
    );
    let backup = s.export_backup().unwrap();
    let mut restored = LedgerStore::memory().unwrap();
    restored.restore(&backup).unwrap();
    assert_eq!(restored.post(input).unwrap(), id);
    assert_eq!(restored.snapshot().unwrap().journal_count, 1);
}
#[test]
fn reversal_is_a_new_balanced_entry_and_never_edits_the_original() {
    let mut s = books();
    let id = s
        .post(entry("first", "2026-10-01", 2, 6, "901.09"))
        .unwrap();
    let before = serde_json::to_value(s.journal(id).unwrap()).unwrap();
    assert!(s.reverse(id, "2026-09-01", "Mistake", "bad-date").is_err());
    let reverse = s
        .reverse(id, "2026-10-02", "Incorrect amount", "reverse-1")
        .unwrap();
    assert_eq!(
        s.reverse(id, "2026-10-02", "Incorrect amount", "reverse-1")
            .unwrap(),
        reverse
    );
    assert!(
        s.reverse(id, "2026-10-02", "Changed reason", "reverse-1")
            .is_err()
    );
    assert!(
        s.reverse(id, "2026-10-03", "Incorrect amount", "reverse-1")
            .is_err()
    );
    assert!(s.reverse(id, "2026-10-02", "Again", "reverse-2").is_err());
    assert!(
        s.reverse(reverse, "2026-10-03", "Again", "reverse-3")
            .is_err()
    );
    let after = serde_json::to_value(s.journal(id).unwrap()).unwrap();
    assert_eq!(before["lines"], after["lines"]);
    assert_eq!(after["reversed_by"], reverse);
    assert_eq!(s.ledger(2, None, None).unwrap().closing_minor, "0");
    assert_eq!(s.snapshot().unwrap().journal_count, 2);
    let b = s.export_backup().unwrap();
    let mut t = LedgerStore::memory().unwrap();
    t.restore(&b).unwrap();
    assert_eq!(t.ledger(2, None, None).unwrap().closing_minor, "0");
}
#[test]
fn malformed_or_forged_backups_leave_current_books_unchanged() {
    let mut s = books();
    let id = s.post(entry("first", "2026-10-01", 2, 6, "75")).unwrap();
    let base = s.export_backup().unwrap();
    let mut unbalanced: Value = serde_json::from_str(&base).unwrap();
    unbalanced["journals"][0]["lines"][0]["amount_minor"] = json!("7499");
    assert!(s.restore(&unbalanced.to_string()).is_err());
    assert!(s.restore("{bad json").is_err());
    assert_eq!(s.journal(id).unwrap().total_minor, "7500");
    s.reverse(id, "2026-10-02", "Correction", "reverse")
        .unwrap();
    let mut forged: Value = serde_json::from_str(&s.export_backup().unwrap()).unwrap();
    forged["journals"][1]["lines"][0]["account_id"] = json!(3);
    assert!(s.restore(&forged.to_string()).is_err());
    assert_eq!(s.snapshot().unwrap().journal_count, 2);
}
#[test]
fn very_large_exact_amounts_and_repeated_postings_remain_balanced() {
    let mut s = books();
    s.post(entry("large", "2026-10-01", 2, 6, "90071992547409.91"))
        .unwrap();
    assert_eq!(
        s.ledger(2, None, None).unwrap().closing_minor,
        "9007199254740991"
    );
    for i in 0..150 {
        s.post(entry(&format!("small-{i}"), "2026-10-02", 8, 2, "0.01"))
            .unwrap();
    }
    assert_eq!(
        s.ledger(2, None, None).unwrap().closing_minor,
        "9007199254740841"
    );
    assert!(s.trial_balance(None).unwrap().balanced);
}
#[test]
fn on_disk_roundtrip_and_recovery_backup_are_real() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("books.sqlite");
    let service = LedgerService::open(&path).unwrap();
    service
        .request(
            "create_company",
            json!({"name":"First company","currency":"INR","fiscal_year_start":"2026-01-01"}),
        )
        .unwrap();
    service
        .request(
            "post_journal",
            serde_json::to_value(entry("first", "2026-10-01", 2, 6, "42.25")).unwrap(),
        )
        .unwrap();
    let old = service.request("backup", json!({})).unwrap();
    let mut other = books();
    other
        .post(entry("other", "2026-10-01", 2, 6, "89"))
        .unwrap();
    service
        .request(
            "restore_backup",
            json!({"contents":other.export_backup().unwrap()}),
        )
        .unwrap();
    let recovery = std::fs::read_dir(dir.path().join("recovery-backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let recovered: Value =
        serde_json::from_str(&std::fs::read_to_string(recovery).unwrap()).unwrap();
    assert_eq!(recovered["company"]["name"], "First company");
    service
        .request("restore_backup", json!({"contents":old.as_str().unwrap()}))
        .unwrap();
    drop(service);
    let reopened = LedgerStore::open(&path).unwrap();
    assert_eq!(
        reopened.ledger(2, None, None).unwrap().closing_minor,
        "4225"
    );
}

#[test]
fn activity_and_older_journals_remain_available_beyond_the_list_limit() {
    let mut store = books();
    let first = store
        .post(entry("earliest", "2026-10-01", 2, 6, "1.01"))
        .unwrap();
    for i in 0..505 {
        store
            .post(entry(&format!("later-{i}"), "2026-10-02", 2, 6, "1"))
            .unwrap();
    }
    let state = store.snapshot().unwrap();
    assert_eq!(state.journal_count, 506);
    assert_eq!(state.journals.len(), 500);
    assert_eq!(state.journal_activity["2026-10-01"], 1);
    assert_eq!(state.journal_activity["2026-10-02"], 505);
    assert_eq!(store.journal(first).unwrap().total_minor, "101");
    assert_eq!(store.ledger(2, None, None).unwrap().closing_minor, "50601");
}
