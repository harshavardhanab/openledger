# India GST accounting scope

Specified for the user's selected jurisdiction, India. This is a scoped regular-registration accounting implementation, subject to independent Indian tax review before a stable release or live statutory use.

## Supported behavior

Configure one GST registration in INR books, its postal address, state name/code, three output liability accounts and three reviewed input-credit asset accounts. Standard control accounts can be created from **Company & periods**. Registration mappings are immutable; another registration uses separate books. The application checks GSTIN structure and state consistency, not the registry or GSTIN checksum.

An invoice can explicitly enable domestic regular forward-charge GST. Enter place-of-supply state/name, counterparty GSTIN when registered, counterparty/delivery addresses, and a reviewed rate and HSN/SAC on every line. Purchases require the supplier's invoice reference/GSTIN, place of supply matching the company's registered state, and explicit independent input-credit review. Rates are user inputs; there is no hardcoded product-rate catalogue or automated eligibility decision. Four-, six-, and eight-digit codes are structurally supported; the correct required classification/detail remains a reviewed input.

For ordinary intra-state supplies, central and state/UT components apply; ordinary inter-state supplies use integrated tax. The implemented branch follows supplier registration state versus the entered place of supply, consistent with the GST Portal's domestic invoice workflow. Place of supply is not inferred from a billing address. Special transactions can differ and are excluded here. [Official GST Portal invoice instructions](https://tutorial.gst.gov.in/userguide/returns/Creation_of_Outward_Supplies_Return_in_GSTR-1.htm).

GST sales numbering is restricted to 1–16 alphanumeric, hyphen or slash characters; numbers remain globally unique in the books, which is stricter than annual reuse. Export includes registration/address information, date, number, line descriptions/quantities, HSN/SAC, reviewed rates, taxable values and separate tax components, place of supply, reverse-charge status and a supplier-authorization space. Review supplier authorization and all applicable legal particulars before issue. [CBIC Rule 46](https://taxinformation.cbic.gov.in/content-page/explore-rules/1000136/1000001), [CBIC place-of-supply circular](https://cbic-gst.gov.in/pdf/circular-cgst-90.pdf).

## Exact calculations and postings

The rate is stored as integer basis points: entered 18.00% means 1,800, rather than binary floating point. The taxable extension is rounded half up to paise. Each intra-state component rounds half up independently at half the combined rate; integrated tax rounds at the full rate. This is an explicit invoice-accounting rounding policy. Statutory return rounding/adjustments need separate review; the app does not round a return liability for filing.

For a synthetic 1,000.00 taxable example with a reviewed combined rate of 18%, an intra-state sale debits receivables 1,180.00 and credits income 1,000.00, output CGST 90.00 and output SGST/UTGST 90.00. An inter-state example credits output IGST 180.00. The 18% fixture is an arithmetic example, not a product-rate recommendation.

An eligible-credit purchase at the same example amounts debits expense/inventory 1,000.00 and reviewed input tax 180.00, and credits payable 1,180.00. Recoverable GST does not inflate inventory carrying cost. Payments and outstanding invoices use the gross tax-inclusive amount. Tax profiles, entered particulars, calculated amounts and linked journals are checked again during backup restore.

Input credit is not established just by receiving an invoice. Its availability can be restricted or blocked; the purchase checkbox records the user's separate review rather than granting entitlement. Ineligible/blocked credit, apportionment and deferred recognition need additional workflows. [CBIC section 17](https://taxinformation.cbic.gov.in/content-page/explore-act/1000286/1000001), [CBIC input-credit rules](https://cbic-gst.gov.in/input-tax-credit-rules.html).

## Register and remaining scope

**Financial statements → GST accounting register** exports a dated, line-level review worksheet with HSN/SAC, rates, taxable values and separate tax components. Ordinary accounting reversals appear on their effective date with negative amounts. They are not represented as statutory credit/debit notes. The API explicitly returns `filing_ready: false`.

This is not a GSTR-1/GSTR-3B JSON uploader, portal filing integration, statutory credit-note workflow, e-invoice/IRN/QR-code generator, or e-way bill system. Composition, reverse charge, SEZ/exports/deemed exports, exempt/nil-rated bills of supply, cess, advances, withholding, mixed/composite supply classification, multiple GST registrations per file, and time-of-supply/registration-date determinations are not implemented. Do not encode these cases as ordinary supported invoices.

GST Portal reporting contains additional tables and classifications beyond invoice tax arithmetic. The current portal instructions distinguish consumer invoice thresholds from older FAQ guidance and describe IRP-generated e-invoice metadata. No historical threshold or incomplete filing schema is embedded in the application. [Current official GSTR-1 preparation guide](https://tutorial.gst.gov.in/userguide/returns/Creation_of_Outward_Supplies_Return_in_GSTR-1.htm).

Independent tax review and realistic anonymized fixtures remain release gates under issue #3. This specification was checked against the linked primary sources on 14 October 2026; changes to law, rates or portal formats require fresh verification.
