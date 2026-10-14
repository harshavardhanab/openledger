# Open Ledger

A free, open-source, local accounting application written in Rust, React, and Tauri 2. This is a prerelease implementation of the general accounting roadmap. It does not use the decompiled Tally program or require Tally to run.

## What works

- Create separate company data files, persistently switch companies, and keep sample books separate, with a starter chart of eight accounts.
- Add accounts for assets, liabilities, equity, income, and expenses.
- Post balanced journals with two or more lines; correct entries through linked reversals.
- Inspect account ledgers with date ranges and opening/closing balances.
- View trial balances as of a chosen date and export reports to CSV.
- Create customers/suppliers, post sales/purchase invoices, record linked partial receipts/payments, and inspect outstanding balances.
- Configure document numbering, save local drafts, export HTML documents, and print to PDF.
- Track fractional inventory units, stock adjustments and movement history with perpetual weighted average costing.
- Reconcile bank statements, report profit/loss and balance sheets, and close consecutive financial periods to equity.
- Preview and atomically import documented JSON journal batches.
- Export portable JSON backups; preview and validate restores before changing records. Save a recovery backup before restoring existing books.

Amounts use exact integer minor units in Rust and strings/BigInt in the interface. Supported currencies are INR, USD, EUR, GBP, CAD, and AUD, each using two decimal places. Journal amounts must fit a signed 64-bit integer; accumulated reports use 128-bit arithmetic. Posted records have database guards against editing and deletion. Repeated posting requests are idempotent.

Jurisdiction-specific taxes/filing, verified Tally data import, payroll, multi-user access, and mobile packaging remain unimplemented. Inventory invoice cancellation/credit-note workflows and statement reopening also remain follow-ups; separate stock journals cannot be reversed. Lists have pagination; reports and backups include full history. The general accounting rules, examples, and correction limits are documented in [docs/ACCOUNTING.md](docs/ACCOUNTING.md).

## Run the desktop app

Build a new candidate from this branch or download artifacts from its exact successful GitHub Actions run. Earlier `.research/session/outputs` application copies are historical foundation builds. No Rust, Node, REA, Ghidra, or internet connection is needed to use a built app. Choose **Create company** or **Try example books**; examples use a separate file. Company switching and additional-company creation are in **Company & periods**.

The desktop app stores `open-ledger.sqlite` in the platform's application data directory for `io.openledger.desktop`. On macOS this is normally `~/Library/Application Support/io.openledger.desktop/`. Additional companies use `books-<uuid>.sqlite`; a small active-file pointer persists selection. Recovery backups are in its `recovery-backups` subdirectory. Set `OPEN_LEDGER_DATA_DIR` to an absolute directory for an isolated portable or test profile. Export backups from the app before moving machines. The SQLite database and JSON backups are not encrypted.

Development macOS packages have no Developer ID notarization. Stable signing, external reviews, and platform installation checks remain release gates. See [docs/RELEASE.md](docs/RELEASE.md).

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

The preview server binds only to loopback, checks browser origins and Host headers, and has no authentication. It is intended for local development. Keep it local. Desktop builds call Rust directly and do not run this HTTP server.

## Source structure

| Directory              | Purpose                                                                          |
| ---------------------- | -------------------------------------------------------------------------------- |
| `crates/ledger-core`   | Exact amounts, dates, account kinds, balanced journal validation                 |
| `crates/ledger-store`  | SQLite schema, posting, reversals, reports, backup validation and shared service |
| `crates/ledger-server` | Optional local browser preview                                                   |
| `src`                  | React interface, exact display amounts, CSV exports                              |
| `src-tauri`            | Native desktop shell and backup save dialog                                      |

`Cargo.lock` and `package-lock.json` pin dependencies. `.github/workflows/build.yml` runs accounting tests, frontend builds, and native desktop compilation on macOS, Windows, and Linux when the source is placed in a GitHub repository. The workflow also enforces Prettier formatting and Rust Clippy lint checks. It also exercises browser workflows and Windows/Linux native WebDriver smoke tests. `.github/workflows/package.yml` prepares development desktop artifacts and checksums on prerelease tags or manual runs. `trusted-release.yml` requires protected signing credentials for trusted candidates. These workflows do not publish a GitHub release.

## Project status and contributing

This is a working general accounting prerelease; remaining roadmap and release gates prevent claiming a complete production accounting suite. See [ROADMAP.md](ROADMAP.md) for the functional work and release validation still required. Read [CONTRIBUTING.md](CONTRIBUTING.md) before changing accounting behavior, [SECURITY.md](SECURITY.md) for private reporting and data boundaries, and [CHANGELOG.md](CHANGELOG.md) for the current scope.

Local decompiled files, original binaries, setup work, logs, and old build artifacts are archived in `.research/` and excluded from Git. They are not required to build or run the app. Hidden names alone do not make files private; this directory must stay ignored.

## Validation

Run `npm test` for the accounting, backup compatibility, migration, interrupted-write recovery, and large-book regressions. Run `npx playwright install chromium webkit`, then `npm run test:e2e` for browser workflows; the harness creates temporary books and never uses your normal database. Native Windows/Linux tests use `npm run test:desktop` after installing the native drivers and building the application. See [VALIDATION.md](VALIDATION.md) for exact results and remaining platform evidence.

See [docs/IMPORT_EXPORT.md](docs/IMPORT_EXPORT.md) for backup compatibility, journal import examples and the separate Tally importer design.

MIT licensed. See `LICENSE`.
