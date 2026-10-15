use crate::store::{insert_business_journal, invalid};
use crate::{LedgerStore, Result};
use ledger_core::*;
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyInput {
    pub name: String,
    pub kind: String,
    pub contact: String,
    pub account_id: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Party {
    pub id: i64,
    pub input: PartyInput,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemInput {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub asset_account_id: i64,
    pub expense_account_id: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: i64,
    pub input: ItemInput,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentLine {
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub account_id: i64,
    pub item_id: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gst_adjustment_reviewed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_lines: Option<Vec<crate::ReturnLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_postings: Option<Vec<PostingInput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gst: Option<crate::GstInput>,
    pub kind: String,
    pub date: String,
    pub due_date: Option<String>,
    pub party_id: Option<i64>,
    pub invoice_id: Option<i64>,
    pub money_account_id: Option<i64>,
    pub amount: Option<String>,
    pub note: String,
    pub lines: Vec<DocumentLine>,
    pub request_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction: Option<crate::Correction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gst: Option<crate::GstCalculation>,
    pub id: i64,
    pub number: String,
    pub journal_id: i64,
    pub total_minor: String,
    pub input: DocumentInput,
}
#[derive(Debug, Clone, Serialize)]
pub struct DocumentView {
    #[serde(flatten)]
    pub document: Document,
    pub paid_minor: String,
    pub credited_minor: String,
    pub outstanding_minor: String,
    pub reversed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Numbering {
    pub kind: String,
    pub prefix: String,
    pub next: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StockMove {
    pub id: i64,
    pub item_id: i64,
    pub document_id: i64,
    pub quantity_milli: i64,
    pub value_minor: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StockBalance {
    pub item: Item,
    pub quantity_milli: String,
    pub value_minor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub id: i64,
    pub start: String,
    pub end: String,
    pub journal_id: Option<i64>,
    pub equity_account_id: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReconcileInput {
    pub account_id: i64,
    pub start: String,
    pub end: String,
    pub opening: String,
    pub closing: String,
    pub reference: String,
    pub line_ids: Vec<i64>,
    pub request_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reconciliation {
    pub id: i64,
    pub input: ReconcileInput,
}
#[derive(Debug, Clone, Serialize)]
pub struct BankLine {
    pub id: i64,
    pub journal_id: i64,
    pub date: String,
    pub reference: String,
    pub description: String,
    pub net_minor: String,
    pub reconciliation_id: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BusinessBackup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gst: Option<crate::GstProfile>,
    pub parties: Vec<Party>,
    pub items: Vec<Item>,
    pub documents: Vec<Document>,
    pub stock_moves: Vec<StockMove>,
    pub numbering: Vec<Numbering>,
    pub periods: Vec<Period>,
    pub reconciliations: Vec<Reconciliation>,
}
impl BusinessBackup {
    pub(crate) fn is_empty(&self) -> bool {
        self.gst.is_none()
            && self.parties.is_empty()
            && self.items.is_empty()
            && self.documents.is_empty()
            && self.stock_moves.is_empty()
            && self.numbering.is_empty()
            && self.periods.is_empty()
            && self.reconciliations.is_empty()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct BusinessState {
    pub gst: Option<crate::GstProfile>,
    pub parties: Vec<Party>,
    pub items: Vec<Item>,
    pub documents: Vec<DocumentView>,
    pub stock: Vec<StockBalance>,
    pub numbering: Vec<Numbering>,
    pub periods: Vec<Period>,
    pub reconciliations: Vec<Reconciliation>,
    pub document_count: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatementRow {
    pub account_id: i64,
    pub code: String,
    pub name: String,
    pub kind: AccountKind,
    pub amount_minor: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct FinancialStatements {
    pub from: Option<String>,
    pub as_of: Option<String>,
    pub profit_loss: Vec<StatementRow>,
    pub balance_sheet: Vec<StatementRow>,
    pub income_minor: String,
    pub expense_minor: String,
    pub profit_minor: String,
    pub assets_minor: String,
    pub liabilities_minor: String,
    pub equity_minor: String,
    pub current_earnings_minor: String,
    pub balanced: bool,
}

fn account_kind(tx: &Transaction<'_>, id: i64, kinds: &[&str]) -> Result<()> {
    let kind: Option<String> = tx
        .query_row("SELECT kind FROM accounts WHERE id=?1", [id], |r| r.get(0))
        .optional()?;
    if !kind.is_some_and(|k| kinds.contains(&k.as_str())) {
        return Err(invalid("Select an account of the correct type."));
    }
    Ok(())
}
fn validate_party(tx: &Transaction<'_>, p: &PartyInput) -> Result<()> {
    if !["customer", "supplier"].contains(&p.kind.as_str()) {
        return Err(invalid("Select customer or supplier."));
    }
    clean_text(&p.name, "Party name", 120, true)?;
    clean_text(&p.contact, "Contact", 500, false)?;
    account_kind(
        tx,
        p.account_id,
        if p.kind == "customer" {
            &["asset"]
        } else {
            &["liability"]
        },
    )?;
    crate::gst::check_gst_control(tx, p.account_id)?;
    let stock: i64 = tx.query_row(
        "SELECT COUNT(*) FROM items WHERE asset_account_id=?1",
        [p.account_id],
        |r| r.get(0),
    )?;
    if stock > 0 {
        return Err(invalid(
            "Inventory accounts cannot be used for party balances.",
        ));
    }
    Ok(())
}
fn validate_item(tx: &Transaction<'_>, i: &ItemInput) -> Result<()> {
    validate_account(&i.code, &i.name)?;
    clean_text(&i.unit, "Unit", 20, true)?;
    account_kind(tx, i.asset_account_id, &["asset"])?;
    crate::gst::check_gst_control(tx, i.asset_account_id)?;
    account_kind(tx, i.expense_account_id, &["expense"])?;
    let used:i64=tx.query_row("SELECT (SELECT COUNT(*) FROM parties WHERE account_id=?1)+(SELECT COUNT(*) FROM lines WHERE account_id=?1)",[i.asset_account_id],|r|r.get(0))?;
    // Once a dedicated inventory account has stock history, additional items may share it.
    let inventory: i64 = tx.query_row(
        "SELECT COUNT(*) FROM items WHERE asset_account_id=?1",
        [i.asset_account_id],
        |r| r.get(0),
    )?;
    if used > 0 && inventory == 0 {
        return Err(invalid("Use a new, dedicated asset account for inventory."));
    }
    Ok(())
}
// Three decimal places for quantities; integers throughout, round monetary extensions half up.
pub fn parse_quantity(s: &str) -> Result<i64> {
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    if (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err(invalid(
            "Quantity must be positive with at most three decimal places.",
        ));
    }
    let w = if whole.is_empty() {
        0
    } else {
        whole
            .parse::<i64>()
            .map_err(|_| invalid("Quantity is too large."))?
    };
    let f = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<3}")
            .parse::<i64>()
            .map_err(|_| invalid("Invalid quantity."))?
    };
    let n = w
        .checked_mul(1000)
        .and_then(|v| v.checked_add(f))
        .ok_or_else(|| invalid("Quantity is too large."))?;
    if n <= 0 {
        return Err(invalid("Quantity must be positive."));
    }
    Ok(n)
}
fn rounded(n: i128, d: i128) -> Result<i64> {
    i64::try_from((n + d / 2) / d).map_err(|_| invalid("Amount exceeds the supported range."))
}
fn signed_amount(s: &str) -> Result<i128> {
    let raw = s.trim();
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if !whole.is_empty()
        && whole.bytes().all(|b| b == b'0')
        && fraction.len() <= 2
        && fraction.bytes().all(|b| b == b'0')
    {
        return Ok(0);
    }
    if let Some(v) = s.strip_prefix('-') {
        Ok(-i128::from(parse_amount(v)?))
    } else {
        Ok(i128::from(parse_amount(s)?))
    }
}
pub(crate) fn check_stock_accounts(tx: &Transaction<'_>, p: &ValidatedJournal) -> Result<()> {
    for l in &p.lines {
        let n: i64 = tx.query_row(
            "SELECT COUNT(*) FROM items WHERE asset_account_id=?1",
            [l.account_id],
            |r| r.get(0),
        )?;
        if n > 0 {
            return Err(invalid(
                "Use stock documents to post to an inventory asset account.",
            ));
        }
    }
    Ok(())
}
pub(crate) fn stock_totals(tx: &Transaction<'_>, item_id: i64) -> Result<(i64, i64)> {
    let mut q =
        tx.prepare("SELECT quantity_milli,value_minor FROM stock_moves WHERE item_id=?1")?;
    let mut qty = 0i128;
    let mut value = 0i128;
    for r in q.query_map([item_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (a, b) = r?;
        qty += i128::from(a);
        value += i128::from(b);
    }
    Ok((
        i64::try_from(qty).map_err(|_| invalid("Stock quantity exceeds range."))?,
        i64::try_from(value).map_err(|_| invalid("Stock value exceeds range."))?,
    ))
}
pub(crate) struct Prepared {
    pub(crate) correction: Option<crate::Correction>,
    pub(crate) gst: Option<crate::GstCalculation>,
    pub(crate) total: i64,
    pub(crate) lines: Vec<PostingInput>,
    pub(crate) moves: Vec<(i64, i64, i64)>,
}
fn prepare_document(
    tx: &Transaction<'_>,
    d: &DocumentInput,
    replay_before: Option<i64>,
) -> Result<Prepared> {
    validate_date(&d.date)?;
    clean_text(&d.request_id, "Request identifier", 90, true)?;
    clean_text(&d.note, "Note", 500, false)?;
    let last_close: Option<String> =
        tx.query_row("SELECT MAX(end) FROM periods", [], |r| r.get(0))?;
    if last_close.is_some_and(|s| d.date <= s) {
        return Err(invalid("This financial period is closed."));
    }
    if ["credit_note", "debit_note"].contains(&d.kind.as_str()) {
        return crate::corrections::prepare_correction(tx, d, replay_before);
    }
    if d.gst_adjustment_reviewed.is_some() {
        return Err(invalid(
            "GST adjustment review applies only to return notes.",
        ));
    }
    if d.return_lines.is_some() {
        return Err(invalid("Return quantities require a credit or debit note."));
    }
    if d.gst.is_some() && !["sale", "purchase"].contains(&d.kind.as_str()) {
        return Err(invalid("Only invoices support GST."));
    }
    let mut result = Prepared {
        correction: None,
        gst: None,
        total: 0,
        lines: vec![],
        moves: vec![],
    };
    if ["receipt", "payment", "customer_refund", "supplier_refund"].contains(&d.kind.as_str()) {
        if !d.lines.is_empty() || d.due_date.is_some() {
            return Err(invalid(
                "Settlements cannot have invoice lines or due dates.",
            ));
        }
        let invoice_id = d
            .invoice_id
            .ok_or_else(|| invalid("Select the invoice to settle."))?;
        let (kind, party, journal, total, date): (String, i64, i64, i64, String) = tx
            .query_row(
                "SELECT kind,party_id,journal_id,total_minor,date FROM documents WHERE id=?1",
                [invoice_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?
            .ok_or_else(|| invalid("Invoice not found."))?;
        let customer = ["receipt", "customer_refund"].contains(&d.kind.as_str());
        let refund = ["customer_refund", "supplier_refund"].contains(&d.kind.as_str());
        if (customer && kind != "sale")
            || (!customer && kind != "purchase")
            || d.party_id != Some(party)
        {
            return Err(invalid("Settlement type and party must match the invoice."));
        }
        if d.date < date {
            return Err(invalid("A settlement cannot precede its invoice."));
        }
        let reversed: i64 = tx.query_row(
            "SELECT COUNT(*) FROM journals WHERE reversal_of=?1 AND (?2 IS NULL OR id<?2)",
            params![journal, replay_before],
            |r| r.get(0),
        )?;
        if reversed > 0 {
            return Err(invalid("A reversed invoice cannot be settled."));
        }
        let latest:Option<String>=tx.query_row("SELECT MAX(date) FROM (SELECT d.date FROM documents d WHERE d.invoice_id=?1 UNION ALL SELECT r.date FROM documents d JOIN journals r ON r.reversal_of=d.journal_id WHERE d.invoice_id=?1 AND (?2 IS NULL OR r.id<?2))",params![invoice_id,replay_before],|r|r.get(0))?;
        if latest.is_some_and(|s| d.date < s) {
            return Err(invalid(
                "Settlements must follow the latest settlement or reversal date for this invoice.",
            ));
        }
        let (paid, credited) =
            crate::corrections::allocations(tx, invoice_id, None, replay_before)?;
        result.total = parse_amount(
            d.amount
                .as_deref()
                .ok_or_else(|| invalid("Enter a settlement amount."))?,
        )?;
        let outstanding = i128::from(total) - paid - credited;
        let available = if refund { -outstanding } else { outstanding };
        if i128::from(result.total) > available {
            return Err(invalid(
                "Settlement exceeds the invoice's outstanding balance or available refund credit.",
            ));
        }
        let bank = d
            .money_account_id
            .ok_or_else(|| invalid("Select a cash or bank account."))?;
        account_kind(tx, bank, &["asset"])?;
        crate::gst::check_gst_control(tx, bank)?;
        let control: i64 =
            tx.query_row("SELECT account_id FROM parties WHERE id=?1", [party], |r| {
                r.get(0)
            })?;
        let used:i64=tx.query_row("SELECT (SELECT COUNT(*) FROM items WHERE asset_account_id=?1)+(SELECT COUNT(*) FROM parties WHERE account_id=?1)",[bank],|r|r.get(0))?;
        if bank == control || used > 0 {
            return Err(invalid("Use a separate cash or bank asset account."));
        }
        let side = if ["receipt", "supplier_refund"].contains(&d.kind.as_str()) {
            Side::Debit
        } else {
            Side::Credit
        };
        result.lines = vec![
            PostingInput {
                account_id: bank,
                side,
                amount: decimal_amount(result.total),
            },
            PostingInput {
                account_id: control,
                side: side.opposite(),
                amount: decimal_amount(result.total),
            },
        ];
        return Ok(result);
    }
    if !["sale", "purchase", "stock"].contains(&d.kind.as_str()) {
        return Err(invalid("Unknown document type."));
    }
    if d.invoice_id.is_some() || d.amount.is_some() || d.money_account_id.is_some() {
        return Err(invalid(
            "Invoice or stock document contains settlement fields.",
        ));
    }
    if !(1..=100).contains(&d.lines.len()) {
        return Err(invalid("Enter between 1 and 100 document lines."));
    }
    let control = if d.kind == "stock" {
        if d.party_id.is_some() || d.due_date.is_some() {
            return Err(invalid(
                "Stock adjustments do not have parties or due dates.",
            ));
        }
        None
    } else {
        let pid = d
            .party_id
            .ok_or_else(|| invalid("Select a customer or supplier."))?;
        let (kind, account): (String, i64) = tx
            .query_row(
                "SELECT kind,account_id FROM parties WHERE id=?1",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| invalid("Party not found."))?;
        if (d.kind == "sale" && kind != "customer") || (d.kind == "purchase" && kind != "supplier")
        {
            return Err(invalid("Party type does not match invoice type."));
        }
        let due = d
            .due_date
            .as_deref()
            .ok_or_else(|| invalid("Enter a due date."))?;
        validate_date(due)?;
        if due < d.date.as_str() {
            return Err(invalid("Due date cannot precede invoice date."));
        }
        Some(account)
    };
    let mut seen = BTreeSet::new();
    let mut total = 0i128;
    for line in &d.lines {
        clean_text(&line.description, "Line description", 200, true)?;
        let qty = parse_quantity(&line.quantity)?;
        let price = parse_amount(&line.unit_price)?;
        let value = rounded(i128::from(qty) * i128::from(price), 1000)?;
        if value <= 0 {
            return Err(invalid("A line amount rounds to zero."));
        }
        total += i128::from(value);
        let side = if d.kind == "sale" {
            Side::Credit
        } else {
            Side::Debit
        };
        if let Some(item_id) = line.item_id {
            if !seen.insert(item_id) {
                return Err(invalid("Combine repeated inventory items into one line."));
            }
            let (asset, expense): (i64, i64) = tx
                .query_row(
                    "SELECT asset_account_id,expense_account_id FROM items WHERE id=?1",
                    [item_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| invalid("Stock item not found."))?;
            let latest: Option<String> = tx.query_row(
                "SELECT MAX(d.date) FROM stock_moves m JOIN documents d ON d.id=m.document_id",
                [],
                |r| r.get(0),
            )?;
            if latest.is_some_and(|s| d.date < s) {
                return Err(invalid(
                    "Stock documents must follow the latest stock movement date.",
                ));
            }
            let (available, carrying) = stock_totals(tx, item_id)?;
            if d.kind == "purchase" || (d.kind == "stock" && line.account_id != asset) {
                if d.kind == "purchase" && line.account_id != asset {
                    return Err(invalid(
                        "A stock purchase must use its item's inventory account.",
                    ));
                }
                if d.kind == "stock" {
                    account_kind(tx, line.account_id, &["equity", "expense", "income"])?;
                    result.lines.push(PostingInput {
                        account_id: line.account_id,
                        side: Side::Credit,
                        amount: decimal_amount(value),
                    });
                }
                available
                    .checked_add(qty)
                    .ok_or_else(|| invalid("Stock quantity exceeds range."))?;
                carrying
                    .checked_add(value)
                    .ok_or_else(|| invalid("Stock value exceeds range."))?;
                result.lines.push(PostingInput {
                    account_id: asset,
                    side: Side::Debit,
                    amount: decimal_amount(value),
                });
                result.moves.push((item_id, qty, value));
            } else {
                if available < qty {
                    return Err(invalid(
                        "Insufficient stock. Negative inventory is not allowed.",
                    ));
                }
                let cost = if qty == available {
                    carrying
                } else {
                    rounded(
                        i128::from(carrying) * i128::from(qty),
                        i128::from(available),
                    )?
                };
                if cost > 0 {
                    result.lines.push(PostingInput {
                        account_id: expense,
                        side: Side::Debit,
                        amount: decimal_amount(cost),
                    });
                    result.lines.push(PostingInput {
                        account_id: asset,
                        side: Side::Credit,
                        amount: decimal_amount(cost),
                    });
                }
                result.moves.push((item_id, -qty, -cost));
                if d.kind == "stock" {
                    total -= i128::from(value);
                    total += i128::from(cost);
                } else {
                    account_kind(tx, line.account_id, &["income"])?;
                    result.lines.push(PostingInput {
                        account_id: line.account_id,
                        side,
                        amount: decimal_amount(value),
                    });
                }
            }
        } else {
            if d.kind == "stock" {
                return Err(invalid("Stock adjustments need an item on every line."));
            }
            account_kind(
                tx,
                line.account_id,
                if d.kind == "sale" {
                    &["income"]
                } else {
                    &["expense", "asset"]
                },
            )?;
            let stock: i64 = tx.query_row(
                "SELECT COUNT(*) FROM items WHERE asset_account_id=?1",
                [line.account_id],
                |r| r.get(0),
            )?;
            if stock > 0 {
                return Err(invalid(
                    "Select an inventory item when using an inventory account.",
                ));
            }
            if control == Some(line.account_id) {
                return Err(invalid(
                    "Invoice line cannot use the party control account.",
                ));
            }
            result.lines.push(PostingInput {
                account_id: line.account_id,
                side,
                amount: decimal_amount(value),
            });
        }
    }
    result.total = i64::try_from(total).map_err(|_| invalid("Invoice total exceeds range."))?;
    if result.total <= 0 {
        return Err(invalid("Document total must be positive."));
    }
    if let Some((calc, accounts)) = crate::gst::calculate_gst(tx, d)? {
        for (id, minor) in [
            (accounts.cgst, &calc.cgst_minor),
            (accounts.sgst, &calc.sgst_minor),
            (accounts.igst, &calc.igst_minor),
        ] {
            let n = minor
                .parse::<i64>()
                .map_err(|_| invalid("Invalid GST total."))?;
            if n > 0 {
                result.total = result
                    .total
                    .checked_add(n)
                    .ok_or_else(|| invalid("Invoice total exceeds range."))?;
                result.lines.push(PostingInput {
                    account_id: id,
                    side: if d.kind == "sale" {
                        Side::Credit
                    } else {
                        Side::Debit
                    },
                    amount: decimal_amount(n),
                });
            }
        }
        result.gst = Some(calc);
    }
    if let Some(account) = control {
        result.lines.push(PostingInput {
            account_id: account,
            side: if d.kind == "sale" {
                Side::Debit
            } else {
                Side::Credit
            },
            amount: decimal_amount(result.total),
        });
    }
    Ok(result)
}
fn posting(d: &DocumentInput, p: &Prepared, number: &str) -> Result<ValidatedJournal> {
    Ok(validate_journal(JournalInput {
        date: d.date.clone(),
        reference: number.into(),
        description: format!(
            "{} {}{}",
            d.kind,
            number,
            if d.note.is_empty() {
                String::new()
            } else {
                format!(": {}", d.note)
            }
        ),
        request_id: format!("document:{}", d.request_id),
        lines: p.lines.clone(),
    })?)
}
fn next_number(tx: &Transaction<'_>, kind: &str) -> Result<String> {
    let (prefix, next): (String, i64) = tx.query_row(
        "SELECT prefix,next FROM numbering WHERE kind=?1",
        [kind],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let n = next
        .checked_add(1)
        .ok_or_else(|| invalid("Number sequence exhausted."))?;
    tx.execute(
        "UPDATE numbering SET next=?1 WHERE kind=?2",
        params![n, kind],
    )?;
    Ok(format!("{prefix}{next:06}"))
}
fn save_document(tx: &Transaction<'_>, doc: &Document, p: &Prepared) -> Result<()> {
    tx.execute("INSERT INTO documents(id,kind,number,date,party_id,invoice_id,journal_id,total_minor,request_id,request_payload,data) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![doc.id,doc.input.kind,doc.number,doc.input.date,doc.input.party_id,doc.input.invoice_id,doc.journal_id,p.total,doc.input.request_id,serde_json::to_string(&doc.input)?,serde_json::to_string(doc)?])?;
    for &(item, qty, value) in &p.moves {
        tx.execute("INSERT INTO stock_moves(item_id,document_id,quantity_milli,value_minor) VALUES(?1,?2,?3,?4)",params![item,doc.id,qty,value])?;
    }
    Ok(())
}
impl LedgerStore {
    pub fn create_party(&mut self, mut input: PartyInput) -> Result<Party> {
        self.require_company()?;
        input.name = clean_text(&input.name, "Party name", 120, true)?;
        input.contact = clean_text(&input.contact, "Contact", 500, false)?;
        let tx = self.conn.transaction()?;
        validate_party(&tx, &input)?;
        tx.execute(
            "INSERT INTO parties(name,kind,contact,account_id) VALUES(?1,?2,?3,?4)",
            params![input.name, input.kind, input.contact, input.account_id],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Party { id, input })
    }
    pub fn create_item(&mut self, mut input: ItemInput) -> Result<Item> {
        self.require_company()?;
        let (code, name) = validate_account(&input.code, &input.name)?;
        input.code = code;
        input.name = name;
        input.unit = clean_text(&input.unit, "Unit", 20, true)?;
        let tx = self.conn.transaction()?;
        validate_item(&tx, &input)?;
        tx.execute("INSERT INTO items(code,name,unit,asset_account_id,expense_account_id) VALUES(?1,?2,?3,?4,?5)",params![input.code,input.name,input.unit,input.asset_account_id,input.expense_account_id])?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Item { id, input })
    }
    pub fn post_document(&mut self, input: DocumentInput) -> Result<Document> {
        self.require_company()?;
        let tx = self.conn.transaction()?;
        if let Some((data, payload)) = tx
            .query_row(
                "SELECT data,request_payload FROM documents WHERE request_id=?1",
                [&input.request_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if payload != serde_json::to_string(&input)? {
                return Err(invalid("This request already posted a different document."));
            }
            return Ok(serde_json::from_str(&data)?);
        }
        let prepared = prepare_document(&tx, &input, None)?;
        if let Some(expected) = &input.reviewed_postings
            && expected != &prepared.lines
        {
            return Err(invalid(
                "The posting changed since review. Preview the document again.",
            ));
        }
        let number = next_number(&tx, &input.kind)?;
        crate::gst::validate_gst_number(&input, &number)?;
        let journal_id =
            insert_business_journal(&tx, &posting(&input, &prepared, &number)?, "document")?;
        let id: i64 = tx.query_row("SELECT COALESCE(MAX(id),0)+1 FROM documents", [], |r| {
            r.get(0)
        })?;
        let doc = Document {
            correction: prepared.correction.clone(),
            gst: prepared.gst.clone(),
            id,
            number,
            journal_id,
            total_minor: prepared.total.to_string(),
            input,
        };
        save_document(&tx, &doc, &prepared)?;
        tx.commit()?;
        Ok(doc)
    }
    pub fn set_numbering(&mut self, input: Numbering) -> Result<()> {
        let prefix = clean_text(&input.prefix, "Prefix", 32, true)?;
        if input.next <= 0 || input.next == i64::MAX {
            return Err(invalid(
                "Next number must be a positive integer below the maximum.",
            ));
        }
        if self.conn.execute(
            "UPDATE numbering SET prefix=?1,next=?2 WHERE kind=?3",
            params![prefix, input.next, input.kind],
        )? != 1
        {
            return Err(invalid("Unknown numbering sequence."));
        }
        Ok(())
    }
    pub(crate) fn check_business_reversal_date(&self, id: i64, date: &str) -> Result<()> {
        let linked: Option<i64> = self
            .conn
            .query_row(
                "SELECT invoice_id FROM documents WHERE journal_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if let Some(invoice) = linked {
            let latest: Option<String> = self.conn.query_row("SELECT MAX(date) FROM (SELECT date FROM documents WHERE id=?1 OR invoice_id=?1 UNION ALL SELECT r.date FROM documents d JOIN journals r ON r.reversal_of=d.journal_id WHERE d.invoice_id=?1)", [invoice], |r| r.get(0))?;
            if latest.is_some_and(|latest| date < latest.as_str()) {
                return Err(invalid(
                    "Settlement reversal must follow all related invoice events.",
                ));
            }
        }
        let latest:Option<String>=self.conn.query_row("SELECT MAX(r.date) FROM documents i JOIN documents p ON p.invoice_id=i.id JOIN journals r ON r.reversal_of=p.journal_id WHERE i.journal_id=?1",[id],|r|r.get(0))?;
        if latest.is_some_and(|s| date < s.as_str()) {
            return Err(invalid(
                "Invoice reversal cannot precede the reversal of its settlements.",
            ));
        }
        Ok(())
    }
    pub(crate) fn check_business_reversal(&self, id: i64) -> Result<()> {
        let notes:i64=self.conn.query_row("SELECT COUNT(*) FROM documents WHERE (journal_id=?1 AND kind IN ('credit_note','debit_note')) OR (invoice_id IN (SELECT id FROM documents WHERE journal_id=?1) AND kind IN ('credit_note','debit_note'))",[id],|r|r.get(0))?;
        if notes > 0 {
            return Err(invalid(
                "Credit/debit notes and their original invoices cannot be reversed independently. Use a new replacement invoice for a correction.",
            ));
        }
        let settlement: Option<(i64,String,i64)> = self.conn.query_row("SELECT invoice_id,kind,total_minor FROM documents WHERE journal_id=?1 AND kind IN ('receipt','payment')", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((invoice, _, amount)) = settlement {
            let (paid, _) = crate::corrections::allocations(&self.conn, invoice, None, None)?;
            if paid < i128::from(amount) {
                return Err(invalid(
                    "Reverse linked refunds before reversing their original receipt or payment.",
                ));
            }
        }
        let stock:i64=self.conn.query_row("SELECT COUNT(*) FROM stock_moves m JOIN documents d ON m.document_id=d.id WHERE d.journal_id=?1",[id],|r|r.get(0))?;
        if stock > 0 {
            return Err(invalid(
                "Use an opposite stock adjustment to correct inventory documents; their journals cannot be reversed separately.",
            ));
        }
        let paid:i64=self.conn.query_row("SELECT COUNT(*) FROM documents p JOIN documents i ON i.id=p.invoice_id WHERE i.journal_id=?1 AND NOT EXISTS(SELECT 1 FROM journals r WHERE r.reversal_of=p.journal_id)",[id],|r|r.get(0))?;
        if paid > 0 {
            return Err(invalid(
                "Reverse linked settlements before reversing the invoice.",
            ));
        }
        let close: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM periods WHERE journal_id=?1",
            [id],
            |r| r.get(0),
        )?;
        if close > 0 {
            return Err(invalid("Period closing entries cannot be reversed."));
        }
        Ok(())
    }
    fn business_metadata(&self) -> Result<BusinessBackup> {
        let mut result = BusinessBackup {
            gst: self.gst_profile()?,
            ..Default::default()
        };
        let mut q = self
            .conn
            .prepare("SELECT id,name,kind,contact,account_id FROM parties ORDER BY id")?;
        result.parties = q
            .query_map([], |r| {
                Ok(Party {
                    id: r.get(0)?,
                    input: PartyInput {
                        name: r.get(1)?,
                        kind: r.get(2)?,
                        contact: r.get(3)?,
                        account_id: r.get(4)?,
                    },
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q = self.conn.prepare(
            "SELECT id,code,name,unit,asset_account_id,expense_account_id FROM items ORDER BY id",
        )?;
        result.items = q
            .query_map([], |r| {
                Ok(Item {
                    id: r.get(0)?,
                    input: ItemInput {
                        code: r.get(1)?,
                        name: r.get(2)?,
                        unit: r.get(3)?,
                        asset_account_id: r.get(4)?,
                        expense_account_id: r.get(5)?,
                    },
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q = self
            .conn
            .prepare("SELECT kind,prefix,next FROM numbering ORDER BY kind")?;
        result.numbering = q
            .query_map([], |r| {
                Ok(Numbering {
                    kind: r.get(0)?,
                    prefix: r.get(1)?,
                    next: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q = self
            .conn
            .prepare("SELECT id,start,end,journal_id,equity_account_id FROM periods ORDER BY id")?;
        result.periods = q
            .query_map([], |r| {
                Ok(Period {
                    id: r.get(0)?,
                    start: r.get(1)?,
                    end: r.get(2)?,
                    journal_id: r.get(3)?,
                    equity_account_id: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q = self
            .conn
            .prepare("SELECT id,request_payload FROM reconciliations ORDER BY id")?;
        for r in q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (id, json) = r?;
            result.reconciliations.push(Reconciliation {
                id,
                input: serde_json::from_str(&json)?,
            });
        }
        Ok(result)
    }
    pub fn business_backup(&self) -> Result<BusinessBackup> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        let mut result = self.business_metadata()?;
        let mut q = self
            .conn
            .prepare("SELECT data FROM documents ORDER BY id")?;
        for r in q.query_map([], |r| r.get::<_, String>(0))? {
            result.documents.push(serde_json::from_str(&r?)?);
        }
        let mut q = self.conn.prepare(
            "SELECT id,item_id,document_id,quantity_milli,value_minor FROM stock_moves ORDER BY id",
        )?;
        result.stock_moves = q
            .query_map([], |r| {
                Ok(StockMove {
                    id: r.get(0)?,
                    item_id: r.get(1)?,
                    document_id: r.get(2)?,
                    quantity_milli: r.get(3)?,
                    value_minor: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(result)
    }
    pub fn business_state(
        &self,
        offset: i64,
        limit: i64,
        as_of: Option<&str>,
    ) -> Result<BusinessState> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        if offset < 0 || !(1..=200).contains(&limit) {
            return Err(invalid("Invalid page."));
        }
        if let Some(s) = as_of {
            validate_date(s)?;
        }
        let b = self.business_metadata()?;
        let mut docs = vec![];
        let mut query=self.conn.prepare("SELECT d.data,EXISTS(SELECT 1 FROM journals r WHERE r.reversal_of=d.journal_id AND (?1 IS NULL OR r.date<=?1)) FROM documents d WHERE (?1 IS NULL OR d.date<=?1) ORDER BY d.date DESC,d.id DESC LIMIT ?2 OFFSET ?3")?;
        for r in query.query_map(params![as_of, limit, offset], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?))
        })? {
            let (json, reversed) = r?;
            let d: Document = serde_json::from_str(&json)?;
            let (paid, credited) = crate::corrections::allocations(&self.conn, d.id, as_of, None)?;
            let total = d
                .total_minor
                .parse::<i128>()
                .map_err(|_| invalid("Invalid stored total."))?;
            let outstanding = if reversed || !["sale", "purchase"].contains(&d.input.kind.as_str())
            {
                0
            } else {
                total - paid - credited
            };
            docs.push(DocumentView {
                document: d,
                paid_minor: paid.to_string(),
                credited_minor: credited.to_string(),
                outstanding_minor: outstanding.to_string(),
                reversed,
            });
        }
        let mut totals: BTreeMap<i64, (i128, i128)> = BTreeMap::new();
        let mut moves=self.conn.prepare("SELECT m.item_id,m.quantity_milli,m.value_minor FROM stock_moves m JOIN documents d ON d.id=m.document_id WHERE (?1 IS NULL OR d.date<=?1)")?;
        for r in moves.query_map([as_of], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })? {
            let (id, q, v) = r?;
            let total = totals.entry(id).or_default();
            total.0 += i128::from(q);
            total.1 += i128::from(v);
        }
        let stock = b
            .items
            .iter()
            .map(|item| {
                let (q, v) = totals.get(&item.id).copied().unwrap_or_default();
                StockBalance {
                    item: item.clone(),
                    quantity_milli: q.to_string(),
                    value_minor: v.to_string(),
                }
            })
            .collect();
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE (?1 IS NULL OR date<=?1)",
            [as_of],
            |r| r.get(0),
        )?;
        Ok(BusinessState {
            gst: b.gst,
            parties: b.parties,
            items: b.items,
            documents: docs,
            stock,
            numbering: b.numbering,
            periods: b.periods,
            reconciliations: b.reconciliations,
            document_count: count,
        })
    }
    pub fn financial_statements(
        &self,
        from: Option<&str>,
        as_of: Option<&str>,
    ) -> Result<FinancialStatements> {
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        if let Some(s) = from {
            validate_date(s)?;
        }
        if from.zip(as_of).is_some_and(|(a, b)| a > b) {
            return Err(invalid("Invalid report date range."));
        }
        let tb = self.trial_balance(as_of)?;
        let mut profit_loss = vec![];
        let mut balance_sheet = vec![];
        let mut income = 0i128;
        let mut expense = 0i128;
        let mut assets = 0i128;
        let mut liabilities = 0i128;
        let mut equity = 0i128;
        let mut earnings = 0i128;
        for row in tb.rows {
            let net = row
                .net_debit_minor
                .parse::<i128>()
                .map_err(|_| invalid("Invalid balance."))?;
            let a = row.account;
            if matches!(a.kind, AccountKind::Income | AccountKind::Expense) {
                earnings -= net;
                let mut value = 0i128;
                let mut q=self.conn.prepare("SELECT l.side,l.amount_minor FROM lines l JOIN journals j ON j.id=l.journal_id WHERE l.account_id=?1 AND j.status='posted' AND (?2 IS NULL OR j.date>=?2) AND (?3 IS NULL OR j.date<=?3) AND NOT EXISTS(SELECT 1 FROM periods p WHERE p.journal_id=j.id)")?;
                for r in q.query_map(params![a.id, from, as_of], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                })? {
                    let (side, n) = r?;
                    value += if side == "debit" {
                        i128::from(n)
                    } else {
                        -i128::from(n)
                    };
                }
                let displayed = if a.kind == AccountKind::Income {
                    -value
                } else {
                    value
                };
                if a.kind == AccountKind::Income {
                    income += displayed;
                } else {
                    expense += displayed;
                }
                profit_loss.push(StatementRow {
                    account_id: a.id,
                    code: a.code,
                    name: a.name,
                    kind: a.kind,
                    amount_minor: displayed.to_string(),
                });
            } else {
                let displayed = if a.kind == AccountKind::Asset {
                    net
                } else {
                    -net
                };
                match a.kind {
                    AccountKind::Asset => assets += displayed,
                    AccountKind::Liability => liabilities += displayed,
                    AccountKind::Equity => equity += displayed,
                    _ => {}
                }
                balance_sheet.push(StatementRow {
                    account_id: a.id,
                    code: a.code,
                    name: a.name,
                    kind: a.kind,
                    amount_minor: displayed.to_string(),
                });
            }
        }
        Ok(FinancialStatements {
            from: from.map(str::to_owned),
            as_of: as_of.map(str::to_owned),
            profit_loss,
            balance_sheet,
            income_minor: income.to_string(),
            expense_minor: expense.to_string(),
            profit_minor: (income - expense).to_string(),
            assets_minor: assets.to_string(),
            liabilities_minor: liabilities.to_string(),
            equity_minor: equity.to_string(),
            current_earnings_minor: earnings.to_string(),
            balanced: assets == liabilities + equity + earnings,
        })
    }
    pub fn close_period(&mut self, start: &str, end: &str, equity: i64) -> Result<Period> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        let company = self.require_company()?;
        validate_date(start)?;
        validate_date(end)?;
        let previous: Option<String> =
            self.conn
                .query_row("SELECT MAX(end) FROM periods", [], |r| r.get(0))?;
        let expected = if let Some(s) = previous {
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map_err(|_| invalid("Invalid date."))?
                .succ_opt()
                .ok_or_else(|| invalid("Date range exhausted."))?
                .format("%Y-%m-%d")
                .to_string()
        } else {
            company.fiscal_year_start
        };
        if start != expected || start > end {
            return Err(invalid(
                "Periods must be consecutive, beginning at the company's fiscal start.",
            ));
        }
        let rows = self.trial_balance(Some(end))?.rows;
        account_kind(&tx, equity, &["equity"])?;
        let mut lines = vec![];
        let mut net = 0i128;
        for r in rows
            .into_iter()
            .filter(|r| matches!(r.account.kind, AccountKind::Income | AccountKind::Expense))
        {
            let n = r
                .net_debit_minor
                .parse::<i128>()
                .map_err(|_| invalid("Invalid balance."))?;
            net += n;
            if n != 0 {
                lines.push(PostingInput {
                    account_id: r.account.id,
                    side: if n > 0 { Side::Credit } else { Side::Debit },
                    amount: decimal_amount(
                        i64::try_from(n.abs())
                            .map_err(|_| invalid("Closing balance exceeds range."))?,
                    ),
                });
            }
        }
        if net != 0 {
            lines.push(PostingInput {
                account_id: equity,
                side: if net > 0 { Side::Debit } else { Side::Credit },
                amount: decimal_amount(
                    i64::try_from(net.abs())
                        .map_err(|_| invalid("Closing balance exceeds range."))?,
                ),
            });
        }
        let journal_id = if lines.is_empty() {
            None
        } else {
            let number = next_number(&tx, "close")?;
            let p = validate_journal(JournalInput {
                date: end.into(),
                reference: number,
                description: format!("Close period {start} to {end}"),
                request_id: format!("close:{end}"),
                lines,
            })?;
            Some(insert_business_journal(&tx, &p, "close")?)
        };
        tx.execute(
            "INSERT INTO periods(start,end,journal_id,equity_account_id) VALUES(?1,?2,?3,?4)",
            params![start, end, journal_id, equity],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Period {
            id,
            start: start.into(),
            end: end.into(),
            journal_id,
            equity_account_id: equity,
        })
    }
    pub fn bank_lines(&self, account: i64, as_of: Option<&str>) -> Result<Vec<BankLine>> {
        if let Some(s) = as_of {
            validate_date(s)?;
        }
        let mut q=self.conn.prepare("SELECT l.id,j.id,j.date,j.reference,j.description,l.side,l.amount_minor,c.reconciliation_id FROM lines l JOIN journals j ON j.id=l.journal_id LEFT JOIN cleared_lines c ON c.line_id=l.id WHERE l.account_id=?1 AND j.status='posted' AND (?2 IS NULL OR j.date<=?2) ORDER BY j.date,j.id,l.position")?;
        Ok(q.query_map(params![account, as_of], |r| {
            let side: String = r.get(5)?;
            let n: i64 = r.get(6)?;
            Ok(BankLine {
                id: r.get(0)?,
                journal_id: r.get(1)?,
                date: r.get(2)?,
                reference: r.get(3)?,
                description: r.get(4)?,
                net_minor: (if side == "debit" {
                    i128::from(n)
                } else {
                    -i128::from(n)
                })
                .to_string(),
                reconciliation_id: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
    }
    pub fn reconcile(&mut self, input: ReconcileInput) -> Result<Reconciliation> {
        self.require_company()?;
        let tx = self.conn.transaction()?;
        if let Some((id, payload)) = tx
            .query_row(
                "SELECT id,request_payload FROM reconciliations WHERE request_id=?1",
                [&input.request_id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if payload != serde_json::to_string(&input)? {
                return Err(invalid(
                    "This request already reconciled another statement.",
                ));
            }
            return Ok(Reconciliation { id, input });
        }
        save_reconciliation(&tx, None, &input)?;
        let id = tx.last_insert_rowid(); // cleared_lines changes last_insert_rowid: fetch the statement explicitly
        let id = tx
            .query_row(
                "SELECT id FROM reconciliations WHERE request_id=?1",
                [&input.request_id],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(id);
        tx.commit()?;
        Ok(Reconciliation { id, input })
    }
    pub fn stock_history(&self, offset: i64, limit: i64) -> Result<serde_json::Value> {
        if offset < 0 || !(1..=200).contains(&limit) {
            return Err(invalid("Invalid page."));
        }
        let mut q=self.conn.prepare("SELECT m.id,i.name,i.unit,d.number,d.date,d.journal_id,m.quantity_milli,m.value_minor FROM stock_moves m JOIN documents d ON d.id=m.document_id JOIN items i ON i.id=m.item_id ORDER BY m.id DESC LIMIT ?1 OFFSET ?2")?;
        let rows=q.query_map(params![limit,offset],|r|Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?,"unit":r.get::<_,String>(2)?,"number":r.get::<_,String>(3)?,"date":r.get::<_,String>(4)?,"journal_id":r.get::<_,i64>(5)?,"quantity_milli":r.get::<_,i64>(6)?.to_string(),"value_minor":r.get::<_,i64>(7)?.to_string()})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM stock_moves", [], |r| r.get(0))?;
        Ok(serde_json::json!({"rows":rows,"count":count}))
    }
    pub fn journals_page(&self, offset: i64, limit: i64) -> Result<Vec<crate::Journal>> {
        if offset < 0 || !(1..=200).contains(&limit) {
            return Err(invalid("Invalid page."));
        }
        let mut q=self.conn.prepare("SELECT id FROM journals WHERE status='posted' ORDER BY date DESC,id DESC LIMIT ?1 OFFSET ?2")?;
        let ids = q
            .query_map(params![limit, offset], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.into_iter().map(|id| self.journal(id)).collect()
    }
    pub(crate) fn restore_business(&mut self, b: BusinessBackup) -> Result<()> {
        if b.is_empty() {
            let missing: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM journals WHERE source!='manual'",
                [],
                |r| r.get(0),
            )?;
            if missing > 0 {
                return Err(invalid("Business records are missing."));
            }
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        if let Some(g) = &b.gst {
            crate::gst::validate_gst_profile(&tx, g)?;
            tx.execute(
                "INSERT INTO gst_profile(id,data) VALUES(1,?1)",
                [serde_json::to_string(g)?],
            )?;
        }
        for p in &b.parties {
            if p.id <= 0 {
                return Err(invalid("Invalid party identifier."));
            }
            validate_party(&tx, &p.input)?;
            tx.execute(
                "INSERT INTO parties(id,name,kind,contact,account_id) VALUES(?1,?2,?3,?4,?5)",
                params![
                    p.id,
                    p.input.name,
                    p.input.kind,
                    p.input.contact,
                    p.input.account_id
                ],
            )?;
        }
        for i in &b.items {
            if i.id <= 0 {
                return Err(invalid("Invalid item identifier."));
            }
            validate_account(&i.input.code, &i.input.name)?;
            clean_text(&i.input.unit, "Unit", 20, true)?;
            account_kind(&tx, i.input.asset_account_id, &["asset"])?;
            account_kind(&tx, i.input.expense_account_id, &["expense"])?;
            let party: i64 = tx.query_row(
                "SELECT COUNT(*) FROM parties WHERE account_id=?1",
                [i.input.asset_account_id],
                |r| r.get(0),
            )?;
            if party > 0 {
                return Err(invalid("Inventory account overlaps with party balances."));
            }
            tx.execute("INSERT INTO items(id,code,name,unit,asset_account_id,expense_account_id) VALUES(?1,?2,?3,?4,?5,?6)",params![i.id,i.input.code,i.input.name,i.input.unit,i.input.asset_account_id,i.input.expense_account_id])?;
        }
        if let Some(g) = &b.gst {
            crate::gst::validate_gst_profile(&tx, g)?;
        }
        let mut docs = b.documents.clone();
        docs.sort_by_key(|d| d.id);
        for d in &docs {
            if d.id <= 0 || d.journal_id <= 0 {
                return Err(invalid("Invalid document identifier."));
            }
            clean_text(&d.number, "Document number", 64, true)?;
            let prepared = prepare_document(&tx, &d.input, Some(d.journal_id))?;
            crate::gst::validate_gst_number(&d.input, &d.number)?;
            if d.gst != prepared.gst {
                return Err(invalid("GST amounts do not match the invoice."));
            }
            if d.correction != prepared.correction {
                return Err(invalid(
                    "Credit/debit note details do not match their original invoice.",
                ));
            }
            if let Some(expected) = &d.input.reviewed_postings
                && expected != &prepared.lines
            {
                return Err(invalid("Reviewed posting does not match the document."));
            }
            if d.total_minor != prepared.total.to_string() {
                return Err(invalid("Document total does not match its lines."));
            }
            let expected = posting(&d.input, &prepared, &d.number)?;
            let (payload, reversal, source): (String, Option<i64>, String) = tx.query_row(
                "SELECT request_payload,reversal_of,source FROM journals WHERE id=?1",
                [d.journal_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            if serde_json::to_string(&expected)? != payload
                || reversal.is_some()
                || source != "document"
            {
                return Err(invalid("Document accounting does not match its journal."));
            }
            save_document(&tx, d, &prepared)?;
        }
        // Derived stock movements are verified against the backup instead of trusted.
        let mut q = tx.prepare(
            "SELECT id,item_id,document_id,quantity_milli,value_minor FROM stock_moves ORDER BY id",
        )?;
        let generated = q
            .query_map([], |r| {
                Ok(StockMove {
                    id: r.get(0)?,
                    item_id: r.get(1)?,
                    document_id: r.get(2)?,
                    quantity_milli: r.get(3)?,
                    value_minor: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(q);
        if generated != b.stock_moves {
            return Err(invalid(
                "Backup stock valuation or movements do not match the documents.",
            ));
        }
        for d in &docs {
            let rev: i64 = tx.query_row(
                "SELECT COUNT(*) FROM journals WHERE reversal_of=?1",
                [d.journal_id],
                |r| r.get(0),
            )?;
            if rev > 0 && ["receipt", "payment"].contains(&d.input.kind.as_str()) {
                // A reversed settlement must not fund a still-active refund.
                let (paid, _) =
                    crate::corrections::allocations(&tx, d.input.invoice_id.unwrap(), None, None)?;
                if paid < 0 {
                    return Err(invalid("Refunds exceed active settlements."));
                }
            }
            if rev > 0
                && (d.correction.is_some()
                    || docs
                        .iter()
                        .any(|n| n.input.invoice_id == Some(d.id) && n.correction.is_some())
                    || b.stock_moves.iter().any(|m| m.document_id == d.id))
            {
                return Err(invalid(
                    "An inventory journal cannot be reversed independently.",
                ));
            }
        }
        let mut kinds = BTreeSet::new();
        for n in b.numbering {
            if !kinds.insert(n.kind.clone())
                || ![
                    "sale",
                    "purchase",
                    "receipt",
                    "payment",
                    "stock",
                    "close",
                    "credit_note",
                    "debit_note",
                    "customer_refund",
                    "supplier_refund",
                ]
                .contains(&n.kind.as_str())
                || n.next <= 0
                || n.next == i64::MAX
            {
                return Err(invalid("Invalid numbering configuration."));
            }
            clean_text(&n.prefix, "Prefix", 32, true)?;
            tx.execute(
                "UPDATE numbering SET prefix=?1,next=?2 WHERE kind=?3",
                params![n.prefix, n.next, n.kind],
            )?;
        }
        if !["sale", "purchase", "receipt", "payment", "stock", "close"]
            .iter()
            .all(|k| kinds.contains(*k))
        {
            return Err(invalid("Missing numbering sequences."));
        }
        // Unlinked inventory journals would undermine the stock-to-ledger agreement.
        let unlinked:i64=tx.query_row("SELECT COUNT(*) FROM lines l JOIN items i ON i.asset_account_id=l.account_id WHERE NOT EXISTS(SELECT 1 FROM documents d WHERE d.journal_id=l.journal_id)",[],|r|r.get(0))?;
        if unlinked > 0 {
            return Err(invalid("An inventory posting has no stock document."));
        }
        tx.commit()?;
        // Replay closing calculations independently, using balances immediately before each closing journal.
        for p in b.periods {
            validate_date(&p.start)?;
            validate_date(&p.end)?;
            let expected = if let Some(prev) = self.business_metadata()?.periods.last() {
                chrono::NaiveDate::parse_from_str(&prev.end, "%Y-%m-%d")
                    .map_err(|_| invalid("Invalid date."))?
                    .succ_opt()
                    .ok_or_else(|| invalid("Invalid period."))?
                    .format("%Y-%m-%d")
                    .to_string()
            } else {
                self.require_company()?.fiscal_year_start
            };
            if p.id <= 0 || p.start != expected || p.start > p.end {
                return Err(invalid("Backup periods must be consecutive."));
            }
            let tx = self.conn.transaction()?;
            account_kind(&tx, p.equity_account_id, &["equity"])?;
            let mut balances: BTreeMap<i64, i128> = BTreeMap::new();
            let mut q=tx.prepare("SELECT l.account_id,l.side,l.amount_minor FROM lines l JOIN journals j ON j.id=l.journal_id JOIN accounts a ON a.id=l.account_id WHERE j.status='posted' AND j.date<=?1 AND a.kind IN ('income','expense') AND (?2 IS NULL OR j.id!=?2)")?;
            for r in q.query_map(params![p.end, p.journal_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })? {
                let (a, s, n) = r?;
                *balances.entry(a).or_default() += if s == "debit" {
                    i128::from(n)
                } else {
                    -i128::from(n)
                };
            }
            drop(q);
            let mut expected: BTreeMap<(i64, Side), i128> = BTreeMap::new();
            let net: i128 = balances.values().sum();
            for (a, n) in balances {
                if n != 0 {
                    expected.insert((a, if n > 0 { Side::Credit } else { Side::Debit }), n.abs());
                }
            }
            if net != 0 {
                expected.insert(
                    (
                        p.equity_account_id,
                        if net > 0 { Side::Debit } else { Side::Credit },
                    ),
                    net.abs(),
                );
            }
            if let Some(id) = p.journal_id {
                let (date, reversal, source): (String, Option<i64>, String) = tx.query_row(
                    "SELECT date,reversal_of,source FROM journals WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )?;
                let reversed: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM journals WHERE reversal_of=?1",
                    [id],
                    |r| r.get(0),
                )?;
                if date != p.end || reversal.is_some() || reversed > 0 || source != "close" {
                    return Err(invalid("Invalid closing journal."));
                }
                let mut actual = BTreeMap::new();
                let mut q = tx.prepare(
                    "SELECT account_id,side,amount_minor FROM lines WHERE journal_id=?1",
                )?;
                for r in q.query_map([id], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })? {
                    let (a, s, n) = r?;
                    *actual.entry((a, Side::parse(&s)?)).or_insert(0i128) += i128::from(n);
                }
                if actual != expected {
                    return Err(invalid(
                        "Closing journal does not clear the period's earnings.",
                    ));
                }
            } else if !expected.is_empty() {
                return Err(invalid("Period is missing its closing journal."));
            }
            tx.execute("INSERT INTO periods(id,start,end,journal_id,equity_account_id) VALUES(?1,?2,?3,?4,?5)",params![p.id,p.start,p.end,p.journal_id,p.equity_account_id])?;
            tx.commit()?;
        }
        let tx = self.conn.transaction()?;
        for r in b.reconciliations {
            save_reconciliation(&tx, Some(r.id), &r.input)?;
        }
        let missing:i64=tx.query_row("SELECT COUNT(*) FROM journals j WHERE (j.source='document' AND NOT EXISTS(SELECT 1 FROM documents d WHERE d.journal_id=j.id)) OR (j.source='close' AND NOT EXISTS(SELECT 1 FROM periods p WHERE p.journal_id=j.id))",[],|r|r.get(0))?;
        if missing > 0 {
            return Err(invalid("A business journal is missing its source record."));
        }
        tx.commit()?;
        Ok(())
    }
}
fn save_reconciliation(
    tx: &Transaction<'_>,
    id: Option<i64>,
    input: &ReconcileInput,
) -> Result<()> {
    account_kind(tx, input.account_id, &["asset"])?;
    validate_date(&input.start)?;
    validate_date(&input.end)?;
    if input.start > input.end || id.is_some_and(|n| n <= 0) {
        return Err(invalid("Invalid statement range or identifier."));
    }
    clean_text(&input.reference, "Statement reference", 120, true)?;
    clean_text(&input.request_id, "Request identifier", 128, true)?;
    let overlap: i64 = tx.query_row(
        "SELECT COUNT(*) FROM reconciliations WHERE account_id=?1 AND end>=?2",
        params![input.account_id, input.start],
        |r| r.get(0),
    )?;
    if overlap > 0 {
        return Err(invalid(
            "Statements must follow the last reconciled statement without overlapping.",
        ));
    }
    let prior:Option<String>=tx.query_row("SELECT closing_minor FROM reconciliations WHERE account_id=?1 ORDER BY end DESC LIMIT 1",[input.account_id],|r|r.get(0)).optional()?;
    let opening = signed_amount(&input.opening)?;
    let closing = signed_amount(&input.closing)?;
    if prior.is_some_and(|s| s != opening.to_string()) {
        return Err(invalid(
            "Statement opening must equal the previous closing balance.",
        ));
    }
    let mut net = 0i128;
    let mut seen = BTreeSet::new();
    for &line in &input.line_ids {
        if !seen.insert(line) {
            return Err(invalid("Duplicate cleared line."));
        }
        let row:Option<(String,String,i64,i64)>=tx.query_row("SELECT j.date,l.side,l.amount_minor,l.account_id FROM lines l JOIN journals j ON j.id=l.journal_id WHERE l.id=?1 AND j.status='posted' AND NOT EXISTS(SELECT 1 FROM cleared_lines c WHERE c.line_id=l.id)",[line],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let (date, side, n, account) =
            row.ok_or_else(|| invalid("A selected line is missing or already cleared."))?;
        // Earlier outstanding transactions can clear on a later statement.
        if account != input.account_id || date > input.end {
            return Err(invalid(
                "Selected line does not belong to this bank statement.",
            ));
        }
        net += if side == "debit" {
            i128::from(n)
        } else {
            -i128::from(n)
        };
    }
    if opening + net != closing {
        return Err(invalid(
            "Cleared transactions do not reconcile to the statement closing balance.",
        ));
    }
    tx.execute("INSERT INTO reconciliations(id,account_id,start,end,opening_minor,closing_minor,reference,request_id,request_payload) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,input.account_id,input.start,input.end,opening.to_string(),closing.to_string(),input.reference,input.request_id,serde_json::to_string(input)?])?;
    let id = tx.last_insert_rowid();
    for &line in &input.line_ids {
        tx.execute(
            "INSERT INTO cleared_lines(line_id,reconciliation_id) VALUES(?1,?2)",
            params![line, id],
        )?;
    }
    Ok(())
}
