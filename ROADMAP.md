# Roadmap

Implementation status on branch `codex/roadmap-completion`. Open Ledger remains a prerelease; unchecked release gates are not implied complete by implemented features.

## Current foundation — implemented

- [x] Company and accounts, exact balanced journals, immutable postings, linked reversals.
- [x] Ledgers with opening/running balances, trial balance, backups and restore with recovery copies.

## Functional milestone — implemented

- [x] Sales and purchase invoices with customers and suppliers.
- [x] Linked partial/full payments and receipts, due dates, as-of outstanding balances and overdue display.
- [x] Configurable numbering, device-local drafts, reviewed posting, HTML export and print-to-PDF.
- [x] Consistent vouchers and account mappings, validated against worked examples.

See [accounting workflows](docs/ACCOUNTING.md) for posting rules and correction limitations.

## Further accounting work

- [x] Bank reconciliation and immutable statement history; profit/loss and balance sheet reports.
- [x] Consecutive financial periods, opening-balance journal workflow, closing to equity and automatic balance carry-forward.
- [x] Inventory items, fractional units, movement audit, perpetual weighted average valuation and adjustments.
- [x] Inventory invoice credit/debit notes and cancellation with linked stock returns, exact proportional credits, reviewed valuation and linked customer/supplier refunds. See [returns](docs/RETURNS.md) and [#5](https://github.com/harshavardhanab/openledger/issues/5).
- [x] Scoped India regular GST specification, reviewed rates/HSN, exact CGST/SGST-or-UTGST/IGST postings and an accounting register. See [India GST scope](docs/INDIA_GST.md).
- [ ] Independent India tax verification, statutory return payloads/filing and special tax workflows outside that scope. Tracked in [#3](https://github.com/harshavardhanab/openledger/issues/3).
- [x] Documented native import/export formats and atomic journal-import preview/commit.
- [x] Separately designed, experimental Tally general-voucher XML adapter with explicit mapping, preview, atomic commit, stable source IDs and rejection of unsupported data. See [import/export contracts](docs/IMPORT_EXPORT.md).
- [ ] Extend and verify Tally adapters against anonymized exports from explicitly supported versions/dialects. The supplied installer was inspected statically but did not establish a genuine vendor/version or yield an export fixture. Tracked in [#4](https://github.com/harshavardhanab/openledger/issues/4).

## Before a stable release

- [x] Clean builds/packages on macOS ARM64, Windows x64 and Linux x64; macOS app-bundle manual checks and Windows NSIS/Debian/AppImage installed-package checks. AppImage runtime launch uses extract-and-run mode. Exact source/run evidence and architecture limits are in VALIDATION.md. Broader desktop/GPU checks remain release follow-ups.
- [x] Automated browser end-to-end workflows for posting, reports, invoices, reversal, backup/restore and malformed restore rejection in Chromium and WebKit.
- [x] Native Windows/Linux installed-package WebDriver smoke tests pass in CI; macOS native posting, GST, reports, export dialogs, reversal, backup/restore/recovery, malformed input, switching and restart checks pass for the documented candidate.
- [x] Backup compatibility and transactional schema migration tests, interrupted-write recovery and recovery-backup drills.
- [x] Keyboard basics, focus trapping, skip navigation, current-page semantics, locale-aware presentation and journal/document/ledger/stock pagination.
- [x] Automated WCAG A/AA checks on core pages/forms, improved text/control contrast, forced-colors support and 320-CSS-pixel reflow/keyboard checks. These are implementation self-checks; independent assistive-technology auditing remains below.
- [x] Large-book regression at 2,000 invoices with complete report totals. Broader capacity profiling remains necessary before raising documented limits.
- [ ] Independent screen reader, zoom/high-contrast, and full keyboard accessibility audit.
- [x] Separate company/example databases and persistent company switching.
- [ ] Independent accounting review and independent security review, with named reviewers, exact commit and resolved findings.
- [x] Trusted-candidate signing/notarization workflow and a manual signed-update policy.
- [ ] Production signing credentials, protected release environment, published verification key, successful OS signature/notarization checks, and approved stable artifacts.

Remaining external release gates are tracked in [#6](https://github.com/harshavardhanab/openledger/issues/6).

See [release policy](docs/RELEASE.md) for required credentials, reviews and platform checks. No dates or feature guarantees are assigned. Remaining work must be tracked as scoped GitHub issues. Payroll, multi-user operation, mobile applications, unit conversions and alternate stock valuations are outside this roadmap's current implementation scope.
