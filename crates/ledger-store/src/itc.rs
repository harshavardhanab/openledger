//! Reviewed temporary ITC reversals and bounded reclaims. No legal eligibility is inferred.
use crate::{
    Document, GstProfile, InputCredit, LedgerStore, Result,
    store::{insert_business_journal, invalid},
};
use ledger_core::{
    JournalInput, PostingInput, Side, ValidatedJournal, clean_text, decimal_amount, parse_amount,
    validate_date, validate_journal,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItcAdjustmentInput {
    pub invoice_id: i64,
    /// None creates a temporary reversal; Some links a reclaim to its original reversal.
    pub deferral_id: Option<i64>,
    pub pending_account_id: i64,
    pub date: String,
    pub cgst: String,
    pub sgst: String,
    pub igst: String,
    pub reason: String,
    pub eligibility_reviewed: bool,
    pub request_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItcAdjustment {
    pub id: i64,
    pub journal_id: i64,
    pub input: ItcAdjustmentInput,
}
#[derive(Debug, Serialize)]
pub struct ItcAdjustmentView {
    #[serde(flatten)]
    pub record: ItcAdjustment,
    pub remaining: InputCredit,
}
pub(crate) fn records(conn: &Connection) -> Result<Vec<ItcAdjustment>> {
    let mut q = conn.prepare("SELECT data FROM itc_adjustments ORDER BY id")?;
    q.query_map([], |r| r.get::<_, String>(0))?
        .map(|r| Ok(serde_json::from_str(&r?)?))
        .collect()
}
fn values(i: &ItcAdjustmentInput) -> Result<[i64; 3]> {
    let parse = |s: &str| -> Result<i64> {
        if matches!(s, "0" | "0.0" | "0.00") {
            Ok(0)
        } else {
            parse_amount(s).map_err(Into::into)
        }
    };
    Ok([parse(&i.cgst)?, parse(&i.sgst)?, parse(&i.igst)?])
}
pub(crate) fn pending_account(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM itc_adjustments WHERE pending_account_id=?1)",
        [id],
        |r| r.get(0),
    )?)
}
pub(crate) fn check_return(
    conn: &Connection,
    invoice: i64,
    date: &str,
    before: Option<i64>,
) -> Result<()> {
    let mut pending = [0i128; 3];
    for a in records(conn)?
        .into_iter()
        .filter(|a| a.input.invoice_id == invoice && before.is_none_or(|b| a.journal_id < b))
    {
        if a.input.date.as_str() > date {
            return Err(invalid(
                "Supplier returns must follow all linked ITC adjustments.",
            ));
        }
        let sign = if a.input.deferral_id.is_some() { -1 } else { 1 };
        for (total, n) in pending.iter_mut().zip(values(&a.input)?) {
            *total += sign * i128::from(n);
        }
    }
    if pending != [0; 3] {
        return Err(invalid(
            "Resolve all pending ITC through reviewed linked reclaims before returning this purchase.",
        ));
    }
    Ok(())
}
fn prepare(
    tx: &Transaction<'_>,
    input: &ItcAdjustmentInput,
    before: Option<i64>,
) -> Result<ValidatedJournal> {
    validate_date(&input.date)?;
    clean_text(&input.reason, "ITC review basis", 500, true)?;
    clean_text(&input.request_id, "Request identifier", 124, true)?;
    if !input.eligibility_reviewed {
        return Err(invalid(
            "Review the temporary reversal or reclaim eligibility and supporting evidence.",
        ));
    }
    let close: Option<String> = tx.query_row("SELECT MAX(end) FROM periods", [], |r| r.get(0))?;
    if close.is_some_and(|d| input.date <= d) {
        return Err(invalid("This financial period is closed."));
    }
    let raw: String = tx
        .query_row(
            "SELECT data FROM documents WHERE id=?1",
            [input.invoice_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| invalid("Purchase invoice not found."))?;
    let invoice: Document = serde_json::from_str(&raw)?;
    if invoice.input.kind != "purchase"
        || invoice
            .input
            .gst
            .as_ref()
            .is_none_or(|g| g.classification.is_some())
        || before.is_some_and(|b| invoice.journal_id >= b)
    {
        return Err(invalid(
            "Link ITC adjustments to an earlier regular taxable purchase.",
        ));
    }
    let reversed: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM journals WHERE reversal_of=?1)",
        [invoice.journal_id],
        |r| r.get(0),
    )?;
    let returned: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM documents WHERE invoice_id=?1 AND kind='debit_note' AND (?2 IS NULL OR journal_id<?2))", params![invoice.id,before], |r| r.get(0))?;
    if reversed || returned {
        return Err(invalid(
            "Reversed or returned purchases cannot have new ITC adjustments.",
        ));
    }
    let profile: String =
        tx.query_row("SELECT data FROM gst_profile WHERE id=1", [], |r| r.get(0))?;
    let profile: GstProfile = serde_json::from_str(&profile)?;
    let pending = input.pending_account_id;
    let kind: Option<String> = tx
        .query_row("SELECT kind FROM accounts WHERE id=?1", [pending], |r| {
            r.get(0)
        })
        .optional()?;
    let used: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM parties WHERE account_id=?1 UNION ALL SELECT 1 FROM items WHERE asset_account_id=?1 UNION ALL SELECT 1 FROM lines l WHERE account_id=?1 AND NOT EXISTS(SELECT 1 FROM itc_adjustments a WHERE a.journal_id=l.journal_id) AND (?2 IS NULL OR l.journal_id<?2))",params![pending,before],|r|r.get(0))?;
    if kind.as_deref() != Some("asset")
        || used
        || [
            profile.input.cgst,
            profile.input.sgst,
            profile.input.igst,
            profile.output.cgst,
            profile.output.sgst,
            profile.output.igst,
        ]
        .contains(&pending)
    {
        return Err(invalid(
            "Use a dedicated pending ITC asset account, separate from GST controls, parties, inventory and other postings.",
        ));
    }
    let all = records(tx)?;
    let prior: Vec<_> = all
        .iter()
        .filter(|a| a.input.invoice_id == invoice.id && before.is_none_or(|b| a.journal_id < b))
        .collect();
    if input.date < invoice.input.date || prior.iter().any(|a| input.date < a.input.date) {
        return Err(invalid(
            "ITC adjustments must follow the purchase and its latest ITC adjustment date.",
        ));
    }
    let requested = values(input)?;
    let mut available = invoice
        .gst
        .as_ref()
        .ok_or_else(|| invalid("Missing invoice tax."))?
        .credit_values()?
        .map(i128::from);
    if let Some(id) = input.deferral_id {
        let original = prior
            .iter()
            .find(|a| a.id == id && a.input.deferral_id.is_none())
            .ok_or_else(|| invalid("Select an earlier temporary reversal for this purchase."))?;
        if original.input.pending_account_id != pending {
            return Err(invalid(
                "A reclaim must use its original pending ITC account.",
            ));
        }
        available = values(&original.input)?.map(i128::from);
        for a in &prior {
            if a.input.deferral_id == Some(id) {
                for (remaining, n) in available.iter_mut().zip(values(&a.input)?) {
                    *remaining -= i128::from(n);
                }
            }
        }
    } else {
        for a in &prior {
            let sign = if a.input.deferral_id.is_some() { 1 } else { -1 };
            for (remaining, n) in available.iter_mut().zip(values(&a.input)?) {
                *remaining += sign * i128::from(n);
            }
        }
    }
    if requested.iter().all(|n| *n == 0)
        || requested
            .iter()
            .zip(available)
            .any(|(n, a)| i128::from(*n) > a)
    {
        return Err(invalid(
            "ITC adjustment must be positive and cannot exceed the remaining reviewed credit for any component.",
        ));
    }
    let mut lines = vec![];
    let mut total = 0i64;
    for (id, n) in [profile.input.cgst, profile.input.sgst, profile.input.igst]
        .into_iter()
        .zip(requested)
    {
        if n == 0 {
            continue;
        }
        total = total
            .checked_add(n)
            .ok_or_else(|| invalid("ITC adjustment exceeds range."))?;
        lines.push(PostingInput {
            account_id: id,
            side: if input.deferral_id.is_some() {
                Side::Debit
            } else {
                Side::Credit
            },
            amount: decimal_amount(n),
        });
    }
    lines.push(PostingInput {
        account_id: pending,
        side: if input.deferral_id.is_some() {
            Side::Credit
        } else {
            Side::Debit
        },
        amount: decimal_amount(total),
    });
    validate_journal(JournalInput {
        date: input.date.clone(),
        reference: format!("ITC/{}", invoice.number),
        description: input.reason.clone(),
        request_id: format!("itc:{}", input.request_id),
        lines,
    })
    .map_err(Into::into)
}
fn save(tx: &Transaction<'_>, a: &ItcAdjustment) -> Result<()> {
    tx.execute("INSERT INTO itc_adjustments(id,invoice_id,deferral_id,pending_account_id,journal_id,request_id,data) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![a.id,a.input.invoice_id,a.input.deferral_id,a.input.pending_account_id,a.journal_id,a.input.request_id,serde_json::to_string(a)?])?;
    Ok(())
}
impl LedgerStore {
    pub fn preview_itc_adjustment(
        &mut self,
        input: ItcAdjustmentInput,
    ) -> Result<ValidatedJournal> {
        let tx = self.conn.transaction()?;
        prepare(&tx, &input, None)
    }
    pub fn post_itc_adjustment(&mut self, input: ItcAdjustmentInput) -> Result<ItcAdjustment> {
        self.require_company()?;
        let tx = self.conn.transaction()?;
        let old: Option<String> = tx
            .query_row(
                "SELECT data FROM itc_adjustments WHERE request_id=?1",
                [&input.request_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            let old: ItcAdjustment = serde_json::from_str(&old)?;
            if old.input != input {
                return Err(invalid(
                    "This request already posted a different ITC adjustment.",
                ));
            }
            return Ok(old);
        }
        let p = prepare(&tx, &input, None)?;
        let journal_id = insert_business_journal(&tx, &p, "document")?;
        let id = tx.query_row(
            "SELECT COALESCE(MAX(id),0)+1 FROM itc_adjustments",
            [],
            |r| r.get(0),
        )?;
        let a = ItcAdjustment {
            id,
            journal_id,
            input,
        };
        save(&tx, &a)?;
        tx.commit()?;
        Ok(a)
    }
    pub fn itc_adjustments(&self, invoice: i64) -> Result<Vec<ItcAdjustmentView>> {
        let _read = self.conn.unchecked_transaction()?;
        let all = records(&self.conn)?;
        all.iter()
            .filter(|a| a.input.invoice_id == invoice)
            .map(|a| {
                let mut remaining = values(&a.input)?;
                if a.input.deferral_id.is_none() {
                    for claim in all.iter().filter(|c| c.input.deferral_id == Some(a.id)) {
                        for (r, n) in remaining.iter_mut().zip(values(&claim.input)?) {
                            *r -= n;
                        }
                    }
                } else {
                    remaining = [0; 3];
                }
                Ok(ItcAdjustmentView {
                    record: a.clone(),
                    remaining: InputCredit::from_values(remaining),
                })
            })
            .collect()
    }
    pub(crate) fn restore_itc(&mut self, mut all: Vec<ItcAdjustment>) -> Result<()> {
        all.sort_by_key(|a| a.id);
        let tx = self.conn.transaction()?;
        let mut last_journal = 0;
        for a in all {
            if a.id <= 0 || a.journal_id <= last_journal {
                return Err(invalid("Invalid ITC adjustment order or identifiers."));
            }
            let expected = prepare(&tx, &a.input, Some(a.journal_id))?;
            let (payload,source,reversed):(String,String,bool)=tx.query_row("SELECT request_payload,source,reversal_of IS NOT NULL OR EXISTS(SELECT 1 FROM journals r WHERE r.reversal_of=journals.id) FROM journals WHERE id=?1",[a.journal_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
            let actual: ValidatedJournal = serde_json::from_str(&payload)?;
            if serde_json::to_string(&actual)? != serde_json::to_string(&expected)?
                || source != "document"
                || reversed
            {
                return Err(invalid(
                    "ITC adjustment does not match its immutable posting.",
                ));
            }
            save(&tx, &a)?;
            last_journal = a.journal_id;
        }
        let mut q =
            tx.prepare("SELECT invoice_id,date,journal_id FROM documents WHERE kind='debit_note'")?;
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })? {
            let (invoice, date, id) = row?;
            check_return(&tx, invoice, &date, Some(id))?;
        }
        drop(q);
        // A pending account must remain dedicated even when forged metadata is supplied.
        let other: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM lines l JOIN itc_adjustments a ON a.pending_account_id=l.account_id WHERE NOT EXISTS(SELECT 1 FROM itc_adjustments own WHERE own.journal_id=l.journal_id))",[],|r|r.get(0))?;
        if other {
            return Err(invalid("Pending ITC account contains an unlinked posting."));
        }
        tx.commit()?;
        Ok(())
    }
}

/// A purchase and its same-date temporary reversal are staged together. The destination
/// never exposes an intermediate balance claiming the held credit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredPurchaseInput {
    pub document: crate::DocumentInput,
    pub pending_account_id: i64,
    pub reason: String,
    pub eligibility_reviewed: bool,
}
impl LedgerStore {
    pub fn deferred_purchase(
        &mut self,
        input: DeferredPurchaseInput,
        commit: bool,
    ) -> Result<serde_json::Value> {
        if input.document.kind != "purchase"
            || input
                .document
                .gst
                .as_ref()
                .is_none_or(|g| g.classification.is_some())
        {
            return Err(invalid(
                "Initial ITC deferral requires a regular taxable purchase.",
            ));
        }
        let (mut stage, _) = Self::prepare_restore(&self.export_backup()?)?;
        let document = stage.post_document(input.document)?;
        let credit = document
            .gst
            .as_ref()
            .ok_or_else(|| invalid("Missing purchase tax."))?
            .credit_values()?;
        let adjustment = stage.post_itc_adjustment(ItcAdjustmentInput {
            invoice_id: document.id,
            deferral_id: None,
            pending_account_id: input.pending_account_id,
            date: document.input.date.clone(),
            cgst: decimal_amount(credit[0]),
            sgst: decimal_amount(credit[1]),
            igst: decimal_amount(credit[2]),
            reason: input.reason,
            eligibility_reviewed: input.eligibility_reviewed,
            request_id: format!("purchase:{}", document.input.request_id),
        })?;
        let postings = [document.journal_id, adjustment.journal_id]
            .into_iter()
            .map(|id| stage.journal(id))
            .collect::<Result<Vec<_>>>()?;
        if commit {
            self.replace_with(&stage)?;
        }
        Ok(serde_json::json!({"document":document,"adjustment":adjustment,"postings":postings}))
    }
}
