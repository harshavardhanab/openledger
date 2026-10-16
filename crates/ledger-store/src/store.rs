use chrono::Utc;
use ledger_core::*;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("{0}")]
    Validation(#[from] ValidationError),
    #[error("Could not complete the database operation: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("The backup could not be read: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Conflict(String),
    #[error("Could not access the data file: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, StoreError>;
pub(crate) fn invalid(message: &str) -> StoreError {
    StoreError::Conflict(message.into())
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyInput {
    pub name: String,
    pub currency: String,
    pub fiscal_year_start: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Company {
    pub name: String,
    pub currency: String,
    pub fiscal_year_start: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub kind: AccountKind,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountInput {
    pub code: String,
    pub name: String,
    pub kind: AccountKind,
}
#[derive(Debug, Clone, Serialize)]
pub struct JournalLine {
    pub id: i64,
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub side: Side,
    pub amount_minor: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Journal {
    pub id: i64,
    pub date: String,
    pub reference: String,
    pub description: String,
    pub created_at: String,
    pub total_minor: String,
    pub reversal_of: Option<i64>,
    pub reversed_by: Option<i64>,
    pub lines: Vec<JournalLine>,
}
#[derive(Debug, Clone, Serialize)]
pub struct BalanceRow {
    pub account: Account,
    pub net_debit_minor: String,
    pub debit_minor: String,
    pub credit_minor: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrialBalance {
    pub rows: Vec<BalanceRow>,
    pub debit_minor: String,
    pub credit_minor: String,
    pub balanced: bool,
    pub as_of: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub company: Option<Company>,
    pub accounts: Vec<Account>,
    pub journals: Vec<Journal>,
    pub journal_count: i64,
    pub journal_activity: BTreeMap<String, i64>,
    pub trial_balance: TrialBalance,
    pub statements: crate::FinancialStatements,
}
#[derive(Debug, Clone, Serialize)]
pub struct LedgerLine {
    pub journal_id: i64,
    pub date: String,
    pub reference: String,
    pub description: String,
    pub debit_minor: String,
    pub credit_minor: String,
    pub balance_minor: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Ledger {
    pub account: Account,
    pub lines: Vec<LedgerLine>,
    pub opening_minor: String,
    pub closing_minor: String,
    pub debit_minor: String,
    pub credit_minor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupLine {
    #[serde(default)]
    id: Option<i64>,
    account_id: i64,
    side: Side,
    amount_minor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupJournal {
    #[serde(default = "manual_source")]
    source: String,
    id: i64,
    date: String,
    reference: String,
    description: String,
    request_id: String,
    created_at: String,
    reversal_of: Option<i64>,
    lines: Vec<BackupLine>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    format: String,
    version: u32,
    exported_at: String,
    company: Company,
    accounts: Vec<Account>,
    journals: Vec<BackupJournal>,
    #[serde(default)]
    business: crate::business::BusinessBackup,
}
#[derive(Debug, Serialize)]
pub struct BackupPreview {
    pub company_name: String,
    pub currency: String,
    pub exported_at: String,
    pub account_count: usize,
    pub journal_count: usize,
}

pub struct LedgerStore {
    pub(crate) conn: Connection,
}
impl LedgerStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::initialize(Connection::open(path)?, true)
    }
    pub fn inspect_company(path: &Path) -> Result<Option<Company>> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let app_id: i64 = conn.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if app_id != 1330399058 || !(1..=5).contains(&version) {
            return Err(invalid(
                "Books file is unsupported or belongs to another application.",
            ));
        }
        Self { conn }.company()
    }
    pub fn memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?, false)
    }
    fn initialize(conn: Connection, wal: bool) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let app_id: i64 = conn.pragma_query_value(None, "application_id", |r| r.get(0))?;
        if app_id != 0 && app_id != 1330399058 {
            return Err(invalid("This file belongs to a different application."));
        }
        if version > 5 {
            return Err(invalid(
                "This database needs a newer version of Open Ledger.",
            ));
        }
        if version == 0 {
            let existing:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0))?;
            if existing != 0 {
                return Err(invalid("The selected file is not an Open Ledger database."));
            }
            conn.execute_batch(include_str!("schema.sql"))?;
        } else if app_id != 1330399058 {
            return Err(invalid("The database identity is missing."));
        }
        if version < 2 {
            conn.execute_batch(include_str!("migration_v2.sql"))?;
        }
        if version < 3 {
            conn.execute_batch(include_str!("migration_v3.sql"))?;
        }
        if version < 4 {
            // SQLite's documented table rebuild procedure preserves child foreign keys.
            // Disable FK enforcement outside the transaction; validate before committing.
            conn.pragma_update(None, "foreign_keys", false)?;
            let result = (|| -> Result<()> {
                conn.execute_batch("BEGIN IMMEDIATE")?;
                conn.execute_batch(include_str!("migration_v4.sql"))?;
                let broken: i64 =
                    conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                        r.get(0)
                    })?;
                if broken != 0 {
                    return Err(invalid("Schema migration found broken references."));
                }
                conn.execute_batch("COMMIT")?;
                Ok(())
            })();
            if result.is_err() && !conn.is_autocommit() {
                conn.execute_batch("ROLLBACK")?;
            }
            conn.pragma_update(None, "foreign_keys", true)?;
            result?;
        }
        if version < 5 {
            conn.execute_batch(include_str!("migration_v5.sql"))?;
        }
        if wal {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self { conn })
    }
    pub fn company(&self) -> Result<Option<Company>> {
        Ok(self
            .conn
            .query_row(
                "SELECT name,currency,fiscal_year_start,created_at FROM company WHERE id=1",
                [],
                |r| {
                    Ok(Company {
                        name: r.get(0)?,
                        currency: r.get(1)?,
                        fiscal_year_start: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }
    pub(crate) fn require_company(&self) -> Result<Company> {
        self.company()?
            .ok_or_else(|| invalid("Create your company before adding accounts or journals."))
    }
    pub fn create_company(&mut self, input: CompanyInput) -> Result<Company> {
        if self.company()?.is_some() {
            return Err(invalid(
                "This data file already has a company. Restore a backup to change books.",
            ));
        }
        let company = validated_company(Company {
            name: input.name,
            currency: input.currency,
            fiscal_year_start: input.fiscal_year_start,
            created_at: now(),
        })?;
        let tx = self.conn.transaction()?;
        insert_company(&tx, &company)?;
        for (code, name, kind) in [
            ("1000", "Cash", AccountKind::Asset),
            ("1100", "Bank", AccountKind::Asset),
            ("1200", "Accounts receivable", AccountKind::Asset),
            ("1500", "Equipment", AccountKind::Asset),
            ("2000", "Accounts payable", AccountKind::Liability),
            ("3000", "Owner capital", AccountKind::Equity),
            ("4000", "Sales income", AccountKind::Income),
            ("5000", "Operating expenses", AccountKind::Expense),
        ] {
            tx.execute(
                "INSERT INTO accounts(code,name,kind) VALUES(?1,?2,?3)",
                params![code, name, kind.as_str()],
            )?;
        }
        tx.commit()?;
        Ok(company)
    }
    pub fn accounts(&self) -> Result<Vec<Account>> {
        let mut q = self
            .conn
            .prepare("SELECT id,code,name,kind FROM accounts ORDER BY code COLLATE NOCASE,id")?;
        let raw = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        raw.into_iter()
            .map(|(id, code, name, kind)| {
                Ok(Account {
                    id,
                    code,
                    name,
                    kind: AccountKind::parse(&kind)?,
                })
            })
            .collect()
    }
    pub fn create_account(&mut self, input: AccountInput) -> Result<Account> {
        self.require_company()?;
        let (code, name) = validate_account(&input.code, &input.name)?;
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM accounts WHERE code=?1 COLLATE NOCASE",
            [&code],
            |r| r.get(0),
        )?;
        if exists > 0 {
            return Err(invalid("An account already uses that code."));
        }
        self.conn.execute(
            "INSERT INTO accounts(code,name,kind) VALUES(?1,?2,?3)",
            params![code, name, input.kind.as_str()],
        )?;
        Ok(Account {
            id: self.conn.last_insert_rowid(),
            code,
            name,
            kind: input.kind,
        })
    }
    pub fn post(&mut self, input: JournalInput) -> Result<i64> {
        self.require_company()?;
        let posting = validate_journal(input)?;
        let payload = serde_json::to_string(&posting)?;
        let tx = self.conn.transaction()?;
        if let Some((id, original)) = tx
            .query_row(
                "SELECT id,request_payload FROM journals WHERE request_id=?1",
                [&posting.request_id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if original == payload {
                return Ok(id);
            }
            return Err(invalid(
                "This request already posted a different journal. Start a new entry.",
            ));
        }
        crate::business::check_stock_accounts(&tx, &posting)?;
        let id = insert_journal(&tx, &posting, None, None, &now())?;
        tx.commit()?;
        Ok(id)
    }
    pub fn reverse(&mut self, id: i64, date: &str, reason: &str, request_id: &str) -> Result<i64> {
        self.check_business_reversal(id)?;
        self.check_business_reversal_date(id, date)?;
        let original = self.journal(id)?;
        validate_date(date)?;
        if date < original.date.as_str() {
            return Err(invalid(
                "A reversal cannot be dated before the original entry.",
            ));
        }
        if original.reversal_of.is_some() {
            return Err(invalid(
                "A reversal entry cannot itself be reversed in this milestone.",
            ));
        }
        let reason = clean_text(reason, "Reason", 500, true)?;
        let posting = validate_journal(JournalInput {
            date: date.into(),
            reference: format!(
                "REV-{}",
                original.reference.chars().take(60).collect::<String>()
            ),
            description: format!("Reversal of {}: {}", original.reference, reason),
            request_id: request_id.into(),
            lines: original
                .lines
                .iter()
                .map(|line| {
                    Ok(PostingInput {
                        account_id: line.account_id,
                        side: line.side.opposite(),
                        amount: decimal_amount(
                            line.amount_minor
                                .parse::<i64>()
                                .map_err(|_| invalid("A stored amount is invalid."))?,
                        ),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        })?;
        if let Some(reversed) = original.reversed_by {
            let (old_request, old_payload): (String, String) = self.conn.query_row(
                "SELECT request_id,request_payload FROM journals WHERE id=?1",
                [reversed],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if old_request == request_id && old_payload == serde_json::to_string(&posting)? {
                return Ok(reversed);
            }
            return Err(invalid("This journal has already been reversed."));
        }
        let tx = self.conn.transaction()?;
        let result = insert_journal(&tx, &posting, None, Some(id), &now())?;
        tx.commit()?;
        Ok(result)
    }
    pub fn journal(&self, id: i64) -> Result<Journal> {
        let mut journal=self.conn.query_row("SELECT j.id,j.date,j.reference,j.description,j.created_at,j.total_minor,j.reversal_of,(SELECT id FROM journals r WHERE r.reversal_of=j.id AND r.status='posted') FROM journals j WHERE j.id=?1 AND j.status='posted'",[id],|r|Ok(Journal{id:r.get(0)?,date:r.get(1)?,reference:r.get(2)?,description:r.get(3)?,created_at:r.get(4)?,total_minor:r.get::<_,i64>(5)?.to_string(),reversal_of:r.get(6)?,reversed_by:r.get(7)?,lines:vec![]})).optional()?.ok_or_else(||invalid("Journal entry not found."))?;
        let mut q=self.conn.prepare("SELECT l.account_id,a.code,a.name,l.side,l.amount_minor,l.id FROM lines l JOIN accounts a ON a.id=l.account_id WHERE l.journal_id=?1 ORDER BY l.position")?;
        let raw = q
            .query_map([id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        journal.lines = raw
            .into_iter()
            .map(
                |(account_id, account_code, account_name, side, amount, id)| {
                    Ok(JournalLine {
                        id,
                        account_id,
                        account_code,
                        account_name,
                        side: Side::parse(&side)?,
                        amount_minor: amount.to_string(),
                    })
                },
            )
            .collect::<Result<_>>()?;
        Ok(journal)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        let company = self.company()?;
        let accounts = self.accounts()?;
        let mut q = self.conn.prepare(
            "SELECT id FROM journals WHERE status='posted' ORDER BY date DESC,id DESC LIMIT 500",
        )?;
        let ids = q
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let journals = ids
            .into_iter()
            .map(|id| self.journal(id))
            .collect::<Result<_>>()?;
        let journal_count = self.conn.query_row(
            "SELECT COUNT(*) FROM journals WHERE status='posted'",
            [],
            |r| r.get(0),
        )?;
        let mut activity_query = self
            .conn
            .prepare("SELECT date,COUNT(*) FROM journals WHERE status='posted' GROUP BY date")?;
        let journal_activity = activity_query
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<std::result::Result<BTreeMap<_, _>, _>>()?;
        Ok(Snapshot {
            company,
            accounts,
            journals,
            journal_count,
            journal_activity,
            trial_balance: self.trial_balance(None)?,
            statements: self.financial_statements(None, None)?,
        })
    }
    pub fn trial_balance(&self, as_of: Option<&str>) -> Result<TrialBalance> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        if let Some(date) = as_of {
            validate_date(date)?;
        }
        let mut balances: BTreeMap<i64, i128> = BTreeMap::new();
        let mut q=self.conn.prepare("SELECT l.account_id,l.side,l.amount_minor FROM lines l JOIN journals j ON j.id=l.journal_id WHERE j.status='posted' AND (?1 IS NULL OR j.date<=?1)")?;
        for item in q.query_map([as_of], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })? {
            let (id, side, amount) = item?;
            *balances.entry(id).or_default() += if side == "debit" {
                i128::from(amount)
            } else {
                -i128::from(amount)
            };
        }
        let mut debit = 0i128;
        let mut credit = 0i128;
        let rows = self
            .accounts()?
            .into_iter()
            .map(|account| {
                let value = balances.get(&account.id).copied().unwrap_or(0);
                let d = value.max(0);
                let c = (-value).max(0);
                debit += d;
                credit += c;
                BalanceRow {
                    account,
                    net_debit_minor: value.to_string(),
                    debit_minor: d.to_string(),
                    credit_minor: c.to_string(),
                }
            })
            .collect();
        Ok(TrialBalance {
            rows,
            debit_minor: debit.to_string(),
            credit_minor: credit.to_string(),
            balanced: debit == credit,
            as_of: as_of.map(str::to_owned),
        })
    }
    pub fn ledger(&self, account_id: i64, from: Option<&str>, to: Option<&str>) -> Result<Ledger> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        if let Some(d) = from {
            validate_date(d)?;
        }
        if let Some(d) = to {
            validate_date(d)?;
        }
        if from.zip(to).is_some_and(|(a, b)| a > b) {
            return Err(invalid("The start date must be before the end date."));
        }
        let account = self
            .accounts()?
            .into_iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| invalid("Account not found."))?;
        let mut q=self.conn.prepare("SELECT j.id,j.date,j.reference,j.description,l.side,l.amount_minor FROM lines l JOIN journals j ON j.id=l.journal_id WHERE l.account_id=?1 AND j.status='posted' AND (?2 IS NULL OR j.date<=?2) ORDER BY j.date,j.id,l.position")?;
        let raw = q
            .query_map(params![account_id, to], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut balance = 0i128;
        let mut opening = 0i128;
        let mut debit = 0i128;
        let mut credit = 0i128;
        let mut lines = vec![];
        for (id, date, reference, description, side, amount) in raw {
            let value = i128::from(amount);
            balance += if side == "debit" { value } else { -value };
            if from.is_some_and(|d| date.as_str() < d) {
                opening = balance;
                continue;
            }
            let d = if side == "debit" { value } else { 0 };
            let c = if side == "credit" { value } else { 0 };
            debit += d;
            credit += c;
            lines.push(LedgerLine {
                journal_id: id,
                date,
                reference,
                description,
                debit_minor: d.to_string(),
                credit_minor: c.to_string(),
                balance_minor: balance.to_string(),
            });
        }
        Ok(Ledger {
            account,
            lines,
            opening_minor: opening.to_string(),
            closing_minor: balance.to_string(),
            debit_minor: debit.to_string(),
            credit_minor: credit.to_string(),
        })
    }
    pub fn export_backup(&self) -> Result<String> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        let company = self.require_company()?;
        let accounts = self.accounts()?;
        let mut q=self.conn.prepare("SELECT id,date,reference,description,request_id,created_at,reversal_of,source FROM journals WHERE status='posted' ORDER BY id")?;
        let mut journals = q
            .query_map([], |r| {
                Ok(BackupJournal {
                    source: r.get(7)?,
                    id: r.get(0)?,
                    date: r.get(1)?,
                    reference: r.get(2)?,
                    description: r.get(3)?,
                    request_id: r.get(4)?,
                    created_at: r.get(5)?,
                    reversal_of: r.get(6)?,
                    lines: vec![],
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for journal in &mut journals {
            let payload: String = self.conn.query_row(
                "SELECT request_payload FROM journals WHERE id=?1",
                [journal.id],
                |r| r.get(0),
            )?;
            journal.reference = serde_json::from_str::<ValidatedJournal>(&payload)?.reference;
            journal.lines = self
                .journal(journal.id)?
                .lines
                .into_iter()
                .map(|l| BackupLine {
                    id: Some(l.id),
                    account_id: l.account_id,
                    side: l.side,
                    amount_minor: l.amount_minor,
                })
                .collect();
        }
        Ok(serde_json::to_string_pretty(&Backup {
            format: "open-ledger-backup".into(),
            version: 5,
            exported_at: now(),
            company,
            accounts,
            journals,
            business: self.business_backup()?,
        })?)
    }
    pub fn prepare_restore(text: &str) -> Result<(Self, BackupPreview)> {
        if text.len() > 20 * 1024 * 1024 {
            return Err(invalid("This milestone supports backups up to 20 MB."));
        }
        let raw: serde_json::Value = serde_json::from_str(text)?;
        if raw
            .get("version")
            .and_then(|v| v.as_u64())
            .is_some_and(|n| n >= 2)
            && raw.get("business").is_none()
        {
            return Err(invalid(
                "Version 2 and newer backups require business records.",
            ));
        }
        let mut backup: Backup = serde_json::from_value(raw)?;
        if backup.format != "open-ledger-backup" || !(1..=5).contains(&backup.version) {
            return Err(invalid("This is not a supported Open Ledger backup."));
        }
        if backup.version == 1 && !backup.business.is_empty() {
            return Err(invalid(
                "Version 1 backups cannot contain business records.",
            ));
        }
        if backup.version < 3
            && (backup.business.gst.is_some()
                || backup
                    .business
                    .documents
                    .iter()
                    .any(|d| d.gst.is_some() || d.input.gst.is_some()))
        {
            return Err(invalid("GST records require backup version 3."));
        }
        if backup.version < 5
            && backup.business.documents.iter().any(|d| {
                d.input
                    .gst
                    .as_ref()
                    .is_some_and(|g| g.classification.is_some())
                    || d.correction
                        .as_ref()
                        .and_then(|c| c.original_gst.as_ref())
                        .is_some_and(|g| g.classification.is_some())
            })
        {
            return Err(invalid(
                "Explicit supply classifications require backup version 5.",
            ));
        }
        if backup.version < 4
            && (backup.business.numbering.iter().any(|n| {
                [
                    "credit_note",
                    "debit_note",
                    "customer_refund",
                    "supplier_refund",
                ]
                .contains(&n.kind.as_str())
            }) || backup.business.documents.iter().any(|d| {
                d.correction.is_some()
                    || d.input.gst_adjustment_reviewed.is_some()
                    || d.input.return_lines.is_some()
                    || d.input.reviewed_postings.is_some()
                    || [
                        "credit_note",
                        "debit_note",
                        "customer_refund",
                        "supplier_refund",
                    ]
                    .contains(&d.input.kind.as_str())
            }))
        {
            return Err(invalid("Returns and refunds require backup version 4."));
        }
        if backup.version >= 4
            && !backup.business.is_empty()
            && backup.business.numbering.len() != 10
        {
            return Err(invalid(
                "Version 4 and newer require all ten numbering sequences.",
            ));
        }
        chrono::DateTime::parse_from_rfc3339(&backup.exported_at)
            .map_err(|_| invalid("The backup date is invalid."))?;
        let company = validated_company(backup.company)?;
        let preview = BackupPreview {
            company_name: company.name.clone(),
            currency: company.currency.clone(),
            exported_at: backup.exported_at,
            account_count: backup.accounts.len(),
            journal_count: backup.journals.len(),
        };
        let mut stage = Self::memory()?;
        let tx = stage.conn.transaction()?;
        insert_company(&tx, &company)?;
        let mut ids = BTreeSet::new();
        for account in backup.accounts {
            if account.id <= 0 || !ids.insert(account.id) {
                return Err(invalid(
                    "The backup has duplicate or invalid account identifiers.",
                ));
            }
            let (code, name) = validate_account(&account.code, &account.name)?;
            tx.execute(
                "INSERT INTO accounts(id,code,name,kind) VALUES(?1,?2,?3,?4)",
                params![account.id, code, name, account.kind.as_str()],
            )?;
        }
        backup.journals.sort_by_key(|j| j.id);
        let mut prior: BTreeMap<i64, ValidatedJournal> = BTreeMap::new();
        let mut reversed = BTreeSet::new();
        for journal in backup.journals {
            if journal.id <= 0 || prior.contains_key(&journal.id) {
                return Err(invalid(
                    "The backup has duplicate or invalid journal identifiers.",
                ));
            }
            chrono::DateTime::parse_from_rfc3339(&journal.created_at)
                .map_err(|_| invalid("A journal timestamp is invalid."))?;
            let line_ids = journal.lines.iter().map(|l| l.id).collect::<Vec<_>>();
            if line_ids.iter().flatten().any(|id| *id <= 0) {
                return Err(invalid("Invalid posting line identifier."));
            }
            let lines = journal
                .lines
                .into_iter()
                .map(|l| {
                    let n = l
                        .amount_minor
                        .parse::<i64>()
                        .map_err(|_| invalid("A backup amount is invalid."))?;
                    if n <= 0 {
                        return Err(invalid("A backup amount must be positive."));
                    }
                    Ok(PostingInput {
                        account_id: l.account_id,
                        side: l.side,
                        amount: decimal_amount(n),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let validated = validate_journal(JournalInput {
                date: journal.date,
                reference: journal.reference,
                description: journal.description,
                request_id: journal.request_id,
                lines,
            })?;
            if let Some(original_id) = journal.reversal_of {
                let original = prior
                    .get(&original_id)
                    .ok_or_else(|| invalid("A reversal refers to a missing or later journal."))?;
                if !reversed.insert(original_id) || validated.date < original.date {
                    return Err(invalid(
                        "A backup reversal is duplicated or incorrectly dated.",
                    ));
                }
                let signature = |lines: &[Posting], opposite: bool| {
                    let mut values = lines
                        .iter()
                        .map(|l| {
                            (
                                l.account_id,
                                if opposite { l.side.opposite() } else { l.side },
                                l.amount_minor,
                            )
                        })
                        .collect::<Vec<_>>();
                    values.sort();
                    values
                };
                if signature(&original.lines, true) != signature(&validated.lines, false) {
                    return Err(invalid(
                        "The backup's reversal does not match the original journal.",
                    ));
                }
                let was_reversal: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM journals WHERE id=?1 AND reversal_of IS NOT NULL",
                    [original_id],
                    |r| r.get(0),
                )?;
                if was_reversal > 0 {
                    return Err(invalid("The backup contains a reversal of a reversal."));
                }
            }
            insert_journal_with_ids(
                &tx,
                &validated,
                Some(journal.id),
                journal.reversal_of,
                &journal.created_at,
                &line_ids,
                &journal.source,
            )?;
            prior.insert(journal.id, validated);
        }
        tx.commit()?;
        stage.restore_business(backup.business)?;
        if !stage.trial_balance(None)?.balanced {
            return Err(invalid("The restored trial balance does not balance."));
        }
        Ok((stage, preview))
    }
    pub fn replace_with(&mut self, stage: &Self) -> Result<()> {
        let backup = rusqlite::backup::Backup::new(&stage.conn, &mut self.conn)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
        Ok(())
    }
    pub fn restore(&mut self, text: &str) -> Result<BackupPreview> {
        let (stage, preview) = Self::prepare_restore(text)?;
        self.replace_with(&stage)?;
        Ok(preview)
    }
}
fn validated_company(mut company: Company) -> Result<Company> {
    company.name = clean_text(&company.name, "Company name", 120, true)?;
    validate_currency(&company.currency)?;
    validate_date(&company.fiscal_year_start)?;
    chrono::DateTime::parse_from_rfc3339(&company.created_at)
        .map_err(|_| invalid("The company timestamp is invalid."))?;
    Ok(company)
}
fn insert_company(tx: &Transaction<'_>, c: &Company) -> Result<()> {
    tx.execute(
        "INSERT INTO company(id,name,currency,fiscal_year_start,created_at) VALUES(1,?1,?2,?3,?4)",
        params![c.name, c.currency, c.fiscal_year_start, c.created_at],
    )?;
    Ok(())
}
pub(crate) fn insert_journal(
    tx: &Transaction<'_>,
    p: &ValidatedJournal,
    id: Option<i64>,
    reversal_of: Option<i64>,
    created: &str,
) -> Result<i64> {
    insert_journal_with_ids(tx, p, id, reversal_of, created, &[], "manual")
}
fn insert_journal_with_ids(
    tx: &Transaction<'_>,
    p: &ValidatedJournal,
    id: Option<i64>,
    reversal_of: Option<i64>,
    created: &str,
    line_ids: &[Option<i64>],
    source: &str,
) -> Result<i64> {
    if !["manual", "document", "close"].contains(&source) {
        return Err(invalid("Unknown journal source."));
    }
    for line in &p.lines {
        let found: i64 = tx.query_row(
            "SELECT COUNT(*) FROM accounts WHERE id=?1",
            [line.account_id],
            |r| r.get(0),
        )?;
        if found != 1 {
            return Err(invalid("A posting account does not exist."));
        }
    }
    let payload = serde_json::to_string(p)?;
    tx.execute("INSERT INTO journals(id,date,reference,description,request_id,request_payload,created_at,total_minor,reversal_of,source) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,p.date,p.reference,p.description,p.request_id,payload,created,p.total_minor,reversal_of,source])?;
    let id = tx.last_insert_rowid();
    if p.reference.is_empty() {
        tx.execute(
            "UPDATE journals SET reference=?1 WHERE id=?2",
            params![format!("JRN-{id:06}"), id],
        )?;
    }
    for (position, line) in p.lines.iter().enumerate() {
        tx.execute("INSERT INTO lines(id,journal_id,position,account_id,side,amount_minor) VALUES(?1,?2,?3,?4,?5,?6)",params![line_ids.get(position).copied().flatten(),id,position as i64,line.account_id,line.side.as_str(),line.amount_minor])?;
    }
    tx.execute("UPDATE journals SET status='posted' WHERE id=?1", [id])?;
    Ok(id)
}

fn manual_source() -> String {
    "manual".into()
}
pub(crate) fn insert_business_journal(
    tx: &Transaction<'_>,
    p: &ValidatedJournal,
    source: &str,
) -> Result<i64> {
    insert_journal_with_ids(tx, p, None, None, &now(), &[], source)
}

#[cfg(test)]
mod sql_guards {
    use super::*;
    #[test]
    fn direct_database_writes_cannot_change_posted_journals() {
        let mut store = LedgerStore::memory().unwrap();
        store
            .create_company(CompanyInput {
                name: "Test".into(),
                currency: "INR".into(),
                fiscal_year_start: "2026-01-01".into(),
            })
            .unwrap();
        let id = store
            .post(JournalInput {
                date: "2026-10-14".into(),
                reference: "".into(),
                description: "Capital".into(),
                request_id: "guard".into(),
                lines: vec![
                    PostingInput {
                        account_id: 2,
                        side: Side::Debit,
                        amount: "100".into(),
                    },
                    PostingInput {
                        account_id: 6,
                        side: Side::Credit,
                        amount: "100".into(),
                    },
                ],
            })
            .unwrap();
        for sql in [
            format!("UPDATE journals SET description='changed' WHERE id={id}"),
            format!("DELETE FROM journals WHERE id={id}"),
            format!("UPDATE lines SET amount_minor=1 WHERE journal_id={id}"),
            format!("DELETE FROM lines WHERE journal_id={id}"),
            format!(
                "INSERT INTO lines(journal_id,position,account_id,side,amount_minor) VALUES({id},9,2,'debit',1)"
            ),
            "UPDATE accounts SET kind='income' WHERE id=2".into(),
            "UPDATE company SET currency='USD'".into(),
        ] {
            assert!(store.conn.execute(&sql, []).is_err(), "{sql}");
        }
        assert!(store.trial_balance(None).unwrap().balanced);
    }
}
