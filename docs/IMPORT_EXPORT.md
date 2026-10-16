# Import and export contracts

## Portable books

JSON backups use `format: "open-ledger-backup"`, version 5. They include the company, accounts, complete immutable journals and posting-line IDs, parties, inventory items, documents, stock movements, numbering, periods, reconciliations, and the optional India GST registration and invoice calculations. The exporter writes a timestamp. Restore builds a separate in-memory database, validates all accounts and journals, re-derives document postings and stock valuations, verifies period closing journals and bank statements, then replaces the destination. The service saves a recovery backup before replacement. Book switching metadata and local drafts are not accounting records and are not exported.

Version 1 foundation, version 2 business, version 3 GST and version 4 returns backups remain readable. Version 5 adds explicit nil-rated/exempt/non-GST supply classifications and reviewed supplier state for classified purchases. Version 4 adds quantity-linked return details, tax-adjustment review records and refunds. Older SQLite databases migrate transactionally to schema 5; listing inactive books uses a read-only connection. Future versions are rejected. Version 2 and newer backups require business records and source links; an invoice or closing journal cannot silently lose its metadata. Backup imports are limited to 20 MiB. This is a structural validation format, not a cryptographic authenticity guarantee. Store backups securely.

## Journal batch import

Use **Company & periods → Import journals** with this exact format:

```json
{
  "format": "open-ledger-journals",
  "version": 1,
  "journals": [
    {
      "date": "2026-10-01",
      "reference": "OPENING-001",
      "description": "Opening capital",
      "request_id": "external-system:opening-001",
      "lines": [
        { "account_id": 2, "side": "debit", "amount": "1000.00" },
        { "account_id": 6, "side": "credit", "amount": "1000.00" }
      ]
    }
  ]
}
```

Account IDs refer to existing accounts in the active company; use the chart of accounts to confirm mappings. Dates use ISO `YYYY-MM-DD`. Amounts must be decimal strings without currency symbols, separators, exponents, or signs. Each journal has 2–500 lines and equal debit/credit totals. References are at most 64 characters, descriptions 1,000, request IDs 128. Imports allow up to 10,000 journals within the request body limit. Unknown fields are rejected.

Preview validates the entire batch against staged copies of the active books and returns a trial balance. Import stages and validates again, then replaces the database atomically. Any invalid journal rejects the whole batch. Repeating identical request IDs and payloads does not duplicate journals. A changed payload with an existing ID fails. Inventory, period closing, and invoice workflows must use their dedicated commands, not generic imported journals. Export a backup before a large import.

CSV reports are quoted UTF-8 display exports; potentially executable spreadsheet strings are escaped. Numeric report displays follow the selected locale. Stock and financial-statement exports also include integer minor-unit amounts for precise interchange. Invoice HTML exports escape text, contain no scripts, and can be printed to PDF. GST invoice exports include the configured and entered particulars described in [India GST scope](INDIA_GST.md); unsupported cases and statutory filing require separate workflows.

## Experimental Tally general-voucher XML adapter

The implemented `experimental-tally-general-vouchers-v1` adapter accepts a UTF-8 `ENVELOPE` containing only simple Journal, Receipt, Payment and Contra vouchers. It requires `GUID`, `DATE` (`YYYYMMDD`), `VOUCHERTYPENAME`, `VOUCHERNUMBER`, and positive/negative `AMOUNT` plus `ISDEEMEDPOSITIVE` for every ledger entry. Both `ALLLEDGERENTRIES.LIST` and `LEDGERENTRIES.LIST` are recognized. Negative amounts with `Yes` become debits; positive amounts with `No` become credits. The sign and flag must agree, and every voucher must balance. These tags/sign examples follow [Tally's official XML documentation](https://help.tallysolutions.com/sample-xml/). The repository fixture is synthetic and independently written.

Use **Company & periods → Experimental Tally XML general-voucher import**. Inspect the XML, map every source ledger to an existing account, confirm INR source/destination currency and choose a stable source-company identifier. Preview validates staged books and shows the resulting trial balance before committing. Repeating identical source IDs/GUIDs is idempotent; changed data under the same identity rejects the whole batch. Do not change the source identifier to bypass a conflict.

The parser rejects DTDs, external/custom entities, processing instructions, non-UTF-8 declarations, malformed/duplicate fields, deep trees and oversized data. Limits are 20 MiB, 32 nesting levels, 200,000 nodes and 10,000 vouchers. Known unsupported financial structures, nonempty allocation lists, master imports, stock/GST/foreign-currency metadata, cancelled/optional vouchers and alteration/deletion actions reject the batch. Nothing is silently skipped. Normal XML predefined/character entities are decoded as data.

Invoice-mode sales/purchases, bill settlement allocations, stock, statutory tax records, master/opening-balance transformations, payroll and custom voucher types need separately specified adapters. Imported general journals do not populate invoice, stock or GST subledgers. Native Tally databases and installers are not input formats; decompiled artifacts are not used.

The API returns `compatibility_verified: false`: no verified versioned export fixture is available. A supplied installer was inspected without execution; its opaque payload did not establish a genuine Tally version or provide an export fixture. Compare the source opening/closing balances, voucher counts and mapped account totals on a copy of books before using an import. No broad Tally ERP 9 or TallyPrime version compatibility is claimed. Future adapters must preserve source IDs, provide explicit master/group mapping and verify document/party/stock/tax totals against anonymized real exports for every claimed version. See issue #4.
