use ledger_core::{JournalInput, PostingInput, Side};
use ledger_store::{CompanyInput, LedgerStore};
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn interrupted_write_child() {
    let Some(path) = std::env::var_os("OPEN_LEDGER_DURABILITY_CHILD") else {
        return;
    };
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE; INSERT INTO journals(date,reference,description,request_id,request_payload,created_at,total_minor) VALUES('2026-10-02','INTERRUPTED','Incomplete','incomplete','{}','2026-10-02T00:00:00Z',100); INSERT INTO lines(journal_id,position,account_id,side,amount_minor) VALUES(last_insert_rowid(),0,2,'debit',100);").unwrap();
    std::fs::write(
        std::path::PathBuf::from(path).with_extension("ready"),
        b"transaction-open",
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(30));
    panic!("Parent should kill this process before commit");
}
#[test]
fn abrupt_process_exit_rolls_back_partial_writes_and_preserves_committed_books() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("durable.sqlite");
    let mut s = LedgerStore::open(&path).unwrap();
    s.create_company(CompanyInput {
        name: "Durability".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-01-01".into(),
    })
    .unwrap();
    s.post(JournalInput {
        date: "2026-10-01".into(),
        reference: String::new(),
        description: "Committed capital".into(),
        request_id: "committed".into(),
        lines: vec![
            PostingInput {
                account_id: 2,
                side: Side::Debit,
                amount: "1000".into(),
            },
            PostingInput {
                account_id: 6,
                side: Side::Credit,
                amount: "1000".into(),
            },
        ],
    })
    .unwrap();
    drop(s);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "interrupted_write_child", "--nocapture"])
        .env("OPEN_LEDGER_DURABILITY_CHILD", &path)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.with_extension("ready").exists() {
        if Instant::now() > deadline {
            child.kill().ok();
            child.wait().ok();
            panic!("Child failed to open its transaction");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let s = LedgerStore::open(&path).unwrap();
    assert_eq!(s.snapshot().unwrap().journal_count, 1);
    assert_eq!(s.ledger(2, None, None).unwrap().closing_minor, "100000");
    assert!(s.trial_balance(None).unwrap().balanced);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM journals", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
#[test]
fn failed_schema_migration_rolls_back_and_a_retry_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("migration.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    conn.execute_batch("CREATE TABLE parties (collision TEXT)")
        .unwrap();
    drop(conn);
    assert!(LedgerStore::open(&path).is_err());
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='items'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    conn.execute_batch("DROP TABLE parties").unwrap();
    drop(conn);
    LedgerStore::open(&path).unwrap();
}
#[test]
fn return_schema_rebuild_rolls_back_atomically_and_preserves_child_references() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v3.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    conn.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    conn.execute_batch(include_str!("../src/migration_v3.sql"))
        .unwrap();
    conn.execute_batch("CREATE TABLE documents_new(collision TEXT)")
        .unwrap();
    drop(conn);
    assert!(LedgerStore::open(&path).is_err());
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM numbering", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    conn.execute_batch("DROP TABLE documents_new").unwrap();
    drop(conn);
    drop(LedgerStore::open(&path).unwrap());
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    let fk: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='stock_moves'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(fk.contains("REFERENCES documents(id)"));
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    let mut s = LedgerStore::open(&path).unwrap();
    s.create_company(CompanyInput {
        name: "Migrated".into(),
        currency: "INR".into(),
        fiscal_year_start: "2026-04-01".into(),
    })
    .unwrap();
}
