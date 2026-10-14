# Changelog

## Unreleased roadmap implementation

- Customers and suppliers; exact sales/purchase invoices and linked partial settlements.
- Document numbering, local drafts, review, native exports and printable HTML.
- Fractional inventory, perpetual weighted average costing and paginated movement history.
- Profit/loss and balance sheet reports, bank reconciliation, period closing and carry-forward.
- Separate company/example files with persistent switching and locale preferences.
- Schema/backup version 2 with version 1 compatibility, linked-record validation and stable posting-line IDs.
- Atomic journal batch imports; paginated lists and large-book regression.
- Browser/native test harnesses, interrupted-write and migration recovery tests, release-signing workflow and manual update policy.

Taxes, verified Tally imports, inventory credit/debit notes, independent reviews and trusted stable release validation remain pending.

## Unreleased — 0.1.0 foundation

- Rust accounting engine with exact amounts, balanced posting, request idempotency, and linked reversals.
- SQLite persistence with transactional posting and database guards for posted records.
- Company and account setup, dated ledgers, trial balances, and CSV exports.
- Portable JSON backup validation and recovery copies before restore.
- React interface and native Tauri desktop shell; optional loopback browser preview.
- Eleven accounting regression tests, strict TypeScript checks, formatting and Rust lint checks.
- Cross-platform CI, packaging workflow, contributor guidance, and security reporting instructions.

This is a prerelease foundation. Inventory, invoicing workflows, taxes, payroll, Tally data import, period closing, and multi-user operation are not implemented.
