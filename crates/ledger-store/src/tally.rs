//! Experimental adapter for a deliberately narrow, explicit Tally XML voucher profile.
use crate::{Result, store::invalid};
use ledger_core::{
    JournalInput, PostingInput, Side, clean_text, parse_amount, validate_date, validate_journal,
};
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
struct Node {
    name: String,
    text: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
}
impl Node {
    fn field(&self, name: &str) -> Result<&str> {
        let fields = self
            .children
            .iter()
            .filter(|n| n.name == name)
            .collect::<Vec<_>>();
        if fields.len() > 1 {
            return Err(invalid("Duplicate Tally XML fields are ambiguous."));
        }
        let Some(n) = fields.first() else {
            return Ok("");
        };
        if !n.children.is_empty() {
            return Err(invalid("Tally scalar field contains nested data."));
        }
        Ok(n.text.trim())
    }
    fn nonempty(&self) -> bool {
        !self.text.trim().is_empty() || self.children.iter().any(Self::nonempty)
    }
    fn collect<'a>(&'a self, name: &str, out: &mut Vec<&'a Self>) {
        if self.name == name {
            out.push(self);
        }
        for child in &self.children {
            child.collect(name, out);
        }
    }
}
fn xml_error(e: impl std::fmt::Display) -> crate::StoreError {
    invalid(&format!("Invalid Tally XML: {e}"))
}
fn parse(text: &str) -> Result<Node> {
    if text.len() > 20 * 1024 * 1024 {
        return Err(invalid("Tally XML is limited to 20 MiB."));
    }
    let mut reader = Reader::from_str(text);
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        let event = reader.read_event().map_err(xml_error)?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                count += 1;
                if count > 200000 || stack.len() > 32 {
                    return Err(invalid("Tally XML exceeds node or nesting limits."));
                }
                let name = e.name().as_ref().to_ascii_uppercase();
                let mut node = Node {
                    name,
                    ..Default::default()
                };
                for a in e.attributes() {
                    let a = a.map_err(xml_error)?;
                    let key = a.key.as_ref().to_ascii_uppercase();
                    let value = a
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(xml_error)?
                        .into_owned();
                    if node.attrs.insert(key, value).is_some() {
                        return Err(invalid("Duplicate XML attribute."));
                    }
                }
                if empty {
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err(invalid("Unmatched XML end tag."));
                }
                let n = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(n);
            }
            Event::Text(e) => stack.last_mut().unwrap().text.push_str(e.as_ref()),
            Event::CData(e) => stack.last_mut().unwrap().text.push_str(e.as_ref()),
            Event::GeneralRef(e) => {
                let reference = e.as_ref();
                let escaped = format!("&{reference};");
                let decoded = quick_xml::escape::unescape(&escaped).map_err(xml_error)?;
                stack.last_mut().unwrap().text.push_str(&decoded);
            }
            Event::DocType(_) | Event::PI(_) => {
                return Err(invalid(
                    "DTD, entities and processing instructions are not accepted.",
                ));
            }
            Event::Decl(e) => {
                if let Some(enc) = e.encoding() {
                    let enc = enc.map_err(xml_error)?;
                    if !enc.eq_ignore_ascii_case("UTF-8") {
                        return Err(invalid("Only UTF-8 Tally XML is supported."));
                    }
                }
            }
            Event::Eof => break,
            Event::Comment(_) => {}
        }
    }
    if stack.len() != 1 {
        return Err(invalid("Incomplete XML document."));
    }
    let mut root = stack.pop().unwrap();
    if root.children.len() != 1
        || root.children[0].name != "ENVELOPE"
        || !root.text.trim().is_empty()
    {
        return Err(invalid("Expected one Tally ENVELOPE document."));
    }
    Ok(root.children.remove(0))
}
#[derive(Debug, Clone, Serialize)]
pub struct TallyEntry {
    pub ledger: String,
    pub side: Side,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct TallyVoucher {
    pub guid: String,
    pub date: String,
    pub number: String,
    pub kind: String,
    pub narration: String,
    pub entries: Vec<TallyEntry>,
}
#[derive(Debug, Serialize)]
pub struct TallyInspection {
    pub profile: String,
    pub ledgers: BTreeSet<String>,
    pub vouchers: Vec<TallyVoucher>,
    pub compatibility_verified: bool,
}
fn check_structure(n: &Node) -> Result<()> {
    if ["LEDGER", "GROUP", "STOCKITEM", "STOCKGROUP", "VOUCHERTYPE"].contains(&n.name.as_str()) {
        return Err(invalid(
            "Master imports are not supported. Export vouchers only and map existing accounts.",
        ));
    }
    let unsupported = [
        "INVENTORY",
        "STOCKITEM",
        "BILLALLOCATIONS",
        "GST",
        "VAT",
        "TAXTYPE",
        "COSTCENTRE",
        "CATEGORYALLOCATIONS",
        "PAYROLL",
        "FOREX",
        "CURRENCY",
    ];
    if unsupported.iter().any(|s| n.name.contains(s)) && n.nonempty() {
        return Err(invalid(&format!(
            "Unsupported Tally data structure: {}. Nothing was imported.",
            n.name
        )));
    }
    for c in &n.children {
        check_structure(c)?;
    }
    Ok(())
}
pub fn inspect_tally(text: &str) -> Result<TallyInspection> {
    let root = parse(text)?;
    check_structure(&root)?;
    let mut nodes = vec![];
    root.collect("VOUCHER", &mut nodes);
    if nodes.is_empty() || nodes.len() > 10000 {
        return Err(invalid(
            "Import requires 1–10,000 supported general vouchers.",
        ));
    }
    let mut ledgers = BTreeSet::new();
    let mut guids = BTreeSet::new();
    let mut vouchers = vec![];
    for n in nodes {
        if n.attrs
            .get("ACTION")
            .is_some_and(|a| !a.eq_ignore_ascii_case("create"))
        {
            return Err(invalid(
                "Tally alteration/deletion actions are unsupported.",
            ));
        }
        for flag in ["ISCANCELLED", "ISOPTIONAL", "ISINVOICE"] {
            let v = n.field(flag)?;
            if !v.is_empty() && !v.eq_ignore_ascii_case("no") {
                return Err(invalid(
                    "Cancelled, optional or invoice-mode vouchers are unsupported.",
                ));
            }
        }
        let kind = n.field("VOUCHERTYPENAME")?;
        if n.attrs.get("VCHTYPE").is_some_and(|s| s != kind) {
            return Err(invalid("Voucher type attribute disagrees with its field."));
        }
        let mut nested = vec![];
        for c in &n.children {
            c.collect("VOUCHER", &mut nested);
        }
        if !nested.is_empty() {
            return Err(invalid("Nested vouchers are unsupported."));
        }
        if !["Journal", "Receipt", "Payment", "Contra"].contains(&kind) {
            return Err(invalid(
                "Only simple Journal, Receipt, Payment and Contra vouchers are supported.",
            ));
        }
        let guid = clean_text(n.field("GUID")?, "Tally voucher GUID", 64, true)?;
        if !guids.insert(guid.clone()) {
            return Err(invalid("Duplicate Tally voucher GUID."));
        }
        let raw = n.field("DATE")?;
        if raw.len() != 8 || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid("Tally date must use YYYYMMDD."));
        }
        let date = format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..]);
        validate_date(&date)?;
        let number = clean_text(n.field("VOUCHERNUMBER")?, "Voucher number", 64, true)?;
        let narration = clean_text(n.field("NARRATION")?, "Narration", 900, false)?;
        let mut entries = vec![];
        for line in n
            .children
            .iter()
            .filter(|c| ["ALLLEDGERENTRIES.LIST", "LEDGERENTRIES.LIST"].contains(&c.name.as_str()))
        {
            // Other nonempty list structures must not be silently discarded.
            for child in &line.children {
                if child.name.ends_with(".LIST") && child.nonempty() {
                    return Err(invalid(
                        "Voucher allocations require a separately specified import profile.",
                    ));
                }
            }
            let ledger = clean_text(line.field("LEDGERNAME")?, "Tally ledger", 120, true)?;
            let raw = line.field("AMOUNT")?;
            let (side, amount) = if let Some(n) = raw.strip_prefix('-') {
                (Side::Debit, n)
            } else {
                (Side::Credit, raw)
            };
            let n = parse_amount(amount)?;
            let deemed = line.field("ISDEEMEDPOSITIVE")?;
            if (side == Side::Debit && !deemed.eq_ignore_ascii_case("yes"))
                || (side == Side::Credit && !deemed.eq_ignore_ascii_case("no"))
            {
                return Err(invalid(
                    "Tally amount sign disagrees with ISDEEMEDPOSITIVE.",
                ));
            }
            ledgers.insert(ledger.clone());
            entries.push(TallyEntry {
                ledger,
                side,
                amount: ledger_core::decimal_amount(n),
            });
        }
        for child in &n.children {
            if child.name.ends_with(".LIST")
                && !["ALLLEDGERENTRIES.LIST", "LEDGERENTRIES.LIST"].contains(&child.name.as_str())
                && child.nonempty()
            {
                return Err(invalid("Unsupported voucher list structure."));
            }
        }
        let trial = JournalInput {
            date: date.clone(),
            reference: number.clone(),
            description: "Tally validation".into(),
            request_id: "validation".into(),
            lines: entries
                .iter()
                .map(|e| PostingInput {
                    account_id: 1,
                    side: e.side,
                    amount: e.amount.clone(),
                })
                .collect(),
        };
        validate_journal(trial)?;
        vouchers.push(TallyVoucher {
            guid,
            date,
            number,
            kind: kind.into(),
            narration,
            entries,
        });
    }
    Ok(TallyInspection {
        profile: "experimental-tally-general-vouchers-v1".into(),
        ledgers,
        vouchers,
        compatibility_verified: false,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TallyImport {
    pub contents: String,
    pub source_id: String,
    pub currency: String,
    pub accounts: BTreeMap<String, i64>,
    pub preview: bool,
}
impl TallyImport {
    pub fn journals(&self) -> Result<Vec<JournalInput>> {
        if self.currency != "INR"
            || self.source_id.is_empty()
            || self.source_id.len() > 30
            || !self
                .source_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(invalid(
                "Confirm INR source books and enter a stable 1–30 character company source ID (letters, numbers, hyphen, underscore).",
            ));
        }
        let inspection = inspect_tally(&self.contents)?;
        inspection
            .vouchers
            .into_iter()
            .map(|v| {
                let lines = v
                    .entries
                    .into_iter()
                    .map(|e| {
                        Ok(PostingInput {
                            account_id: *self.accounts.get(&e.ledger).ok_or_else(|| {
                                invalid(&format!("Map Tally ledger: {}", e.ledger))
                            })?,
                            side: e.side,
                            amount: e.amount,
                        })
                    })
                    .collect::<Result<_>>()?;
                Ok(JournalInput {
                    date: v.date,
                    reference: v.number,
                    description: if v.narration.is_empty() {
                        format!("Tally {} import", v.kind)
                    } else {
                        v.narration
                    },
                    request_id: format!("tally:{}:{}", self.source_id, v.guid),
                    lines,
                })
            })
            .collect()
    }
}
