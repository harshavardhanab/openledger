//! Immutable quantity-linked returns. Tax amounts and sale costs follow the original invoice.
use crate::{Document, DocumentInput, GstCalculation, GstLine, GstProfile, LedgerStore, Result};
use crate::{
    business::{Prepared, stock_totals},
    store::invalid,
};
use ledger_core::{PostingInput, Side, clean_text, decimal_amount};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReturnLine {
    pub line: usize,
    pub quantity: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReturnDetail {
    pub original_line: usize,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub item_id: Option<i64>,
    pub account_id: i64,
    pub taxable_minor: String,
    pub inventory_cost_minor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Correction {
    pub invoice_number: String,
    pub invoice_date: String,
    pub original_gst: Option<crate::GstInput>,
    pub lines: Vec<ReturnDetail>,
}

pub(crate) fn allocations(
    conn: &Connection,
    invoice: i64,
    as_of: Option<&str>,
    before: Option<i64>,
) -> Result<(i128, i128)> {
    let mut q=conn.prepare("SELECT kind,total_minor FROM documents d WHERE invoice_id=?1 AND (?2 IS NULL OR date<=?2) AND (?3 IS NULL OR journal_id<?3) AND NOT EXISTS(SELECT 1 FROM journals r WHERE r.reversal_of=d.journal_id AND (?2 IS NULL OR r.date<=?2) AND (?3 IS NULL OR r.id<?3))")?;
    let mut paid = 0i128;
    let mut credited = 0i128;
    for row in q.query_map(params![invoice, as_of, before], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (kind, n) = row?;
        match kind.as_str() {
            "receipt" | "payment" => paid += i128::from(n),
            "customer_refund" | "supplier_refund" => paid -= i128::from(n),
            "credit_note" | "debit_note" => credited += i128::from(n),
            _ => return Err(invalid("Invalid linked document type.")),
        }
    }
    Ok((paid, credited))
}
fn original(conn: &Connection, id: i64) -> Result<Document> {
    let raw: Option<String> = conn
        .query_row("SELECT data FROM documents WHERE id=?1", [id], |r| r.get(0))
        .optional()?;
    Ok(serde_json::from_str(
        &raw.ok_or_else(|| invalid("Original invoice not found."))?,
    )?)
}
fn quantities(conn: &Connection, invoice: i64) -> Result<BTreeMap<usize, i64>> {
    let mut q=conn.prepare("SELECT data FROM documents WHERE invoice_id=?1 AND kind IN ('credit_note','debit_note') ORDER BY id")?;
    let mut used = BTreeMap::<usize, i64>::new();
    for row in q.query_map([invoice], |r| r.get::<_, String>(0))? {
        let d: Document = serde_json::from_str(&row?)?;
        for l in d
            .input
            .return_lines
            .ok_or_else(|| invalid("Return quantities are missing."))?
        {
            let n = crate::parse_quantity(&l.quantity)?;
            let p = used.entry(l.line).or_default();
            *p = p
                .checked_add(n)
                .ok_or_else(|| invalid("Return quantity exceeds range."))?;
        }
    }
    Ok(used)
}
fn proportional(n: i64, prior: i64, qty: i64, whole: i64, reserve_tail: bool) -> Result<i64> {
    let round = |q: i64| {
        let rounded = (i128::from(n) * i128::from(q) + i128::from(whole) / 2) / i128::from(whole);
        // Preserve a final minor unit while quantity remains. Otherwise an early
        // rounded credit can strand an unpostable zero-value physical return.
        if reserve_tail && q < whole && n > 0 {
            rounded.min(i128::from(n - 1))
        } else {
            rounded
        }
    };
    i64::try_from(round(prior + qty) - round(prior))
        .map_err(|_| invalid("Return amount exceeds range."))
}
fn minor(s: &str) -> Result<i64> {
    s.parse()
        .map_err(|_| invalid("Invalid stored invoice amount."))
}
fn push(lines: &mut Vec<PostingInput>, id: i64, side: Side, n: i64) {
    if n > 0 {
        lines.push(PostingInput {
            account_id: id,
            side,
            amount: decimal_amount(n),
        });
    }
}
pub(crate) fn prepare_correction(
    tx: &Transaction<'_>,
    d: &DocumentInput,
    before: Option<i64>,
) -> Result<Prepared> {
    clean_text(&d.note, "Return reason", 500, true)?;
    if !d.lines.is_empty()
        || d.due_date.is_some()
        || d.money_account_id.is_some()
        || d.amount.is_some()
        || d.gst.is_some()
    {
        return Err(invalid(
            "A return uses original invoice details and selected quantities only.",
        ));
    }
    let invoice = d
        .invoice_id
        .ok_or_else(|| invalid("Select the original invoice."))?;
    let orig = original(tx, invoice)?;
    let sale = d.kind == "credit_note";
    if orig.input.kind != if sale { "sale" } else { "purchase" }
        || d.party_id != orig.input.party_id
    {
        return Err(invalid(
            "Return type and party must match the original invoice.",
        ));
    }
    let reversed: i64 = tx.query_row(
        "SELECT COUNT(*) FROM journals WHERE reversal_of=?1 AND (?2 IS NULL OR id<?2)",
        params![orig.journal_id, before],
        |r| r.get(0),
    )?;
    let latest:Option<String>=tx.query_row("SELECT MAX(date) FROM (SELECT date FROM documents WHERE id=?1 OR invoice_id=?1 UNION ALL SELECT r.date FROM documents d JOIN journals r ON r.reversal_of=d.journal_id WHERE d.invoice_id=?1 AND (?2 IS NULL OR r.id<?2))",params![invoice,before],|r|r.get(0))?;
    if reversed > 0 || latest.is_some_and(|date| d.date < date) {
        return Err(invalid(
            "Returns must follow the original invoice and its latest settlement, return or reversal; reversed invoices cannot be returned.",
        ));
    }
    crate::itc::check_return(tx, invoice, &d.date, before)?;
    let tax_review = orig
        .input
        .gst
        .as_ref()
        .is_some_and(|g| g.classification.is_none());
    if (tax_review && d.gst_adjustment_reviewed != Some(true))
        || (!tax_review && d.gst_adjustment_reviewed.is_some())
    {
        return Err(invalid(
            "GST returns require explicit review of tax-adjustment eligibility and supporting supplier/recipient documents.",
        ));
    }
    let returns = d
        .return_lines
        .as_ref()
        .ok_or_else(|| invalid("Enter return quantities."))?;
    if !(1..=100).contains(&returns.len()) {
        return Err(invalid("Select between 1 and 100 return lines."));
    }
    let used = quantities(tx, invoice)?;
    let mut seen = BTreeSet::new();
    let mut prepared = Prepared {
        correction: None,
        gst: None,
        total: 0,
        lines: vec![],
        moves: vec![],
    };
    let mut details = vec![];
    let mut taxes = vec![];
    let mut tax_totals = [0i64; 4];
    let mut credit_totals = [0i64; 3];
    let mut gross = 0i128;
    for selection in returns {
        if !seen.insert(selection.line) {
            return Err(invalid("Combine duplicate return lines."));
        }
        let l = orig
            .input
            .lines
            .get(selection.line)
            .ok_or_else(|| invalid("Unknown original invoice line."))?;
        let whole = crate::parse_quantity(&l.quantity)?;
        let qty = crate::parse_quantity(&selection.quantity)?;
        let prior = *used.get(&selection.line).unwrap_or(&0);
        if qty > whole - prior {
            return Err(invalid(
                "Return quantity exceeds the remaining invoiced quantity.",
            ));
        }
        let base = if let Some(calc) = &orig.gst {
            minor(&calc.lines[selection.line].taxable_minor)?
        } else {
            i64::try_from(
                (i128::from(whole) * i128::from(ledger_core::parse_amount(&l.unit_price)?) + 500)
                    / 1000,
            )
            .map_err(|_| invalid("Amount exceeds range."))?
        };
        let credit = proportional(base, prior, qty, whole, true)?;
        gross += i128::from(credit);
        let mut values = [credit, 0, 0, 0];
        let mut eligible = [0i64; 3];
        let mut nonrecoverable = 0i64;
        if let Some(calc) = &orig.gst {
            let g = &calc.lines[selection.line];
            let original_credit = g.credit_values()?;
            for (i, n) in [&g.cgst_minor, &g.sgst_minor, &g.igst_minor]
                .into_iter()
                .enumerate()
            {
                let full = minor(n)?;
                let previous = proportional(full, 0, prior, whole, true)?;
                values[i + 1] = proportional(full, prior, qty, whole, true)?;
                gross += i128::from(values[i + 1]);
                // Allocate credit by cumulative returned TAX units, not independently
                // by quantity: a component cannot refund more credit than its tax.
                eligible[i] = if full == 0 {
                    0
                } else {
                    proportional(original_credit[i], previous, values[i + 1], full, false)?
                };
                let blocked = values[i + 1]
                    .checked_sub(eligible[i])
                    .filter(|n| *n >= 0)
                    .ok_or_else(|| invalid("Return credit exceeds tax."))?;
                nonrecoverable = nonrecoverable
                    .checked_add(blocked)
                    .ok_or_else(|| invalid("Return cost exceeds range."))?;
                credit_totals[i] = credit_totals[i]
                    .checked_add(eligible[i])
                    .ok_or_else(|| invalid("Return credit exceeds range."))?;
            }
        }
        let cost_credit = if sale {
            credit
        } else {
            credit
                .checked_add(nonrecoverable)
                .ok_or_else(|| invalid("Return cost exceeds range."))?
        };
        let mut cost = 0;
        if let Some(item) = l.item_id {
            let latest: Option<String> = tx.query_row(
                "SELECT MAX(d.date) FROM stock_moves m JOIN documents d ON d.id=m.document_id",
                [],
                |r| r.get(0),
            )?;
            if latest.is_some_and(|date| d.date < date) {
                return Err(invalid(
                    "Stock returns must follow the latest stock movement date.",
                ));
            }
            let (asset, expense): (i64, i64) = tx.query_row(
                "SELECT asset_account_id,expense_account_id FROM items WHERE id=?1",
                [item],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let (available, carrying) = stock_totals(tx, item)?;
            if sale {
                let issue: i64 = tx.query_row(
                    "SELECT -value_minor FROM stock_moves WHERE document_id=?1 AND item_id=?2",
                    params![invoice, item],
                    |r| r.get(0),
                )?;
                cost = proportional(issue, prior, qty, whole, false)?;
                available
                    .checked_add(qty)
                    .ok_or_else(|| invalid("Stock quantity exceeds range."))?;
                carrying
                    .checked_add(cost)
                    .ok_or_else(|| invalid("Stock value exceeds range."))?;
                push(&mut prepared.lines, asset, Side::Debit, cost);
                push(&mut prepared.lines, expense, Side::Credit, cost);
                prepared.moves.push((item, qty, cost));
                push(&mut prepared.lines, l.account_id, Side::Debit, credit);
            } else {
                if qty > available {
                    return Err(invalid("Insufficient stock for supplier return."));
                }
                cost = if qty == available {
                    carrying
                } else {
                    i64::try_from(
                        (i128::from(carrying) * i128::from(qty) + i128::from(available) / 2)
                            / i128::from(available),
                    )
                    .map_err(|_| invalid("Stock value exceeds range."))?
                };
                push(&mut prepared.lines, asset, Side::Credit, cost);
                let difference = i128::from(cost) - i128::from(cost_credit);
                let variance = i64::try_from(difference.abs())
                    .map_err(|_| invalid("Return variance exceeds range."))?;
                push(
                    &mut prepared.lines,
                    expense,
                    if difference > 0 {
                        Side::Debit
                    } else {
                        Side::Credit
                    },
                    variance,
                );
                prepared.moves.push((item, -qty, -cost));
            }
        } else {
            push(
                &mut prepared.lines,
                l.account_id,
                if sale { Side::Debit } else { Side::Credit },
                cost_credit,
            );
        }
        if let Some(calc) = &orig.gst {
            let g = &calc.lines[selection.line];
            for i in 0..4 {
                tax_totals[i] = tax_totals[i]
                    .checked_add(values[i])
                    .ok_or_else(|| invalid("Return tax exceeds range."))?;
            }
            taxes.push(GstLine {
                input_credit: g
                    .input_credit
                    .as_ref()
                    .map(|_| crate::InputCredit::from_values(eligible)),
                taxable_minor: credit.to_string(),
                rate_bps: g.rate_bps,
                hsn_sac: g.hsn_sac.clone(),
                cgst_minor: values[1].to_string(),
                sgst_minor: values[2].to_string(),
                igst_minor: values[3].to_string(),
            });
        }
        details.push(ReturnDetail {
            original_line: selection.line,
            description: l.description.clone(),
            quantity: selection.quantity.clone(),
            unit_price: l.unit_price.clone(),
            item_id: l.item_id,
            account_id: l.account_id,
            taxable_minor: credit.to_string(),
            inventory_cost_minor: cost.to_string(),
        });
    }
    if orig.gst.is_some() {
        let raw: String =
            tx.query_row("SELECT data FROM gst_profile WHERE id=1", [], |r| r.get(0))?;
        let profile: GstProfile = serde_json::from_str(&raw)?;
        let accounts = if sale { profile.output } else { profile.input };
        for (id, n) in [
            (
                accounts.cgst,
                if sale {
                    tax_totals[1]
                } else {
                    credit_totals[0]
                },
            ),
            (
                accounts.sgst,
                if sale {
                    tax_totals[2]
                } else {
                    credit_totals[1]
                },
            ),
            (
                accounts.igst,
                if sale {
                    tax_totals[3]
                } else {
                    credit_totals[2]
                },
            ),
        ] {
            push(
                &mut prepared.lines,
                id,
                if sale { Side::Debit } else { Side::Credit },
                n,
            );
        }
        prepared.gst = Some(GstCalculation {
            taxable_minor: tax_totals[0].to_string(),
            cgst_minor: tax_totals[1].to_string(),
            sgst_minor: tax_totals[2].to_string(),
            igst_minor: tax_totals[3].to_string(),
            lines: taxes,
        });
    }
    prepared.total = i64::try_from(gross).map_err(|_| invalid("Return total exceeds range."))?;
    if prepared.total <= 0 {
        return Err(invalid(
            "Return rounds to zero. Combine quantities into a larger return.",
        ));
    }
    let control: i64 = tx.query_row(
        "SELECT account_id FROM parties WHERE id=?1",
        [d.party_id],
        |r| r.get(0),
    )?;
    push(
        &mut prepared.lines,
        control,
        if sale { Side::Credit } else { Side::Debit },
        prepared.total,
    );
    prepared.correction = Some(Correction {
        invoice_number: orig.number,
        invoice_date: orig.input.date,
        original_gst: orig.input.gst,
        lines: details,
    });
    Ok(prepared)
}
impl LedgerStore {
    pub fn returnable_invoice(&self, id: i64) -> Result<serde_json::Value> {
        let _read = self.conn.unchecked_transaction()?;
        let d = original(&self.conn, id)?;
        if !["sale", "purchase"].contains(&d.input.kind.as_str()) {
            return Err(invalid("Select a sales or purchase invoice."));
        }
        let used = quantities(&self.conn, id)?;
        let remaining = d
            .input
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                Ok(
                    (crate::parse_quantity(&l.quantity)? - used.get(&i).copied().unwrap_or(0))
                        .to_string(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(serde_json::json!({"invoice":d,"remaining_milli":remaining}))
    }
    pub fn preview_document(&self, input: DocumentInput) -> Result<serde_json::Value> {
        let (mut stage, _) = Self::prepare_restore(&self.export_backup()?)?;
        let d = stage.post_document(input)?;
        let journal = stage.journal(d.journal_id)?;
        Ok(serde_json::json!({"document":d,"journal":journal}))
    }
}
