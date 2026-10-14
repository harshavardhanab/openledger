# Contributing

Open Ledger is an early accounting foundation. Please keep changes focused and describe the accounting behavior they affect.

## Local setup

Install Rust stable, Node.js matching `.nvmrc`, and your platform's [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Then run:

```sh
npm ci
npm run desktop
```

For the browser development loop, follow the README. Desktop and browser modes use different local databases.

## Before a pull request

```sh
npm run format:check
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
npm test
npm run build
```

Use `npm run format` and `cargo fmt --all` to format changed files. Use the lockfiles; dependency changes should explain why they are needed. CI runs checks and native compilation on macOS, Windows, and Linux.

Add focused regression tests when changing posting rules, money handling, reversals, reports, schema, or restore behavior. Assert independently calculated outcomes, not just that one implementation matches another. UI-only styling changes usually need visual verification rather than extra accounting tests.

## Accounting invariants

- Use exact minor units; do not convert money to floating point.
- Reject unbalanced, invalid, or incomplete journals before committing any rows.
- Keep posted entries immutable and corrections traceable through reversals.
- Preserve idempotency when requests are retried and books are restored.
- Validate a restore in a separate database before replacing current records.
- Save current books for recovery before restoring over an existing company.
- Keep database changes transactional. Introduce explicit migrations before changing schema version 1.

Include a clear problem statement, resulting behavior, validation, and any migration or compatibility notes in the pull request. Please do not include financial data, backup files, local research, decompiled artifacts, original installers, or generated builds in commits. `.research/` is intentionally excluded from this repository.
