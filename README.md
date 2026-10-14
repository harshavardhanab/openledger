# Open Ledger

A free, open-source, local accounting application written in Rust, React, and Tauri 2. This is the first working milestone of a new implementation. It does not use the decompiled Tally program or require Tally to run.

## What works

- Create one company per data file, with a starter chart of eight accounts.
- Add accounts for assets, liabilities, equity, income, and expenses.
- Post balanced journals with two or more lines; correct entries through linked reversals.
- Inspect account ledgers with date ranges and opening/closing balances.
- View trial balances as of a chosen date and export reports to CSV.
- Export portable JSON backups; preview and validate restores before changing records. Save a recovery backup before restoring existing books.

Amounts use exact integer minor units in Rust and strings/BigInt in the interface. Supported currencies are INR, USD, EUR, GBP, CAD, and AUD, each using two decimal places. Journal amounts must fit a signed 64-bit integer; accumulated reports use 128-bit arithmetic. Posted records have database guards against editing and deletion. Repeated posting requests are idempotent.

This milestone has no inventory, invoicing workflow, tax filing, payroll, bank reconciliation, multi-user access, Tally data import, or mobile packaging. The journal list shows the latest 500 entries; ledgers, reports, and backups include the full history. The financial-year start is metadata; closing periods and carry-forward workflows are not implemented.

## Run the desktop app

For this local checkout, the previously verified Apple Silicon macOS app is at `.research/session/outputs/Open Ledger.app`. Open that app to use the current foundation. Future packaged builds will be attached to GitHub Actions runs. No Rust, Node, REA, Ghidra, or internet connection is needed to use the built app. Choose **Create company** for empty books or **Try example books** to explore sample transactions. Example books can only be added to an empty data file.

The desktop app stores `open-ledger.sqlite` in the platform's application data directory for `io.openledger.desktop`. On macOS this is normally `~/Library/Application Support/io.openledger.desktop/`. Recovery backups are in its `recovery-backups` subdirectory. Export backups from the app before moving machines. The SQLite database and JSON backups are not encrypted.

The supplied macOS build is a local development distribution and has no Developer ID notarization. Windows and Linux builds are configured but have not been tested on this machine.

## Build from source

Install a current stable Rust toolchain, Node.js 22.22.1 or later (the local tested version is recorded in `.nvmrc`), and the [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. macOS desktop builds require Xcode Command Line Tools; Windows requires MSVC build tools and WebView2; Linux requires the distribution's WebKitGTK and development libraries.

Clone [this repository](https://github.com/harshavardhanab/openledger), change into its root, and run:

```sh
npm ci
cargo test --workspace --exclude open-ledger-desktop --locked
npm run desktop
```

The development command runs Vite and the native desktop application. Build a distributable on the target operating system:

```sh
npm run desktop:build
```

Artifacts are under `target/release/bundle/`, unless `CARGO_TARGET_DIR` is set. For a macOS app without DMG packaging, use `npm run desktop:build -- --bundles app`. If the macOS compiler cannot find system headers, run with `SDKROOT=$(xcrun --show-sdk-path)`.

## Browser preview

The optional Rust server uses the same accounting service and a separate local database:

```sh
npm ci
npm run build
cargo run -p ledger-server -- --data .data/open-ledger.sqlite --frontend dist --port 4173
```

Open `http://127.0.0.1:4173`. For frontend development, keep the server running and run `npm run dev` in another terminal, then open `http://127.0.0.1:1420`.

The preview server binds only to loopback, checks browser origins, and has no authentication. It is intended for local development. Keep it local. Desktop builds call Rust directly and do not run this HTTP server.

## Source structure

| Directory              | Purpose                                                                          |
| ---------------------- | -------------------------------------------------------------------------------- |
| `crates/ledger-core`   | Exact amounts, dates, account kinds, balanced journal validation                 |
| `crates/ledger-store`  | SQLite schema, posting, reversals, reports, backup validation and shared service |
| `crates/ledger-server` | Optional local browser preview                                                   |
| `src`                  | React interface, exact display amounts, CSV exports                              |
| `src-tauri`            | Native desktop shell and backup save dialog                                      |

`Cargo.lock` and `package-lock.json` pin dependencies. `.github/workflows/build.yml` runs accounting tests, frontend builds, and native desktop compilation on macOS, Windows, and Linux when the source is placed in a GitHub repository. The workflow also enforces Prettier formatting and Rust Clippy lint checks. It has not yet been run remotely. `.github/workflows/package.yml` prepares downloadable desktop artifacts on a version tag or manual run; it does not publish a GitHub release.

## Project status and contributing

This is a working prerelease foundation, not a completed production accounting suite. See [ROADMAP.md](ROADMAP.md) for the functional work and release validation still required. Read [CONTRIBUTING.md](CONTRIBUTING.md) before changing accounting behavior, [SECURITY.md](SECURITY.md) for private reporting and data boundaries, and [CHANGELOG.md](CHANGELOG.md) for the current scope.

Local decompiled files, original binaries, setup work, logs, and old build artifacts are archived in `.research/` and excluded from Git. They are not required to build or run the app. Hidden names alone do not make files private; this directory must stay ignored.

## Validation

Eleven Rust tests cover independently calculated reports, date-range opening balances, exact large amounts, failed-post atomicity, idempotency through restores, reversal integrity, malformed backups, on-disk persistence, recovery backups, database immutability, and complete activity counts beyond the journal list limit. Run them with the test command above. The React build includes strict TypeScript checking.

MIT licensed. See `LICENSE`.
