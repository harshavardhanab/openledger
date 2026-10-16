//! Scoped domestic regular GST accounting. Rates and legal eligibility are user-reviewed inputs.
use crate::{DocumentInput, LedgerStore, Result, store::invalid};
use ledger_core::{clean_text, parse_amount};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GstAccounts {
    pub cgst: i64,
    pub sgst: i64,
    pub igst: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GstProfile {
    pub gstin: String,
    pub state_code: String,
    pub state_name: String,
    pub address: String,
    pub output: GstAccounts,
    pub input: GstAccounts,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SupplyClassification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplier_state: Option<String>,
    pub kind: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItcAllocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_treatment: Option<String>,
    pub eligible_bps: Vec<u32>,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InputCredit {
    pub cgst_minor: String,
    pub sgst_minor: String,
    pub igst_minor: String,
}
impl InputCredit {
    pub(crate) fn values(&self) -> Result<[i64; 3]> {
        let parse = |n: &str| {
            n.parse()
                .map_err(|_| invalid("Invalid stored input credit."))
        };
        Ok([
            parse(&self.cgst_minor)?,
            parse(&self.sgst_minor)?,
            parse(&self.igst_minor)?,
        ])
    }
    pub(crate) fn from_values(n: [i64; 3]) -> Self {
        Self {
            cgst_minor: n[0].to_string(),
            sgst_minor: n[1].to_string(),
            igst_minor: n[2].to_string(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GstInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub itc_allocation: Option<ItcAllocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<SupplyClassification>,
    pub supply_state: String,
    pub supply_state_name: String,
    pub counterparty_gstin: Option<String>,
    pub counterparty_address: String,
    pub delivery_address: String,
    pub supplier_reference: Option<String>,
    pub credit_reviewed: bool,
    pub rates_bps: Vec<u32>,
    pub hsn_sac: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GstLine {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_credit: Option<InputCredit>,
    pub taxable_minor: String,
    pub rate_bps: u32,
    pub hsn_sac: String,
    pub cgst_minor: String,
    pub sgst_minor: String,
    pub igst_minor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GstCalculation {
    pub taxable_minor: String,
    pub cgst_minor: String,
    pub sgst_minor: String,
    pub igst_minor: String,
    pub lines: Vec<GstLine>,
}
fn valid_state(s: &str) -> bool {
    s.len() == 2
        && s.bytes().all(|b| b.is_ascii_digit())
        && s.parse::<u8>().is_ok_and(|n| (1..=38).contains(&n))
}
fn validate_gstin(s: &str) -> Result<()> {
    let b = s.as_bytes();
    if !s.is_ascii()
        || b.len() != 15
        || !valid_state(&s[..2])
        || !b[2..7].iter().all(u8::is_ascii_uppercase)
        || !b[7..11].iter().all(u8::is_ascii_digit)
        || !b[11].is_ascii_uppercase()
        || !b[12].is_ascii_alphanumeric()
        || b[13] != b'Z'
        || !b[14].is_ascii_alphanumeric()
        || b.iter().any(u8::is_ascii_lowercase)
    {
        return Err(invalid(
            "GSTIN must use the 15-character uppercase registration format. Verify registration independently.",
        ));
    }
    Ok(())
}
fn valid_invoice_number(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 16
        && n.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'/')
}
pub(crate) fn validate_gst_profile(tx: &Transaction<'_>, p: &GstProfile) -> Result<()> {
    let currency: String =
        tx.query_row("SELECT currency FROM company WHERE id=1", [], |r| r.get(0))?;
    if currency != "INR" {
        return Err(invalid("India GST accounting requires INR books."));
    }
    validate_gstin(&p.gstin)?;
    if !valid_state(&p.state_code) || !p.gstin.starts_with(&p.state_code) {
        return Err(invalid("Company state code must match GSTIN."));
    }
    clean_text(&p.state_name, "State name", 80, true)?;
    clean_text(&p.address, "Registered address", 500, true)?;
    let mut seen = std::collections::BTreeSet::new();
    for (accounts, kind) in [(&p.output, "liability"), (&p.input, "asset")] {
        for id in [accounts.cgst, accounts.sgst, accounts.igst] {
            if !seen.insert(id) {
                return Err(invalid("Use six distinct GST control accounts."));
            }
            let actual: Option<String> = tx
                .query_row("SELECT kind FROM accounts WHERE id=?1", [id], |r| r.get(0))
                .optional()?;
            let mapped:i64=tx.query_row("SELECT (SELECT COUNT(*) FROM parties WHERE account_id=?1)+(SELECT COUNT(*) FROM items WHERE asset_account_id=?1)",[id],|r|r.get(0))?;
            if actual.as_deref() != Some(kind) || mapped > 0 {
                return Err(invalid(
                    "GST outputs need dedicated liability accounts; reviewed input credits need dedicated asset accounts.",
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_gst_number(input: &DocumentInput, number: &str) -> Result<()> {
    if ((input.gst.is_some() && input.kind == "sale")
        || ["credit_note", "debit_note"].contains(&input.kind.as_str()))
        && !valid_invoice_number(number)
    {
        return Err(invalid(
            "A GST sales invoice or return note number must be 1–16 alphanumeric, hyphen or slash characters. Adjust numbering.",
        ));
    }
    Ok(())
}
fn amount(n: i128) -> Result<i64> {
    i64::try_from(n).map_err(|_| invalid("GST amount exceeds range."))
}
pub(crate) fn calculate_gst(
    tx: &Transaction<'_>,
    d: &DocumentInput,
) -> Result<Option<(GstCalculation, GstAccounts)>> {
    let Some(g) = &d.gst else {
        return Ok(None);
    };
    if let Some(itc) = &g.itc_allocation {
        if d.kind != "purchase"
            || g.classification.is_some()
            || itc.eligible_bps.len() != d.lines.len()
            || itc.eligible_bps.iter().any(|n| *n > 10000)
        {
            return Err(invalid(
                "Reviewed ITC allocations apply only to regular taxable purchases, with a 0–100% eligible share for each line.",
            ));
        }
        clean_text(&itc.reason, "Reviewed ITC allocation basis", 500, true)?;
        if itc
            .return_treatment
            .as_deref()
            .is_some_and(|s| !["permanent_reversal", "excluded_4d2"].contains(&s))
        {
            return Err(invalid(
                "Select reviewed permanent reversal, excluded 4(D)(2), or leave return mapping pending.",
            ));
        }
    }
    if let Some(c) = &g.classification {
        if !["sale", "purchase"].contains(&d.kind.as_str())
            || !["nil_rated", "exempt", "non_gst"].contains(&c.kind.as_str())
        {
            return Err(invalid(
                "Select a supported nil-rated, exempt or non-GST classification.",
            ));
        }
        clean_text(&c.reason, "Reviewed classification basis", 500, true)?;
        if d.kind == "sale" && c.supplier_state.is_some() {
            return Err(invalid(
                "Sales use the configured supplier registration state.",
            ));
        }
        if d.kind == "purchase" && !c.supplier_state.as_deref().is_some_and(valid_state) {
            return Err(invalid(
                "Classified purchases require the reviewed supplier state code.",
            ));
        }
    }
    if !["sale", "purchase"].contains(&d.kind.as_str()) {
        return Err(invalid("GST applies only to sales or purchase invoices."));
    }
    let raw: Option<String> = tx
        .query_row("SELECT data FROM gst_profile WHERE id=1", [], |r| r.get(0))
        .optional()?;
    let p: GstProfile = serde_json::from_str(&raw.ok_or_else(|| {
        invalid("Configure the India GST registration before posting GST invoices.")
    })?)?;
    validate_gst_profile(tx, &p)?;
    if !valid_state(&g.supply_state) {
        return Err(invalid(
            "Enter a domestic two-digit place-of-supply state code.",
        ));
    }
    clean_text(&g.supply_state_name, "Place-of-supply state name", 80, true)?;
    clean_text(&g.counterparty_address, "Counterparty address", 500, true)?;
    clean_text(&g.delivery_address, "Delivery address", 500, true)?;
    if let Some(s) = &g.counterparty_gstin {
        validate_gstin(s)?;
    }
    if g.rates_bps.len() != d.lines.len() || g.hsn_sac.len() != d.lines.len() {
        return Err(invalid(
            "Enter an HSN/SAC and reviewed rate for every invoice line.",
        ));
    }
    let supplier_state = if d.kind == "sale" {
        if g.supplier_reference.is_some() || g.credit_reviewed {
            return Err(invalid(
                "Sales invoices cannot contain purchase-credit fields.",
            ));
        }
        p.state_code.clone()
    } else {
        if (g.classification.is_none() && !g.credit_reviewed)
            || (g.classification.is_some() && g.credit_reviewed)
            || g.supply_state != p.state_code
        {
            return Err(invalid(
                "Purchases require place of supply in the company's registered state. Review eligible ITC only for regular taxable purchases; classified zero-tax purchases cannot claim ITC.",
            ));
        }
        let reference = g
            .supplier_reference
            .as_deref()
            .ok_or_else(|| invalid("Enter the supplier's tax invoice reference."))?;
        if !valid_invoice_number(reference) {
            return Err(invalid(
                "Supplier tax invoice reference must have 1–16 invoice-number characters.",
            ));
        }
        if let Some(c) = &g.classification {
            let state = c.supplier_state.as_deref().unwrap();
            if g.counterparty_gstin
                .as_ref()
                .is_some_and(|id| !id.starts_with(state))
            {
                return Err(invalid(
                    "Reviewed supplier state must match the supplier GSTIN.",
                ));
            }
            state.into()
        } else {
            g.counterparty_gstin
                .as_ref()
                .ok_or_else(|| invalid("A purchase GST invoice requires the supplier's GSTIN."))?
                [..2]
                .into()
        }
    };
    let local = supplier_state == g.supply_state;
    let mut total = (0i128, 0i128, 0i128, 0i128);
    let mut lines = vec![];
    for (i, l) in d.lines.iter().enumerate() {
        check_gst_control(tx, l.account_id)?;
        let rate = g.rates_bps[i];
        let code = &g.hsn_sac[i];
        if (if g.classification.is_some() {
            rate != 0
        } else {
            !(1..=10000).contains(&rate)
        }) || ![4, 6, 8].contains(&code.len())
            || !code.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid(
                "Regular GST rates must be positive percentages up to 100, with two decimals; classified nil-rated/exempt/non-GST supplies must use zero. HSN/SAC must contain 4, 6 or 8 digits. Verify the classification, rate and code independently.",
            ));
        }
        let qty = crate::parse_quantity(&l.quantity)?;
        let price = parse_amount(&l.unit_price)?;
        let base = amount((i128::from(qty) * i128::from(price) + 500) / 1000)?;
        let numerator = i128::from(base) * i128::from(rate);
        let (c, s, i) = if local {
            let n = amount((numerator + 10000) / 20000)?;
            (n, n, 0)
        } else {
            (0, 0, amount((numerator + 5000) / 10000)?)
        };
        total.0 += i128::from(base);
        total.1 += i128::from(c);
        total.2 += i128::from(s);
        total.3 += i128::from(i);
        lines.push(GstLine {
            input_credit: g.itc_allocation.as_ref().map(|a| {
                InputCredit::from_values([c, s, i].map(|n| {
                    ((i128::from(n) * i128::from(a.eligible_bps[lines.len()]) + 5000) / 10000)
                        as i64
                }))
            }),
            taxable_minor: base.to_string(),
            rate_bps: rate,
            hsn_sac: code.clone(),
            cgst_minor: c.to_string(),
            sgst_minor: s.to_string(),
            igst_minor: i.to_string(),
        });
    }
    amount(total.0 + total.1 + total.2 + total.3)?;
    let calc = GstCalculation {
        taxable_minor: total.0.to_string(),
        cgst_minor: total.1.to_string(),
        sgst_minor: total.2.to_string(),
        igst_minor: total.3.to_string(),
        lines,
    };
    Ok(Some((
        calc,
        if d.kind == "sale" { p.output } else { p.input },
    )))
}
impl GstLine {
    pub(crate) fn credit_values(&self) -> Result<[i64; 3]> {
        self.input_credit
            .as_ref()
            .cloned()
            .unwrap_or_else(|| InputCredit {
                cgst_minor: self.cgst_minor.clone(),
                sgst_minor: self.sgst_minor.clone(),
                igst_minor: self.igst_minor.clone(),
            })
            .values()
    }
    pub(crate) fn nonrecoverable(&self) -> Result<i64> {
        let credit = self.credit_values()?;
        let full = InputCredit {
            cgst_minor: self.cgst_minor.clone(),
            sgst_minor: self.sgst_minor.clone(),
            igst_minor: self.igst_minor.clone(),
        }
        .values()?;
        let mut n = 0i64;
        for i in 0..3 {
            if credit[i] < 0 || credit[i] > full[i] {
                return Err(invalid("Input credit exceeds the invoice tax."));
            }
            n = n
                .checked_add(full[i] - credit[i])
                .ok_or_else(|| invalid("Nonrecoverable tax exceeds range."))?;
        }
        Ok(n)
    }
}
impl GstCalculation {
    pub(crate) fn credit_values(&self) -> Result<[i64; 3]> {
        let mut sum = [0i64; 3];
        for l in &self.lines {
            let n = l.credit_values()?;
            for i in 0..3 {
                sum[i] = sum[i]
                    .checked_add(n[i])
                    .ok_or_else(|| invalid("Input credit exceeds range."))?;
            }
        }
        Ok(sum)
    }
}
impl LedgerStore {
    pub fn configure_gst(&mut self, p: GstProfile) -> Result<()> {
        self.require_company()?;
        let tx = self.conn.transaction()?;
        validate_gst_profile(&tx, &p)?;
        tx.execute(
            "INSERT INTO gst_profile(id,data) VALUES(1,?1)",
            [serde_json::to_string(&p)?],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn gst_profile(&self) -> Result<Option<GstProfile>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT data FROM gst_profile WHERE id=1", [], |r| r.get(0))
            .optional()?;
        raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn gst_control_accounts(&mut self) -> Result<serde_json::Value> {
        self.require_company()?;
        let tx = self.conn.transaction()?;
        let mut ids = vec![];
        for (code, name, kind) in [
            ("2101", "Output CGST", "liability"),
            ("2102", "Output SGST / UTGST", "liability"),
            ("2103", "Output IGST", "liability"),
            ("1401", "Reviewed input CGST", "asset"),
            ("1402", "Reviewed input SGST / UTGST", "asset"),
            ("1403", "Reviewed input IGST", "asset"),
        ] {
            let existing: Option<(i64, String)> = tx
                .query_row("SELECT id,kind FROM accounts WHERE code=?1", [code], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()?;
            let id = if let Some((id, k)) = existing {
                if k != kind {
                    return Err(invalid(
                        "A standard GST account code already has a different account type.",
                    ));
                }
                id
            } else {
                tx.execute(
                    "INSERT INTO accounts(code,name,kind) VALUES(?1,?2,?3)",
                    params![code, name, kind],
                )?;
                tx.last_insert_rowid()
            };
            ids.push(id);
        }
        tx.commit()?;
        Ok(
            serde_json::json!({"output":{"cgst":ids[0],"sgst":ids[1],"igst":ids[2]},"input":{"cgst":ids[3],"sgst":ids[4],"igst":ids[5]}}),
        )
    }
    pub fn gst_report(&self, from: &str, to: &str) -> Result<serde_json::Value> {
        ledger_core::validate_date(from)?;
        ledger_core::validate_date(to)?;
        if from > to {
            return Err(invalid("Invalid GST report range."));
        }
        let _read = if self.conn.is_autocommit() {
            Some(self.conn.unchecked_transaction()?)
        } else {
            None
        };
        let mut q=self.conn.prepare("SELECT d.data,(SELECT r.date FROM journals r WHERE r.reversal_of=d.journal_id) FROM documents d WHERE d.kind IN ('sale','purchase','credit_note','debit_note') ORDER BY d.date,d.id")?;
        let mut rows = vec![];
        let mut totals = std::collections::BTreeMap::<String, (i128, i128, i128, i128)>::new();
        for r in q.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })? {
            let (data, reversed) = r?;
            let d: crate::Document = serde_json::from_str(&data)?;
            let source: Option<crate::Document> = if d.correction.is_some() {
                let raw: String = self.conn.query_row(
                    "SELECT data FROM documents WHERE id=?1",
                    [d.input.invoice_id],
                    |r| r.get(0),
                )?;
                Some(serde_json::from_str(&raw)?)
            } else {
                None
            };
            let original = source.as_ref().unwrap_or(&d);
            let (Some(g), Some(calc)) = (&original.input.gst, &d.gst) else {
                continue;
            };
            let direction = if d.correction.is_some() { -1i128 } else { 1 };
            for (date, sign, event) in [
                (
                    Some(d.input.date.as_str()),
                    direction,
                    if d.correction.is_some() {
                        "return_note"
                    } else {
                        "invoice"
                    },
                ),
                (reversed.as_deref(), -1, "accounting_reversal"),
            ] {
                let Some(date) = date.filter(|date| *date >= from && *date <= to) else {
                    continue;
                };
                let total = totals.entry(original.input.kind.clone()).or_default();
                total.0 += sign
                    * calc
                        .taxable_minor
                        .parse::<i128>()
                        .map_err(|_| invalid("Invalid stored GST."))?;
                if original.input.kind == "purchase" {
                    let credit = calc.credit_values()?;
                    total.1 += sign * i128::from(credit[0]);
                    total.2 += sign * i128::from(credit[1]);
                    total.3 += sign * i128::from(credit[2]);
                } else {
                    total.1 += sign
                        * calc
                            .cgst_minor
                            .parse::<i128>()
                            .map_err(|_| invalid("Invalid stored GST."))?;
                    total.2 += sign
                        * calc
                            .sgst_minor
                            .parse::<i128>()
                            .map_err(|_| invalid("Invalid stored GST."))?;
                    total.3 += sign
                        * calc
                            .igst_minor
                            .parse::<i128>()
                            .map_err(|_| invalid("Invalid stored GST."))?;
                }
                rows.push(serde_json::json!({"document_id":d.id,"number":d.number,"original_invoice":source.as_ref().map(|s| &s.number),"supplier_reference":g.supplier_reference,"kind":d.input.kind,"classification":g.classification.as_ref().map(|c|c.kind.as_str()).unwrap_or("regular"),"classification_basis":g.classification.as_ref().map(|c|c.reason.as_str()),"itc_allocation":g.itc_allocation,"date":date,"event":event,"sign":sign,"counterparty_gstin":g.counterparty_gstin,"supply_state":g.supply_state,"supply_state_name":g.supply_state_name,"amounts":calc}));
            }
        }
        let totals=totals.into_iter().map(|(k,(b,c,s,i))|(k,serde_json::json!({"taxable_minor":b.to_string(),"cgst_minor":c.to_string(),"sgst_minor":s.to_string(),"igst_minor":i.to_string()}))).collect::<std::collections::BTreeMap<_,_>>();
        Ok(
            serde_json::json!({"from":from,"to":to,"rows":rows,"totals":totals,"filing_ready":false}),
        )
    }
}

pub(crate) fn check_gst_control(tx: &Transaction<'_>, id: i64) -> Result<()> {
    let raw: Option<String> = tx
        .query_row("SELECT data FROM gst_profile WHERE id=1", [], |r| r.get(0))
        .optional()?;
    if let Some(raw) = raw {
        let p: GstProfile = serde_json::from_str(&raw)?;
        if [
            p.output.cgst,
            p.output.sgst,
            p.output.igst,
            p.input.cgst,
            p.input.sgst,
            p.input.igst,
        ]
        .contains(&id)
        {
            return Err(invalid(
                "GST control accounts cannot be used for party or stock balances.",
            ));
        }
    }
    Ok(())
}
