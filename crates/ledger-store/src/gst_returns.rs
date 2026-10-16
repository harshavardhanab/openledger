//! Return preparation from immutable books; never a portal submission or an ITC entitlement decision.
use crate::{Document, GstCalculation, LedgerStore, Result, store::invalid};
use chrono::{Datelike, NaiveDate};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GstReturnQuery {
    /// Calendar month, or quarter-ending month for quarterly preparation.
    pub period: String,
    pub frequency: String,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReturnAmounts {
    pub taxable_minor: String,
    pub cgst_minor: String,
    pub sgst_minor: String,
    pub igst_minor: String,
}
fn amounts(n: [i128; 4]) -> ReturnAmounts {
    ReturnAmounts {
        taxable_minor: n[0].to_string(),
        cgst_minor: n[1].to_string(),
        sgst_minor: n[2].to_string(),
        igst_minor: n[3].to_string(),
    }
}
fn values(g: &GstCalculation) -> Result<[i128; 4]> {
    let mut n = [0; 4];
    for (i, s) in [
        &g.taxable_minor,
        &g.cgst_minor,
        &g.sgst_minor,
        &g.igst_minor,
    ]
    .into_iter()
    .enumerate()
    {
        n[i] = s
            .parse()
            .map_err(|_| invalid("Invalid stored GST amount."))?;
    }
    Ok(n)
}
fn add(a: &mut [i128; 4], b: [i128; 4], sign: i128) {
    for i in 0..4 {
        a[i] += b[i] * sign;
    }
}
#[derive(Debug, Serialize)]
pub struct ReturnIssue {
    pub code: String,
    pub document_id: Option<i64>,
    pub reference: String,
    pub message: String,
}
fn issue(code: &str, d: &Document, message: &str) -> ReturnIssue {
    ReturnIssue {
        code: code.into(),
        document_id: Some(d.id),
        reference: d.number.clone(),
        message: message.into(),
    }
}
#[derive(Debug, Serialize)]
pub struct ReturnRow {
    pub document_id: i64,
    pub number: String,
    pub date: String,
    pub original_invoice: Option<String>,
    pub section: String,
    pub classification: String,
    pub classification_basis: Option<String>,
    pub gstin: Option<String>,
    pub supply_state: String,
    pub rate_bps: u32,
    pub sign: i32,
    pub amounts: ReturnAmounts,
}
#[derive(Debug, Serialize)]
pub struct HsnReview {
    pub recipient: String,
    pub hsn_sac: String,
    pub rate_bps: u32,
    pub quantity_milli: String,
    pub unit: String,
    pub amounts: ReturnAmounts,
}
#[derive(Debug, Serialize)]
pub struct GstReconciliation {
    pub direction: String,
    pub component: String,
    pub account_id: i64,
    pub book_activity_minor: String,
    pub register_activity_minor: String,
    pub difference_minor: String,
}
#[derive(Debug, Serialize)]
pub struct ReturnFile {
    pub section: String,
    pub filename: String,
    pub row_count: usize,
    pub contents: String,
}
#[derive(Debug, Serialize)]
pub struct InwardNoTax {
    pub category: String,
    pub inter_state_minor: String,
    pub intra_state_minor: String,
}
#[derive(Debug, Serialize)]
pub struct GstReturnDraft {
    pub format: &'static str,
    pub version: u32,
    pub query: GstReturnQuery,
    pub from: String,
    pub to: String,
    pub gstin: String,
    pub filing_ready: bool,
    pub portal_verified: bool,
    pub issues: Vec<ReturnIssue>,
    pub gstr1_rows: Vec<ReturnRow>,
    pub gstr1_sections: BTreeMap<String, ReturnAmounts>,
    pub hsn_review: Vec<HsnReview>,
    pub gstr3b_book_3_1_a: ReturnAmounts,
    pub gstr3b_book_3_1_c: ReturnAmounts,
    pub gstr3b_book_3_1_e: ReturnAmounts,
    pub gstr3b_book_4_a_5: ReturnAmounts,
    pub gstr3b_book_5: Vec<InwardNoTax>,
    pub gstr3b_book_3_2_unregistered: BTreeMap<String, ReturnAmounts>,
    pub reconciliation: Vec<GstReconciliation>,
    pub reconciled: bool,
    pub files: Vec<ReturnFile>,
    pub review_requirements: Vec<&'static str>,
}
fn dates(q: &GstReturnQuery) -> Result<(String, String)> {
    if q.period.len() != 7 || !q.period.is_ascii() {
        return Err(invalid("Return period must use YYYY-MM."));
    }
    let end = NaiveDate::parse_from_str(&format!("{}-01", q.period), "%Y-%m-%d")
        .map_err(|_| invalid("Invalid GST return month."))?;
    if end.format("%Y-%m").to_string() != q.period || q.period.as_str() < "2017-07" {
        return Err(invalid("GST return preparation begins in July 2017."));
    }
    let start = match q.frequency.as_str() {
        "monthly" => end,
        "quarterly" if end.month().is_multiple_of(3) => end.with_month(end.month() - 2).unwrap(),
        _ => {
            return Err(invalid(
                "Choose monthly or a March/June/September/December quarter-end.",
            ));
        }
    };
    let last = end
        .checked_add_months(chrono::Months::new(1))
        .and_then(|d| d.pred_opt())
        .ok_or_else(|| invalid("Return period exceeds date range."))?;
    Ok((start.to_string(), last.to_string()))
}
fn decimal(n: i128) -> String {
    let a = n.abs();
    format!("{}{}.{:02}", if n < 0 { "-" } else { "" }, a / 100, a % 100)
}
fn csv(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|r| {
            r.iter()
                .map(|s| {
                    // Spreadsheet formula protection without altering numeric amounts.
                    let unsafe_text = s.starts_with(['=', '+', '@', '-'])
                        && s.parse::<i128>().is_err()
                        && !s
                            .strip_prefix('-')
                            .unwrap_or(s)
                            .replace('.', "")
                            .bytes()
                            .all(|b| b.is_ascii_digit());
                    format!(
                        "\"{}{}\"",
                        if unsafe_text { "'" } else { "" },
                        s.replace('"', "\"\"")
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
fn rate_rounding_differs(n: [i128; 4], rate: u32, local: bool) -> bool {
    let sign = if n[0] < 0 { -1 } else { 1 };
    let numerator = n[0].abs() * i128::from(rate);
    let expected = if local {
        let half = (numerator + 10_000) / 20_000 * sign;
        [half, half, 0]
    } else {
        [0, 0, (numerator + 5_000) / 10_000 * sign]
    };
    n[1..] != expected
}
fn date_export(date: &str) -> Result<String> {
    Ok(NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| invalid("Invalid stored invoice date."))?
        .format("%d-%b-%y")
        .to_string())
}
const HEADERS: [(&str, &str); 6] = [
    (
        "exemp",
        "Description,Nil Rated Supplies,Exempted(other than nil rated/non GST supply),Non-GST Supplies",
    ),
    (
        "b2b",
        "GSTIN/UIN of Recipient,Receiver Name,Invoice Number,Invoice date,Invoice Value,Place Of Supply,Reverse Charge,Applicable % of Tax Rate,Invoice Type,E-Commerce GSTIN,Rate,Taxable Value,Cess Amount",
    ),
    (
        "b2cl",
        "Invoice Number,Invoice date,Invoice Value,Place Of Supply,Applicable % of Tax Rate,Rate,Taxable Value,Cess Amount,E-Commerce GSTIN",
    ),
    (
        "b2cs",
        "Type,Place Of Supply,Rate,Applicable % of Tax Rate,Taxable Value,Cess Amount,E-Commerce GSTIN",
    ),
    (
        "cdnr",
        "GSTIN/UIN of Recipient,Receiver Name,Note Number,Note Date,Note Type,Place Of Supply,Reverse Charge,Note Supply Type,Note Value,Applicable % of Tax Rate,Rate,Taxable Value,Cess Amount",
    ),
    (
        "cdnur",
        "UR Type,Note Number,Note Date,Note Type,Place Of Supply,Note Value,Applicable % of Tax Rate,Rate,Taxable Value,Cess Amount",
    ),
];
type HsnGroups = BTreeMap<(String, String, u32, String), (i128, [i128; 4])>;
fn accumulate_hsn(
    store: &LedgerStore,
    d: &Document,
    calc: &GstCalculation,
    registered: bool,
    hsn: &mut HsnGroups,
) -> Result<()> {
    let sign = if d.correction.is_some() { -1 } else { 1 };
    for (index, l) in calc.lines.iter().enumerate() {
        let n = [
            l.taxable_minor
                .parse()
                .map_err(|_| invalid("Invalid GST line."))?,
            l.cgst_minor
                .parse()
                .map_err(|_| invalid("Invalid GST line."))?,
            l.sgst_minor
                .parse()
                .map_err(|_| invalid("Invalid GST line."))?,
            l.igst_minor
                .parse()
                .map_err(|_| invalid("Invalid GST line."))?,
        ];
        let (qty, unit) = if let Some(c) = &d.correction {
            let l = &c.lines[index];
            (crate::parse_quantity(&l.quantity)?, l.item_id)
        } else {
            let l = &d.input.lines[index];
            (crate::parse_quantity(&l.quantity)?, l.item_id)
        };
        let unit = if let Some(id) = unit {
            store
                .conn
                .query_row("SELECT unit FROM items WHERE id=?1", [id], |r| {
                    r.get::<_, String>(0)
                })?
        } else {
            "service quantity (UQC not mapped)".into()
        };
        let h = hsn
            .entry((
                if registered { "B2B" } else { "B2C" }.into(),
                l.hsn_sac.clone(),
                l.rate_bps,
                unit,
            ))
            .or_default();
        h.0 += i128::from(qty) * sign;
        add(&mut h.1, n, sign);
    }
    Ok(())
}
impl LedgerStore {
    pub fn gst_return_draft(&self, query: GstReturnQuery) -> Result<GstReturnDraft> {
        self.require_company()?;
        let (from, to) = dates(&query)?;
        let _read = self.conn.unchecked_transaction()?;
        let p = self
            .gst_profile()?
            .ok_or_else(|| invalid("Configure a GST registration first."))?;
        let register = self.gst_report(&from, &to)?;
        let mut q = self.conn.prepare("SELECT d.data,(SELECT r.date FROM journals r WHERE r.reversal_of=d.journal_id) FROM documents d WHERE d.kind IN ('sale','purchase','credit_note','debit_note') AND d.date<=?1 ORDER BY d.date,d.id")?;
        let docs = q
            .query_map([&to], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?
            .map(|r| {
                let (s, rev) = r?;
                Ok((serde_json::from_str::<Document>(&s)?, rev))
            })
            .collect::<Result<Vec<_>>>()?;
        let originals = docs
            .iter()
            .map(|(d, _)| (d.id, d))
            .collect::<BTreeMap<_, _>>();
        let mut out = GstReturnDraft {
            format: "open-ledger-gst-return-draft",
            version: 1,
            query,
            from: from.clone(),
            to: to.clone(),
            gstin: p.gstin.clone(),
            filing_ready: false,
            portal_verified: false,
            issues: vec![],
            gstr1_rows: vec![],
            gstr1_sections: BTreeMap::new(),
            hsn_review: vec![],
            gstr3b_book_3_1_a: amounts([0; 4]),
            gstr3b_book_3_1_c: amounts([0; 4]),
            gstr3b_book_3_1_e: amounts([0; 4]),
            gstr3b_book_4_a_5: amounts([0; 4]),
            gstr3b_book_5: vec![],
            gstr3b_book_3_2_unregistered: BTreeMap::new(),
            reconciliation: vec![],
            reconciled: true,
            files: vec![],
            review_requirements: vec![
                "Draft only: compare every applicable table with the current GST Portal before filing.",
                "Review HSN B2B/B2C summaries, statutory UQC mappings and document-series totals separately.",
                "Book purchase ITC is not GSTR-2B/IMS entitlement: reconcile availability, reversals and eligibility independently.",
                "Review e-invoice, amendments, earlier-period adjustments, other supplies, interest/late fees and payments outside this draft.",
                "CSV layouts follow the official offline templates downloaded 2026-10-16; portal acceptance is unverified.",
            ],
        };
        let mut files = HEADERS
            .into_iter()
            .map(|(key, h)| {
                (
                    key.to_owned(),
                    vec![h.split(',').map(str::to_owned).collect::<Vec<_>>()],
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut sections = BTreeMap::<String, [i128; 4]>::new();
        let mut small = BTreeMap::<(String, u32), (String, [i128; 4])>::new();
        let mut hsn = BTreeMap::<(String, String, u32, String), (i128, [i128; 4])>::new();
        let mut interstate = BTreeMap::<String, [i128; 4]>::new();
        let mut exempt = BTreeMap::<String, [i128; 3]>::new();
        let mut no_tax_book = [0i128; 3];
        let mut no_tax_inward = [[0i128; 2]; 2];
        for (d, reversed) in &docs {
            let in_period = d.input.date >= from && d.input.date <= to;
            let reversal_in_period = reversed
                .as_ref()
                .is_some_and(|date| *date >= from && *date <= to);
            if !in_period && !reversal_in_period {
                continue;
            }
            let original = if let Some(id) = d.input.invoice_id {
                originals.get(&id).copied().unwrap_or(d)
            } else {
                d
            };
            let outward = original.input.kind == "sale";
            let (Some(g), Some(calc)) = (&original.input.gst, &d.gst) else {
                out.issues.push(issue("unclassified_document", d, "No GST classification is recorded. Determine exempt, nil-rated, non-GST or taxable treatment before completing the return."));
                continue;
            };
            if !outward && let Some(class) = &g.classification {
                let index = usize::from(class.kind == "non_gst");
                let locality = usize::from(class.supplier_state.as_ref() == Some(&p.state_code));
                let value = values(calc)?[0];
                if in_period {
                    no_tax_inward[index][locality] +=
                        value * if d.correction.is_some() { -1 } else { 1 };
                }
                if reversal_in_period {
                    no_tax_inward[index][locality] -= value;
                }
            }
            if outward && let Some(class) = &g.classification {
                let index = match class.kind.as_str() {
                    "nil_rated" => 0,
                    "exempt" => 1,
                    "non_gst" => 2,
                    _ => return Err(invalid("Invalid stored supply classification.")),
                };
                let value = values(calc)?[0];
                if in_period {
                    no_tax_book[index] += value * if d.correction.is_some() { -1 } else { 1 };
                }
                if reversal_in_period {
                    no_tax_book[index] -= value;
                }
            }
            if outward
                && g.classification.is_none()
                && g.counterparty_gstin.is_none()
                && g.supply_state != p.state_code
            {
                let sum = interstate.entry(g.supply_state.clone()).or_default();
                if in_period {
                    add(
                        sum,
                        values(calc)?,
                        if d.correction.is_some() { -1 } else { 1 },
                    );
                }
                if reversal_in_period {
                    add(sum, values(calc)?, -1);
                }
            }
            if reversed.as_ref().is_some_and(|date| *date <= to) {
                out.issues.push(issue("accounting_reversal", d, "A journal reversal is not a statutory note or amendment. This document is excluded from GSTR-1 CSV candidates; review both periods and report its legal correction separately."));
                continue;
            }
            if !in_period || !outward {
                continue;
            }
            let registered = g.counterparty_gstin.is_some();
            accumulate_hsn(self, d, calc, registered, &mut hsn)?;
            if let Some(class) = &g.classification {
                let sign = if d.correction.is_some() { -1 } else { 1 };
                let label = format!(
                    "{}-State supplies to {} persons",
                    if g.supply_state == p.state_code {
                        "Intra"
                    } else {
                        "Inter"
                    },
                    if registered {
                        "registered"
                    } else {
                        "unregistered"
                    }
                );
                let index = match class.kind.as_str() {
                    "nil_rated" => 0,
                    "exempt" => 1,
                    "non_gst" => 2,
                    _ => return Err(invalid("Invalid stored supply classification.")),
                };
                let n = values(calc)?;
                exempt.entry(label).or_default()[index] += n[0] * sign;
                add(sections.entry("exemp".into()).or_default(), n, sign);
                out.gstr1_rows.push(ReturnRow {
                    document_id: d.id,
                    number: d.number.clone(),
                    date: d.input.date.clone(),
                    original_invoice: d.correction.as_ref().map(|_| original.number.clone()),
                    section: "exemp".into(),
                    classification: class.kind.clone(),
                    classification_basis: Some(class.reason.clone()),
                    gstin: g.counterparty_gstin.clone(),
                    supply_state: g.supply_state.clone(),
                    rate_bps: 0,
                    sign: sign as i32,
                    amounts: amounts(n.map(|v| v * sign)),
                });
                continue;
            }
            // Classification follows ORIGINAL gross invoice value/date, never a reduced note value.
            let threshold = if original.input.date.as_str() >= "2024-08-01" {
                10_000_000i128
            } else {
                25_000_000
            };
            let large = !registered
                && g.supply_state != p.state_code
                && original
                    .total_minor
                    .parse::<i128>()
                    .map_err(|_| invalid("Invalid invoice value."))?
                    > threshold;
            let note = d.correction.is_some();
            let section = if note && registered {
                "cdnr"
            } else if note && large {
                "cdnur"
            } else if registered {
                "b2b"
            } else if large {
                "b2cl"
            } else {
                "b2cs"
            };
            let sign = if note { -1 } else { 1 };
            let pos = format!("{}-{}", g.supply_state, g.supply_state_name);
            let mut rates = BTreeMap::<u32, [i128; 4]>::new();
            for l in &calc.lines {
                let n = [
                    l.taxable_minor
                        .parse()
                        .map_err(|_| invalid("Invalid GST line."))?,
                    l.cgst_minor
                        .parse()
                        .map_err(|_| invalid("Invalid GST line."))?,
                    l.sgst_minor
                        .parse()
                        .map_err(|_| invalid("Invalid GST line."))?,
                    l.igst_minor
                        .parse()
                        .map_err(|_| invalid("Invalid GST line."))?,
                ];
                add(rates.entry(l.rate_bps).or_default(), n, 1);
            }
            let name: String = self.conn.query_row(
                "SELECT name FROM parties WHERE id=?1",
                [d.input.party_id],
                |r| r.get(0),
            )?;
            for (rate, n) in rates {
                add(sections.entry(section.into()).or_default(), n, sign);
                out.gstr1_rows.push(ReturnRow {
                    document_id: d.id,
                    number: d.number.clone(),
                    date: d.input.date.clone(),
                    original_invoice: note.then(|| original.number.clone()),
                    section: section.into(),
                    classification: "regular".into(),
                    classification_basis: None,
                    gstin: g.counterparty_gstin.clone(),
                    supply_state: g.supply_state.clone(),
                    rate_bps: rate,
                    sign: sign as i32,
                    amounts: amounts(n.map(|v| v * sign)),
                });
                if section == "b2cs" {
                    let s = small
                        .entry((g.supply_state.clone(), rate))
                        .or_insert_with(|| (pos.clone(), [0; 4]));
                    add(&mut s.1, n, sign);
                    continue;
                }
                if rate_rounding_differs(n, rate, g.supply_state == p.state_code) {
                    out.issues.push(issue("offline_rounding_difference", d, "Tax recorded on this rate group differs from tax recalculated on its aggregated taxable value. Reconcile the offline tool's calculated tax with the exact book amounts."));
                }
                let date = date_export(&d.input.date)?;
                let gross = decimal(
                    d.total_minor
                        .parse()
                        .map_err(|_| invalid("Invalid invoice total."))?,
                );
                let taxable = decimal(n[0]);
                let rate_text = decimal(i128::from(rate));
                let gstin = g.counterparty_gstin.clone().unwrap_or_default();
                let row = match section {
                    "b2b" => vec![
                        gstin,
                        name.clone(),
                        d.number.clone(),
                        date,
                        gross,
                        pos.clone(),
                        "N".into(),
                        "".into(),
                        "Regular B2B".into(),
                        "".into(),
                        rate_text,
                        taxable,
                        "0.00".into(),
                    ],
                    "b2cl" => vec![
                        d.number.clone(),
                        date,
                        gross,
                        pos.clone(),
                        "".into(),
                        rate_text,
                        taxable,
                        "0.00".into(),
                        "".into(),
                    ],
                    "cdnr" => vec![
                        gstin,
                        name.clone(),
                        d.number.clone(),
                        date,
                        "C".into(),
                        pos.clone(),
                        "N".into(),
                        "Regular B2B".into(),
                        gross,
                        "".into(),
                        rate_text,
                        taxable,
                        "0.00".into(),
                    ],
                    "cdnur" => vec![
                        "B2CL".into(),
                        d.number.clone(),
                        date,
                        "C".into(),
                        pos.clone(),
                        gross,
                        "".into(),
                        rate_text,
                        taxable,
                        "0.00".into(),
                    ],
                    _ => unreachable!(),
                };
                files.get_mut(section).unwrap().push(row);
            }
        }
        for ((state, rate), (pos, n)) in small {
            if rate_rounding_differs(n, rate, state == p.state_code) {
                out.issues.push(ReturnIssue { code:"offline_rounding_difference".into(),document_id:None,reference:format!("B2CS {pos} at {}%",decimal(i128::from(rate))),message:"Tax on aggregated consumer supplies differs from exact book tax. Reconcile offline-tool rounding before filing.".into() });
            }
            files.get_mut("b2cs").unwrap().push(vec![
                "OE".into(),
                pos,
                decimal(i128::from(rate)),
                "".into(),
                decimal(n[0]),
                "0.00".into(),
                "".into(),
            ]);
        }
        for (label, n) in exempt {
            files.get_mut("exemp").unwrap().push(vec![
                label,
                decimal(n[0]),
                decimal(n[1]),
                decimal(n[2]),
            ]);
        }
        out.gstr3b_book_5 = no_tax_inward
            .into_iter()
            .enumerate()
            .map(|(i, n)| InwardNoTax {
                category: if i == 0 {
                    "Nil-rated / exempt (composition excluded)"
                } else {
                    "Non-GST"
                }
                .into(),
                inter_state_minor: n[0].to_string(),
                intra_state_minor: n[1].to_string(),
            })
            .collect();
        out.gstr3b_book_3_1_c = amounts([no_tax_book[0] + no_tax_book[1], 0, 0, 0]);
        out.gstr3b_book_3_1_e = amounts([no_tax_book[2], 0, 0, 0]);
        out.gstr1_sections = sections.into_iter().map(|(s, n)| (s, amounts(n))).collect();
        out.hsn_review = hsn
            .into_iter()
            .map(|((recipient, hsn_sac, rate_bps, unit), (q, n))| HsnReview {
                recipient,
                hsn_sac,
                rate_bps,
                unit,
                quantity_milli: q.to_string(),
                amounts: amounts(n),
            })
            .collect();
        out.gstr3b_book_3_2_unregistered = interstate
            .into_iter()
            .map(|(s, n)| (s, amounts(n)))
            .collect();
        for (kind, accounts, direction) in
            [("sale", p.output, "output"), ("purchase", p.input, "input")]
        {
            let totals = &register["totals"][kind];
            let mut expected = [0i128; 4];
            for (i, key) in ["taxable_minor", "cgst_minor", "sgst_minor", "igst_minor"]
                .into_iter()
                .enumerate()
            {
                expected[i] = totals[key]
                    .as_str()
                    .unwrap_or("0")
                    .parse()
                    .map_err(|_| invalid("Invalid GST register."))?;
            }
            if kind == "sale" {
                let mut regular = expected;
                regular[0] -= no_tax_book.iter().sum::<i128>();
                out.gstr3b_book_3_1_a = amounts(regular);
            } else {
                out.gstr3b_book_4_a_5 = amounts(expected);
            }
            for (i, (component, id)) in [
                ("CGST", accounts.cgst),
                ("SGST / UTGST", accounts.sgst),
                ("IGST", accounts.igst),
            ]
            .into_iter()
            .enumerate()
            {
                let mut q = self.conn.prepare("SELECT l.side,l.amount_minor FROM lines l JOIN journals j ON j.id=l.journal_id WHERE l.account_id=?1 AND j.date>=?2 AND j.date<=?3 AND j.status='posted'")?;
                let mut activity = 0i128;
                for r in q.query_map(params![id, from, to], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                })? {
                    let (side, n) = r?;
                    activity += i128::from(n)
                        * if (side == "debit") == (direction == "input") {
                            1
                        } else {
                            -1
                        };
                }
                let difference = activity - expected[i + 1];
                out.reconciled &= difference == 0;
                out.reconciliation.push(GstReconciliation {
                    direction: direction.into(),
                    component: component.into(),
                    account_id: id,
                    book_activity_minor: activity.to_string(),
                    register_activity_minor: expected[i + 1].to_string(),
                    difference_minor: difference.to_string(),
                });
                // Net-zero manual entries still need review, even when reconciliation balances.
                let mut q = self.conn.prepare("SELECT DISTINCT j.id,j.reference FROM journals j JOIN lines l ON l.journal_id=j.id WHERE l.account_id=?1 AND j.date>=?2 AND j.date<=?3 AND NOT EXISTS(SELECT 1 FROM documents d WHERE d.journal_id=COALESCE(j.reversal_of,j.id) AND json_extract(d.data,'$.gst') IS NOT NULL)")?;
                for r in q.query_map(params![id, from, to], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
                })? {
                    let (journal, reference) = r?;
                    let code = format!("manual_gst_journal_{journal}");
                    if !out.issues.iter().any(|i| i.code == code) {
                        out.issues.push(ReturnIssue {code,document_id:None,reference,message:"A manual GST-control posting has no classified invoice or note. Reconcile its return treatment independently.".into()});
                    }
                }
            }
        }
        let mut manual = self.conn.prepare("SELECT DISTINCT j.id,j.reference FROM journals j JOIN lines l ON l.journal_id=j.id JOIN accounts a ON a.id=l.account_id WHERE j.source='manual' AND a.kind IN ('income','expense') AND j.date>=?1 AND j.date<=?2 AND NOT EXISTS(SELECT 1 FROM documents d WHERE d.journal_id=COALESCE(j.reversal_of,j.id))")?;
        for r in manual.query_map(params![from, to], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (journal, reference) = r?;
            if !out
                .issues
                .iter()
                .any(|i| i.code == format!("manual_gst_journal_{journal}"))
            {
                out.issues.push(ReturnIssue {code:format!("unclassified_journal_{journal}"),document_id:None,reference,message:"A manual income/expense journal has no supply classification. Check whether any part belongs in this return.".into()});
            }
        }
        if !out.reconciled {
            out.issues.push(ReturnIssue {code:"control_account_difference".into(),document_id:None,reference:"GST controls".into(),message:"GST register and control-account activity differ. Resolve the differences before preparing a return.".into()});
        }
        for (section, rows) in files {
            let width = rows[0].len();
            if rows.iter().any(|r| r.len() != width) {
                return Err(invalid("Invalid GST CSV layout."));
            }
            out.files.push(ReturnFile {
                filename: format!("DRAFT-gstr1-{}-{section}.csv", out.query.period),
                section,
                row_count: rows.len() - 1,
                contents: csv(&rows),
            });
        }
        let mut summary = vec![
            vec![
                "Table",
                "Description",
                "Place Of Supply",
                "Taxable Value",
                "Integrated Tax",
                "Central Tax",
                "State/UT Tax",
                "Cess",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        ];
        for (table, description, pos, n) in std::iter::once((
            "3.1(a)",
            "Book outward taxable supplies",
            "",
            &out.gstr3b_book_3_1_a,
        ))
        .chain(std::iter::once((
            "3.1(c)",
            "Book nil-rated/exempt outward supplies",
            "",
            &out.gstr3b_book_3_1_c,
        )))
        .chain(std::iter::once((
            "3.1(e)",
            "Book non-GST outward supplies",
            "",
            &out.gstr3b_book_3_1_e,
        )))
        .chain(std::iter::once((
            "4(A)(5)",
            "Book reviewed input credit; reconcile GSTR-2B/IMS",
            "",
            &out.gstr3b_book_4_a_5,
        )))
        .chain(out.gstr3b_book_3_2_unregistered.iter().map(|(pos, n)| {
            (
                "3.2",
                "Book inter-state unregistered supplies",
                pos.as_str(),
                n,
            )
        })) {
            let parse = |s: &str| -> Result<String> {
                Ok(decimal(
                    s.parse().map_err(|_| invalid("Invalid draft amount."))?,
                ))
            };
            summary.push(vec![
                table.into(),
                description.into(),
                pos.into(),
                if table == "4(A)(5)" {
                    "".into()
                } else {
                    parse(&n.taxable_minor)?
                },
                parse(&n.igst_minor)?,
                parse(&n.cgst_minor)?,
                parse(&n.sgst_minor)?,
                "0.00".into(),
            ]);
        }
        for row in &out.gstr3b_book_5 {
            for (locality, value) in [
                ("Inter-State", &row.inter_state_minor),
                ("Intra-State", &row.intra_state_minor),
            ] {
                summary.push(vec![
                    "5".into(),
                    row.category.clone(),
                    locality.into(),
                    decimal(
                        value
                            .parse()
                            .map_err(|_| invalid("Invalid inward supply value."))?,
                    ),
                    "".into(),
                    "".into(),
                    "".into(),
                    "".into(),
                ]);
            }
        }
        out.files.push(ReturnFile {
            section: "gstr3b-book-summary".into(),
            filename: format!("DRAFT-gstr3b-{}-book-summary.csv", out.query.period),
            row_count: summary.len() - 1,
            contents: csv(&summary),
        });
        Ok(out)
    }
}
