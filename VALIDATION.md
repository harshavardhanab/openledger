# Roadmap implementation validation — 14 October 2026

Branch: `codex/roadmap-completion`, based on main commit `ad949af`. Local platform: Apple Silicon macOS, Node.js 22.22.1, Rust 1.99.

- Clean `npm ci` and locked Rust dependencies.
- 38 Rust tests: existing journal/reversal/report checks plus sales/purchases, partial receipts/payments, settlement chronology, perpetual weighted average inventory, fractional-cost rounding, negative/backdated stock rejection, financial statements, period closing, reconciliation, document-number collision rollback, company switching, batch import, backup v1 compatibility, business-record tamper rejection, failed migration rollback, interrupted-process recovery, 2,000-invoice paging with full report totals, intra/inter-state GST postings, gross settlements, reviewed ITC, tax reversal dates/tamper rejection, schema/backup v2 compatibility, and read-only inactive-book inventory, experimental Tally sign/mapping/idempotency/atomicity checks and XML rejection cases.
- Eighteen browser tests pass in Chromium and WebKit. They exercise interface invoice posting and receipts, HTML export, journal posting/reversal, report filtering, backup/restore and malformed restore protection, pagination, locale preferences, keyboard shortcuts, inventory adjustments/stock sales, bank reconciliation, empty-earnings period closing, GST setup/invoice/register exports, experimental Tally mapping/preview/import, and foreign-origin rejection. A regression test verifies access when the native locale is `C`, a saved locale is invalid, or optional preference storage is unavailable.
- Strict TypeScript/Vite build, Prettier, Rust formatting and Clippy with warnings denied pass.
- npm audit reports zero known vulnerabilities at this check. This is not an independent security audit.
- ARM64 macOS release application builds (approximately 9.65 MiB), receives an ad hoc signature, and passes `codesign --verify --deep --strict`. Native checks in an isolated temporary profile created a company, posted 1,000.00 capital, reversed it, saved a backup with the native dialog, restored the pre-reversal backup with a verified two-journal recovery file, and exported a CSV through the native save dialog. The restored trial balance showed matching 1,000.00 debit/credit totals. No notarization is claimed.
- The development package workflow succeeded for commit `faf2ee1` on macOS, Windows and Linux: macOS app ZIP, Windows NSIS installer, Debian and AppImage artifacts. Downloaded artifacts matched their checksum manifests; the downloaded macOS app passed ad hoc signature verification. [Run evidence](https://github.com/harshavardhanab/openledger/actions/runs/38606477941). This includes GST/Tally changes but predates the native POSIX-locale correction; it is not validation of the final candidate.
- Native Linux testing exposed an invalid `navigator.language` value (`C`) that crashed currency rendering after company setup. Display locale selection now validates optional preferences and the system value before falling back to `en-IN`. Native installer checks and final packaging must be rerun for this correction.
- Cross-platform build, browser tests, Windows/Linux native WebDriver smoke tests and package workflows are configured. Their exact run conclusions must be verified remotely; configuration alone is not platform execution evidence.

The free Tauri native driver lacks macOS WKWebView support. Browser WebKit testing is distinct from native desktop testing. Production signing secrets and release-signing environment protections were not configured at inspection time. No Developer ID notarization, Windows trusted-signature result, independent accounting/security review or full accessibility audit is claimed. See ROADMAP.md and docs/RELEASE.md for remaining gates.

## Historical foundation evidence

The following records describe the earlier foundation, not the current candidate.

### Foundation validation — 14 October 2026

Validated locally on Apple Silicon macOS with Rust 1.99 and Node.js 22.22.1.

- All 11 Rust tests passed with locked dependencies. This includes a 506-journal regression case: the list remains limited to 500, while activity counts, old journal details, ledgers, and balances include the full history.
- Strict TypeScript checking and the Vite production build passed.
- Tauri release build produced an ARM64 macOS application of approximately 9.2 MiB.
- The supplied app received a local ad hoc signature and passed `codesign --verify --deep --strict`. This is not Developer ID notarization.
- The packaged native app launched and displayed empty company setup through its Rust IPC connection.
- Browser UI checks created a test company and an expense account, blocked a one-paise posting mismatch, balanced and posted the entry, opened its bank ledger, posted a reversal, and verified the zero closing balance with both original and reversal lines preserved.
- Browser backup export, restore preview, and restore succeeded. The recovery file was inspected and contained the previous 9 accounts and 2 journals.
- The final native application starts with empty books. Browser test books are kept in a separate file under the local `.research/session/work` directory. Financial data is excluded from source and application archives.

Windows and Linux compilation is configured in GitHub Actions but has not been executed. Mobile applications, public repository publication, installers for other systems, and notarization are outside this milestone.

### Foundation repository migration checks

The source now lives at the root of `~/work/openledger` on `feat/open-ledger-foundation`. All previous session artifacts and working files were moved into Git-ignored `.research/`; the original supplied installer was copied into the archive without changing the original. Compatibility symlinks preserve earlier artifact paths.

The locked npm dependency audit reported zero known vulnerabilities at migration time. Rust Clippy passed for all workspace targets with warnings denied. Prettier formatting and Rust formatting checks passed from the relocated repository. All 11 accounting tests passed again, and the strict TypeScript/Vite build and native macOS release bundle succeeded at the new location. Windows/Linux CI and packaging remain unexecuted until the branch is pushed.
