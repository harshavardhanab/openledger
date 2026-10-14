use crate::{AccountInput, CompanyInput, LedgerStore, StoreError};
use ledger_core::{JournalInput, PostingInput, Side};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

pub struct LedgerService {
    store: Mutex<LedgerStore>,
    recovery_dir: PathBuf,
}
impl LedgerService {
    pub fn open(path: &Path) -> crate::Result<Self> {
        Ok(Self {
            store: Mutex::new(LedgerStore::open(path)?),
            recovery_dir: path
                .parent()
                .unwrap_or(Path::new("."))
                .join("recovery-backups"),
        })
    }
    pub fn request(&self, command: &str, payload: Value) -> std::result::Result<Value, String> {
        self.execute(command, payload).map_err(|e| e.to_string())
    }
    fn execute(&self, command: &str, payload: Value) -> crate::Result<Value> {
        let mut store = self.store.lock().map_err(|_| {
            StoreError::Conflict(
                "The books are temporarily unavailable. Restart the application.".into(),
            )
        })?;
        Ok(match command {
            "state" => serde_json::to_value(store.snapshot()?)?,
            "journal" => {
                let input: JournalId = serde_json::from_value(payload)?;
                serde_json::to_value(store.journal(input.id)?)?
            }
            "create_company" => serde_json::to_value(
                store.create_company(serde_json::from_value::<CompanyInput>(payload)?)?,
            )?,
            "create_account" => serde_json::to_value(
                store.create_account(serde_json::from_value::<AccountInput>(payload)?)?,
            )?,
            "post_journal" => {
                json!({"id":store.post(serde_json::from_value::<JournalInput>(payload)?)?})
            }
            "reverse_journal" => {
                let input: ReverseInput = serde_json::from_value(payload)?;
                json!({"id":store.reverse(input.id,&input.date,&input.reason,&input.request_id)?})
            }
            "trial_balance" => {
                let input: ReportInput = serde_json::from_value(payload)?;
                serde_json::to_value(store.trial_balance(input.as_of.as_deref())?)?
            }
            "ledger" => {
                let input: LedgerInput = serde_json::from_value(payload)?;
                serde_json::to_value(store.ledger(
                    input.account_id,
                    input.from.as_deref(),
                    input.to.as_deref(),
                )?)?
            }
            "backup" => json!(store.export_backup()?),
            "preview_backup" => {
                let input: RestoreInput = serde_json::from_value(payload)?;
                let (_, preview) = LedgerStore::prepare_restore(&input.contents)?;
                serde_json::to_value(preview)?
            }
            "restore_backup" => {
                let input: RestoreInput = serde_json::from_value(payload)?;
                let (stage, preview) = LedgerStore::prepare_restore(&input.contents)?;
                let recovery = if store.company()?.is_some() {
                    std::fs::create_dir_all(&self.recovery_dir)?;
                    let filename = format!(
                        "before-restore-{}-{}.json",
                        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
                        Uuid::new_v4()
                    );
                    let path = self.recovery_dir.join(filename);
                    write_atomic(&path, &store.export_backup()?)?;
                    Some(path)
                } else {
                    None
                };
                store.replace_with(&stage)?;
                json!({"restored":preview,"recovery_saved":recovery.is_some()})
            }
            "create_demo" => {
                if store.company()?.is_some() {
                    return Err(StoreError::Conflict(
                        "Example books can only be created in an empty data file.".into(),
                    ));
                }
                let mut stage = LedgerStore::memory()?;
                stage.create_company(CompanyInput {
                    name: "Atlas Studio (example)".into(),
                    currency: "INR".into(),
                    fiscal_year_start: "2026-01-01".into(),
                })?;
                for (date, description, debit, credit, amount) in [
                    ("2026-10-01", "Opening capital", 2, 6, "25000.00"),
                    ("2026-10-03", "Office equipment", 4, 2, "4500.00"),
                    ("2026-10-05", "Consulting payment received", 2, 7, "8200.00"),
                    ("2026-10-08", "Workspace rent", 8, 2, "850.00"),
                    ("2026-10-12", "Customer invoice", 3, 7, "1800.00"),
                ] {
                    stage.post(JournalInput {
                        date: date.into(),
                        reference: "".into(),
                        description: description.into(),
                        request_id: Uuid::new_v4().to_string(),
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
                    })?;
                }
                store.replace_with(&stage)?;
                serde_json::to_value(store.snapshot()?)?
            }
            _ => return Err(StoreError::Conflict("This action is not available.".into())),
        })
    }
}
pub fn write_atomic(path: &Path, contents: &str) -> crate::Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(contents.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| StoreError::Io(e.error))?;
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseInput {
    id: i64,
    date: String,
    reason: String,
    request_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalId {
    id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportInput {
    as_of: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerInput {
    account_id: i64,
    from: Option<String>,
    to: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreInput {
    contents: String,
}
