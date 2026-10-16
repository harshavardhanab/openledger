# GST return preparation

**Financial statements → GST return preparation** prepares a read-only draft for one configured India GST registration. Choose a month, or a calendar quarter's ending month (March, June, September or December). The entire period is read in one SQLite snapshot. Preparation does not post, close, submit or mark a return filed.

## GSTR-1 candidates

Supported regular domestic sales are grouped into B2B, B2CL and B2CS. A registered recipient uses B2B. An unregistered inter-state invoice uses B2CL when its original gross value exceeds INR 100,000 from August 2024, or INR 250,000 for earlier invoice dates; other unregistered invoices use B2CS. The comparison is strict: an invoice exactly at the threshold is B2CS. This follows the [GST Portal preparation guide](https://tutorial.gst.gov.in/userguide/returns/Creation_of_Outward_Supplies_Return_in_GSTR-1.htm).

Sales credit notes use CDNR for registered recipients and CDNUR for original B2CL invoices. Other unregistered credits reduce B2CS by place of supply and rate. The original invoice's value/date determines the classification, not the smaller credit amount. Invoice and note dates determine their reporting period. Purchase return notes are commercial inward records and are not exported as supplier-issued outward notes.

Exports use the official offline CSV headers for B2B, B2CL, B2CS, CDNR and CDNUR. Rates are grouped per invoice/note; B2CS is aggregated by place of supply and rate. Credit-note CSV amounts are positive with note type `C`; their draft book totals are negative. Full invoice/note values repeat on each rate row, not each line's value. All decimal text is produced from integer minor units without binary floating point, including values above JavaScript's safe integer range. CSV quotes text and guards spreadsheet formulas. Draft names are prefixed `DRAFT-`.

The official [Returns Offline Tool archive](https://tutorial.gst.gov.in/offlineutilities/returns/gst_offline_tool.zip) was downloaded on 16 October 2026. Its SHA-256 was `f0a7d6f66bf71b30e9ecd045a664dda428c8fac239f51127d656e6faac60e312`. Only the included templates/readme were read; no executable was run or copied into the repository. Header layouts and column counts are pinned in the exporter. See the [official tool manual](https://tutorial.gst.gov.in/downloads/invoiceuploadofflineutility.pdf). CSV import/portal acceptance remains unverified and template changes require a fresh check.

## GSTR-3B book worksheet and reconciliation

The worksheet contains book-based 3.1(a) outward taxable amounts, 4(A)(5) reviewed inward tax amounts and state-wise 3.2 inter-state supplies to unregistered recipients. It includes dated returns and accounting reversals. The 4(A)(5) row intentionally leaves taxable value blank. This is a preparation worksheet, not an official GSTR-3B upload schema, tax payment calculation, or decision that all book credit can be claimed. [GST Portal GSTR-3B table mapping](https://tutorial.gst.gov.in/userguide/returns/GSTR3B.htm), [GSTR-2B/ITC guidance](https://tutorial.gst.gov.in/userguide/returns/FAQ_gstr2b.htm).

All six configured GST control accounts are reconciled independently against the GST register's period movement. Output activity is credit-positive; input activity is debit-positive. Opening balances are excluded. A manual adjustment can cause a difference. Net-zero manual GST postings still generate review items, even when balances match. Matching controls alone cannot establish a complete or compliant return.

## Review items and complete export

The complete `open-ledger-gst-return-draft`, version 1 JSON export preserves the query/date range, candidate rows and section totals, book worksheets, signed HSN B2B/B2C quantities and values, all six reconciliations, review items and CSV contents. All monetary/quantity values are strings. It always returns `filing_ready: false` and `portal_verified: false`.

Review items identify:

- Sales/purchase documents without GST classification. A missing tax calculation does not establish exempt, nil-rated or non-GST treatment.
- Accounting reversals. A reversed invoice is excluded from GSTR-1 CSV candidates as of the period end; the journal reversal is not converted into a statutory note/amendment. Both affected periods need review. GSTR-3B book figures still include the effective-date movements.
- Manual income/expense journals with no supply classification, and manual GST-control postings, including offsetting entries.
- Control-account differences, and differences between exact book tax and tax recalculated from aggregated CSV taxable values. The offline tool calculates tax; those differences require reconciliation.

Review HSN/UQC mappings, document-series totals, GSTR-2B/IMS, eligibility/time limits, amendments, e-invoices, all other applicable tables, payments, interest and late fees separately. The JSON includes an HSN review breakdown; it does not invent statutory UQC codes from free-text stock units. Unsupported tables are not populated with fabricated zero declarations. An empty candidate list is not a determination that a nil return can be filed.

See [India GST scope](INDIA_GST.md) for remaining transaction workflows and independent verification gates.
