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
    default_path: PathBuf,
    active_path: PathBuf,
}
impl LedgerService {
    pub fn open(path: &Path) -> crate::Result<Self> {
        let active_path = path.with_extension("active");
        let selected = if active_path.exists() {
            let name = std::fs::read_to_string(&active_path)?;
            if !valid_book_name(&name) {
                return Err(StoreError::Conflict("Invalid active books file.".into()));
            }
            let selected = path.parent().unwrap_or(Path::new(".")).join(name);
            if !selected.exists() {
                return Err(StoreError::Conflict(
                    "The active books file is missing. Recover the file before continuing.".into(),
                ));
            }
            selected
        } else {
            path.to_path_buf()
        };
        Ok(Self {
            store: Mutex::new(LedgerStore::open(&selected)?),
            default_path: path.to_path_buf(),
            active_path,
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
            "books" => {
                let parent = self.default_path.parent().unwrap_or(Path::new("."));
                let mut books = vec![];
                for entry in std::fs::read_dir(parent)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    if valid_book_name(&name) || entry.path() == self.default_path {
                        let company = LedgerStore::inspect_company(&entry.path())?;
                        books.push(json!({"file":name,"company":company}));
                    }
                }
                json!({"books":books,"active":std::fs::read_to_string(&self.active_path).unwrap_or_else(|_|self.default_path.file_name().unwrap().to_string_lossy().into_owned())})
            }
            "new_books" => {
                let name = format!("books-{}.sqlite", Uuid::new_v4());
                let mut next = LedgerStore::open(
                    &self
                        .default_path
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join(&name),
                )?;
                if !payload.is_null() && payload.get("name").is_some() {
                    next.create_company(serde_json::from_value::<CompanyInput>(payload)?)?;
                }
                write_atomic(&self.active_path, &name)?;
                *store = next;
                serde_json::to_value(store.snapshot()?)?
            }
            "switch_books" => {
                let input: BookSelection = serde_json::from_value(payload)?;
                let default = self.default_path.file_name().unwrap().to_string_lossy();
                if !valid_book_name(&input.file) && input.file != default {
                    return Err(StoreError::Conflict("Unknown books file.".into()));
                }
                let path = self
                    .default_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(&input.file);
                if !path.exists() {
                    return Err(StoreError::Conflict("Books file not found.".into()));
                }
                let next = LedgerStore::open(&path)?;
                if input.file == default {
                    if self.active_path.exists() {
                        std::fs::remove_file(&self.active_path)?;
                    }
                } else {
                    write_atomic(&self.active_path, &input.file)?;
                }
                *store = next;
                serde_json::to_value(store.snapshot()?)?
            }
            "configure_gst" => {
                store.configure_gst(serde_json::from_value(payload)?)?;
                json!({"saved":true})
            }
            "gst_control_accounts" => store.gst_control_accounts()?,
            "gst_return_draft" => {
                serde_json::to_value(store.gst_return_draft(serde_json::from_value(payload)?)?)?
            }
            "gst_report" => {
                let input: GstReportQuery = serde_json::from_value(payload)?;
                store.gst_report(&input.from, &input.to)?
            }
            "business" => {
                let input: BusinessQuery = serde_json::from_value(payload)?;
                serde_json::to_value(store.business_state(
                    input.offset.unwrap_or(0),
                    input.limit.unwrap_or(50),
                    input.as_of.as_deref(),
                )?)?
            }
            "create_party" => {
                serde_json::to_value(store.create_party(serde_json::from_value(payload)?)?)?
            }
            "create_item" => {
                serde_json::to_value(store.create_item(serde_json::from_value(payload)?)?)?
            }
            "returnable_invoice" => {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Query {
                    id: i64,
                }
                let q: Query = serde_json::from_value(payload)?;
                store.returnable_invoice(q.id)?
            }
            "preview_document" => store.preview_document(serde_json::from_value(payload)?)?,
            "post_document" => {
                serde_json::to_value(store.post_document(serde_json::from_value(payload)?)?)?
            }
            "set_numbering" => {
                store.set_numbering(serde_json::from_value(payload)?)?;
                json!({"saved":true})
            }
            "financial_statements" => {
                let input: DateRange = serde_json::from_value(payload)?;
                serde_json::to_value(
                    store.financial_statements(input.from.as_deref(), input.as_of.as_deref())?,
                )?
            }
            "close_period" => {
                let input: CloseInput = serde_json::from_value(payload)?;
                serde_json::to_value(store.close_period(
                    &input.start,
                    &input.end,
                    input.equity_account_id,
                )?)?
            }
            "bank_lines" => {
                let input: BankQuery = serde_json::from_value(payload)?;
                serde_json::to_value(store.bank_lines(input.account_id, input.as_of.as_deref())?)?
            }
            "reconcile" => {
                serde_json::to_value(store.reconcile(serde_json::from_value(payload)?)?)?
            }
            "stock_history" => {
                let input: PageQuery = serde_json::from_value(payload)?;
                store.stock_history(input.offset, input.limit)?
            }
            "ledger_page" => {
                let input: LedgerPageInput = serde_json::from_value(payload)?;
                if input.offset < 0 || !(1..=200).contains(&input.limit) {
                    return Err(StoreError::Conflict("Invalid page.".into()));
                }
                let mut ledger =
                    store.ledger(input.account_id, input.from.as_deref(), input.to.as_deref())?;
                let count = ledger.lines.len();
                ledger.lines = ledger
                    .lines
                    .into_iter()
                    .skip(input.offset as usize)
                    .take(input.limit as usize)
                    .collect();
                let mut value = serde_json::to_value(ledger)?;
                value["line_count"] = json!(count);
                value
            }
            "journals_page" => {
                let input: PageQuery = serde_json::from_value(payload)?;
                serde_json::to_value(store.journals_page(input.offset, input.limit)?)?
            }
            "inspect_tally" => {
                let input: RestoreInput = serde_json::from_value(payload)?;
                serde_json::to_value(crate::inspect_tally(&input.contents)?)?
            }
            "import_tally" => {
                let input: crate::TallyImport = serde_json::from_value(payload)?;
                if store.company()?.is_none_or(|c| c.currency != "INR") {
                    return Err(StoreError::Conflict(
                        "The experimental Tally profile requires INR destination books.".into(),
                    ));
                }
                let journals = input.journals()?;
                import_entries(&mut store, journals, input.preview)?
            }
            "import_journals" => {
                let input: ImportInput = serde_json::from_value(payload)?;
                if input.format != "open-ledger-journals"
                    || input.version != 1
                    || input.journals.len() > 10000
                {
                    return Err(StoreError::Conflict(
                        "Unsupported import format or batch size.".into(),
                    ));
                }
                import_entries(&mut store, input.journals, input.preview)?
            }

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
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;
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

fn valid_book_name(name: &str) -> bool {
    name.strip_prefix("books-")
        .and_then(|s| s.strip_suffix(".sqlite"))
        .is_some_and(|s| Uuid::parse_str(s).is_ok())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookSelection {
    file: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusinessQuery {
    offset: Option<i64>,
    limit: Option<i64>,
    as_of: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DateRange {
    from: Option<String>,
    as_of: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseInput {
    start: String,
    end: String,
    equity_account_id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BankQuery {
    account_id: i64,
    as_of: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    offset: i64,
    limit: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportInput {
    format: String,
    version: u32,
    journals: Vec<JournalInput>,
    preview: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerPageInput {
    account_id: i64,
    from: Option<String>,
    to: Option<String>,
    offset: i64,
    limit: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GstReportQuery {
    from: String,
    to: String,
}

fn import_entries(
    store: &mut LedgerStore,
    journals: Vec<JournalInput>,
    preview: bool,
) -> crate::Result<Value> {
    let (mut stage, _) = LedgerStore::prepare_restore(&store.export_backup()?)?;
    let mut ids = vec![];
    for journal in journals {
        ids.push(stage.post(journal)?);
    }
    let tb = stage.trial_balance(None)?;
    if !preview {
        store.replace_with(&stage)?;
    }
    Ok(json!({"count":ids.len(),"trial_balance":tb}))
}
