# Release and update policy

Open Ledger remains a prerelease until the remaining release gates in ROADMAP.md are met. Passing automated checks does not establish jurisdictional compliance or replace independent accounting/security reviews.

## Build evidence

`build.yml` runs locked Rust tests, formatting, Clippy, browser workflows in Chromium/WebKit, and native compilation across macOS, Windows, and Linux. Windows/Linux also exercise the packaged application's IPC and rendered reporting through native WebDriver. The free native Tauri driver does not support macOS WKWebView; macOS requires the manual native checklist alongside WebKit browser tests. See [Tauri testing](https://v2.tauri.app/develop/tests/webdriver/).

`package.yml` produces development artifacts and SHA-256 checksums. Its macOS signature is ad hoc. These artifacts must not be represented as trusted stable releases. CI run conclusions and downloaded packages must be checked for the exact source commit before claiming platform validation.

## Trusted candidates

`trusted-release.yml` is manual and uses the protected `release-signing` GitHub environment. Configure that environment to permit only reviewed release refs and require maintainer approval. This configuration must be made in GitHub; declaring an environment in YAML does not enable those protections. The workflow uploads candidates and does not publish a release.

Required secrets:

- All platforms: `RELEASE_GPG_PRIVATE_KEY` (armored private key), `RELEASE_GPG_PASSPHRASE`.
- macOS: `APPLE_CERTIFICATE` (base64 P12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific notarization credential), `APPLE_TEAM_ID`.
- Windows: `WINDOWS_CERTIFICATE` (base64 PFX) and `WINDOWS_CERTIFICATE_PASSWORD`. Hardware-backed signing requires a separately configured issuer service instead of this PFX path.

The workflow signs macOS packages, performs notarization and checks the stapled ticket; it validates Windows Authenticode signatures; it signs each platform's checksum manifest with GPG. Missing keys fail the trusted workflow. It has not been exercised with production credentials. Store signing keys outside the repository and establish a rotation/revocation plan. See the official [macOS signing](https://v2.tauri.app/distribute/sign/macos/) and [Windows signing](https://v2.tauri.app/distribute/sign/windows/) instructions.

## Safe updates

The app has no automatic updater or remote executable download path. Updates are explicit manual installations. Before updating, export every company's backup and verify the candidate on a copy of the books. Verify the release's detached manifest signature against a published, independently verified maintainer-key fingerprint; then verify the artifact's SHA-256 hash. Checksums without a trusted signature establish integrity, not publisher authenticity. Do not publish a stable release until a public verification key/fingerprint is established and OS signing checks pass.

A future automatic updater requires a pinned verification key, HTTPS endpoints, signed metadata and payloads, opt-in installation, rollback/downgrade restrictions, and migration/recovery tests. It must never execute unsigned update content or silently migrate all company files. Schema upgrades should reject unsupported future versions and preserve tested backup compatibility. No endpoint/key is invented in this repository.

## Independent reviews and manual native checks

A qualified accounting reviewer must verify source-to-ledger examples, income/expense signs, settlement chronology, fractional costing, cutoff dates, closing/carry-forward behavior, and exceptions. A separate security reviewer must inspect the IPC surface, loopback origins/hosts, restore/import parsing, filesystem selection, recovery atomicity, dependency risks, and signing pipeline. Record reviewer identity, exact commit, findings, and resolutions; self-review and automated checks are not independent reviews.

On each target OS, install the candidate in an empty test profile, create books, post and reverse an entry, filter and export reports, save a native backup, restore with recovery, reject a malformed backup, switch companies and restart. Verify all dialogs with keyboard navigation, focus order and screen reader output, high contrast/zoom, and application signature/install behavior. The browser tests cover keyboard basics; they are not a complete accessibility audit.
