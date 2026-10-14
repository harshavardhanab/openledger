# Validation — 14 October 2026

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

## Repository migration checks

The source now lives at the root of `~/work/openledger` on `feat/open-ledger-foundation`. All previous session artifacts and working files were moved into Git-ignored `.research/`; the original supplied installer was copied into the archive without changing the original. Compatibility symlinks preserve earlier artifact paths.

The locked npm dependency audit reported zero known vulnerabilities at migration time. Rust Clippy passed for all workspace targets with warnings denied. Prettier formatting and Rust formatting checks passed from the relocated repository. All 11 accounting tests passed again, and the strict TypeScript/Vite build and native macOS release bundle succeeded at the new location. Windows/Linux CI and packaging remain unexecuted until the branch is pushed.
