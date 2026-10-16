# Changelog

## Unreleased roadmap implementation

- Customers and suppliers; exact sales/purchase invoices and linked partial settlements.
- Reviewed blocked/apportioned purchase ITC, nonrecoverable cost, exact supplier-return credit allocations and draft permanent/excluded return mappings.
- Explicit nil-rated/exempt/non-GST sales and purchases, reviewed classification/supplier state, bill-of-supply exports and classified return/draft reporting.
- Monthly/quarterly GST draft preparation: GSTR-1 candidate CSVs, GSTR-3B book worksheet, HSN review and GST-control reconciliation with unresolved source/rounding items.
- India GST registration, reviewed per-line rates/HSN, component tax postings, reviewed purchase ITC and register exports.
- Linked partial/full invoice returns, stock cost restoration/variance, explicit GST-adjustment review and customer/supplier refunds.
- Document numbering, local drafts, review, native exports and printable HTML.
- Fractional inventory, perpetual weighted average costing and paginated movement history.
- Profit/loss and balance sheet reports, bank reconciliation, period closing and carry-forward.
- Separate company/example files with persistent switching and locale preferences.
- Schema/backup version 6 with version 1/2/3/4/5 compatibility, linked-record validation and stable posting-line IDs.
- Experimental Tally XML general-voucher adapter with mapping, preview, staged commit and defensive parsing.
- Atomic journal batch imports; paginated lists and large-book regression.
- WCAG automated page/form checks, stronger contrast, forced colors, skip navigation and narrow reflow/keyboard checks.
- AppImage runtime launch test in the installed-package matrix.
- Browser/native test harnesses, interrupted-write and migration recovery tests, release-signing workflow and manual update policy.

India domestic regular GST accounting is included; statutory GST filing/special cases, verified Tally imports, independent reviews and trusted stable release validation remain pending.

## Unreleased — 0.1.0 foundation

- Rust accounting engine with exact amounts, balanced posting, request idempotency, and linked reversals.
- SQLite persistence with transactional posting and database guards for posted records.
- Company and account setup, dated ledgers, trial balances, and CSV exports.
- Portable JSON backup validation and recovery copies before restore.
- React interface and native Tauri desktop shell; optional loopback browser preview.
- Eleven accounting regression tests, strict TypeScript checks, formatting and Rust lint checks.
- Cross-platform CI, packaging workflow, contributor guidance, and security reporting instructions.

This is a prerelease foundation. Inventory, invoicing workflows, taxes, payroll, Tally data import, period closing, and multi-user operation are not implemented.
