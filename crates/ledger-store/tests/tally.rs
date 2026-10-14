use ledger_store::*;
use serde_json::json;
const XML: &str = include_str!("fixtures/tally-general.xml");
#[test]
fn tally_general_profile_resolves_entities_and_exact_debit_credit_signs() {
    let x = inspect_tally(XML).unwrap();
    assert_eq!(x.vouchers.len(), 1);
    assert!(x.ledgers.contains("Bank & Cash"));
    assert_eq!(x.vouchers[0].date, "2026-10-01");
    assert_eq!(x.vouchers[0].narration, "Opening capital & funding");
    assert!(!x.compatibility_verified);
    let input = TallyImport {
        contents: XML.into(),
        source_id: "source-book".into(),
        currency: "INR".into(),
        accounts: std::collections::BTreeMap::from([
            ("Bank & Cash".into(), 2),
            ("Capital".into(), 6),
        ]),
        preview: true,
    };
    let entries = input.journals().unwrap();
    assert_eq!(entries[0].lines[0].amount, "1000.01");
    assert_eq!(
        entries[0].request_id,
        "tally:source-book:synthetic-voucher-001"
    );
}
#[test]
fn tally_rejects_unsupported_allocations_actions_and_corrupt_balances() {
    for bad in [
        XML.replace(
            "1000.01</AMOUNT></ALLLEDGERENTRIES.LIST>\n</VOUCHER>",
            "999.99</AMOUNT></ALLLEDGERENTRIES.LIST>\n</VOUCHER>",
        ),
        XML.replace("<ISINVOICE>No", "<ISINVOICE>Yes"),
        XML.replace("ACTION=\"Create\"", "ACTION=\"Alter\""),
        XML.replace("<ISDEEMEDPOSITIVE>Yes", "<ISDEEMEDPOSITIVE>No"),
        XML.replace(
            "</VOUCHER>",
            "<BILLALLOCATIONS.LIST><AMOUNT>1</AMOUNT></BILLALLOCATIONS.LIST></VOUCHER>",
        ),
        XML.replace("<GUID>synthetic-voucher-001</GUID>", ""),
        XML.replace("<VOUCHERTYPENAME>Journal", "<VOUCHERTYPENAME>Sales"),
    ] {
        assert!(inspect_tally(&bad).is_err(), "{bad}");
    }
}
#[test]
fn tally_parser_rejects_external_entities_deep_trees_and_malformed_documents() {
    for bad in [
        format!("<!DOCTYPE ENVELOPE [<!ENTITY e SYSTEM 'file:///etc/passwd'>]>{XML}"),
        XML.replace("&amp;", "&external;"),
        "<ENVELOPE><VOUCHER></ENVELOPE>".into(),
        "<ENVELOPE>".repeat(40),
        format!("{XML}{XML}"),
        XML.replace("encoding=\"UTF-8\"", "encoding=\"UTF-16\""),
    ] {
        assert!(inspect_tally(&bad).is_err());
    }
}
#[test]
fn tally_preview_commit_mapping_and_repeat_import_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let service = LedgerService::open(&dir.path().join("tally.sqlite")).unwrap();
    service
        .request(
            "create_company",
            json!({"name":"Synthetic import","currency":"INR","fiscal_year_start":"2026-04-01"}),
        )
        .unwrap();
    let mut input = json!({"contents":XML,"source_id":"same-source","currency":"INR","accounts":{"Bank & Cash":2,"Capital":6},"preview":true});
    service.request("import_tally", input.clone()).unwrap();
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        0
    );
    input["preview"] = json!(false);
    let mut bad = input.clone();
    bad["accounts"]["Capital"] = json!(999);
    assert!(service.request("import_tally", bad).is_err());
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        0
    );
    service.request("import_tally", input.clone()).unwrap();
    service.request("import_tally", input.clone()).unwrap();
    assert_eq!(
        service.request("state", json!({})).unwrap()["journal_count"],
        1
    );
    input["contents"] = json!(XML.replace("1000.01", "1001.01"));
    assert!(service.request("import_tally", input).is_err());
}
