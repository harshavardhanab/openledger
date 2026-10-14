# Import and export contracts

## Portable books

JSON backups use `format: "open-ledger-backup"`, version 2. They include the company, accounts, complete immutable journals and posting-line IDs, parties, inventory items, documents, stock movements, numbering, periods, and reconciliations. The exporter writes a timestamp. Restore builds a separate in-memory database, validates all accounts and journals, re-derives document postings and stock valuations, verifies period closing journals and bank statements, then replaces the destination. The service saves a recovery backup before replacement. Book switching metadata and local drafts are not accounting records and are not exported.

Version 1 foundation backups remain readable. Version 1 SQLite databases migrate transactionally to schema 2. Future versions are rejected. Version 2 backups require business records and source links; an invoice or closing journal cannot silently lose its metadata. Backup imports are limited to 20 MiB. This is a structural validation format, not a cryptographic authenticity guarantee. Store backups securely.

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

CSV reports are quoted UTF-8 display exports; potentially executable spreadsheet strings are escaped. Numeric report displays follow the selected locale. Stock and financial-statement exports also include integer minor-unit amounts for precise interchange. Invoice HTML exports escape text, contain no scripts, and can be printed to PDF. They are general accounting documents without jurisdiction-specific tax fields.

## Tally importer design — specification required

Native Tally databases and installers are not input formats. The future importer should accept an explicitly supported exported dataset, with a declared Tally version, export dialect, currency precision, fiscal range, and anonymized fixtures. Do not infer compatibility from the decompiled application.

The first scope should be chart-of-accounts masters and balanced posted vouchers, followed by optional party allocations and stock movements. The adapter must preserve source identifiers, map account groups explicitly to Open Ledger account kinds, and create a reviewable mapping preview. All documents, taxes, inventory costs, voucher cancellations, and opening balances need explicit transformation rules. Unsupported voucher types must be reported rather than silently collapsed into generic entries.

Parse exports with external entities and network resolution disabled; enforce size, depth, record, date, and amount limits. Stage the entire import with idempotent source IDs. Verify independently supplied opening/closing balances, voucher counts, party balances, and stock totals before committing. Tests must include real anonymized exports from every claimed supported version. No verified Tally import compatibility is currently claimed.
