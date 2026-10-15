# Roadmap implementation validation — 15 October 2026

Branch: `codex/roadmap-completion`, based on main commit `ad949af`. Application source candidate: `b19f798`. Later changes at `f114fe1` affect the Windows test runner only; application/dependency source is identical. Local platform: Apple Silicon macOS, Node.js 22.22.1, Rust 1.99.

- Clean `npm ci` and locked Rust dependencies.
- 38 Rust tests: existing journal/reversal/report checks plus sales/purchases, partial receipts/payments, settlement chronology, perpetual weighted average inventory, fractional-cost rounding, negative/backdated stock rejection, financial statements, period closing, reconciliation, document-number collision rollback, company switching, batch import, backup v1 compatibility, business-record tamper rejection, failed migration rollback, interrupted-process recovery, 2,000-invoice paging with full report totals, intra/inter-state GST postings, gross settlements, reviewed ITC, tax reversal dates/tamper rejection, schema/backup v2 compatibility, and read-only inactive-book inventory, experimental Tally sign/mapping/idempotency/atomicity checks and XML rejection cases.
- Eighteen browser tests pass in Chromium and WebKit. They exercise interface invoice posting and receipts, HTML export, journal posting/reversal, report filtering, backup/restore and malformed restore protection, pagination, locale preferences, keyboard shortcuts, inventory adjustments/stock sales, bank reconciliation, empty-earnings period closing, GST setup/invoice/register exports, experimental Tally mapping/preview/import, and foreign-origin rejection. A regression test verifies access when the native locale is `C`, a saved locale is invalid, or optional preference storage is unavailable.
- Strict TypeScript/Vite build, Prettier, Rust formatting and Clippy with warnings denied pass.
- The full macOS/Windows/Linux build-and-test matrix passed at `f114fe1`, including Rust/browser checks, native compilation and Windows/Linux native smoke tests. [Full CI run](https://github.com/harshavardhanab/openledger/actions/runs/38750164228). Subsequent validation-document changes do not alter application or test code.
- npm audit reported zero known vulnerabilities on 14 October 2026. This is not an independent security audit.
- ARM64 macOS release application builds (approximately 9.92 MiB), receives an ad hoc signature, and passes `codesign --verify --deep --strict`. Manual native checks used an isolated test profile. They configured synthetic GST books, posted a 1,000.00 taxable sale plus 90.00 CGST and 90.00 SGST, verified a balanced 1,180.00 receivable, saved and parsed the GST register CSV through the native dialog, saved a version 3 backup, reversed the invoice, restored its one-journal backup and inspected the preserved two-journal recovery file. A malformed backup was rejected without changing books. GST/financial cutoff dates excluded and included the invoice correctly. Switching to separate example books and back, quitting and restarting preserved the selected company's balances. No notarization is claimed.
- Development packaging passed for application commit `b19f798` on macOS ARM64, Windows x64 and Linux x64: macOS app ZIP, Windows NSIS installer, Debian and AppImage artifacts. [Package run](https://github.com/harshavardhanab/openledger/actions/runs/38610110268). The downloaded macOS archive matched its checksum and the extracted app passed ad hoc signature verification.
- Installed Windows NSIS and Debian package smoke tests passed using runner commit `f114fe1` with the exact same application source as `b19f798`. They verify package provenance/checksums, actual installation, rendered startup/reports, native IPC posting, filtered ledger totals, reversal, backup, restore with recovery and malformed-restore rejection. [Installed-package run](https://github.com/harshavardhanab/openledger/actions/runs/38750182479). Linux also verifies the AppImage checksum; the AppImage itself was built but not launched.
- Native Linux testing exposed a real dashboard crash when `navigator.language` is the POSIX value `C`. Display locales now validate saved/system values before falling back to `en-IN`; browser regressions also cover unavailable storage and successful invoice posting when optional draft cleanup fails. Linux native execution passes with that correction. Xvfb uses software rendering and does not establish coverage of every desktop/GPU combination.
- Windows WebView2 150+ ignores driver environment overrides in elevated hosts. Native tests now run at medium integrity, following [Tauri's upstream CI](https://github.com/tauri-apps/tauri/blob/dev/.github/workflows/test-api-e2e.yml) and the [WebView2 security change](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5645). Only the existing runner user receives workspace access; application source and OS security policies remain unchanged.

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
