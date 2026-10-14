# Security

Open Ledger is at version 0.1 and has not received an independent security review. The currently maintained code is on the development branch; no stable production release has been declared.

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/harshavardhanab/openledger/security/advisories/new) if the repository has it enabled. If it is unavailable, open an issue requesting a private contact channel without publishing exploit details or private data. No private email address is configured in this repository.

Include the affected version or commit, reproduction steps using fictional records, expected and actual behavior, and the possible impact. Never attach live financial databases or credentials.

## Data and deployment boundaries

- The desktop application stores books locally and does not require a cloud account.
- Database files and JSON backups are unencrypted. Protect them with operating-system permissions and disk encryption; store backups securely.
- The browser server is a local development preview, bound to loopback with an origin check. It has no authentication and must not be exposed through a public tunnel or remote interface.
- Native builds use Tauri capabilities and a content security policy. Changes that widen these permissions need explicit justification and review.
- Decompilation and session artifacts are kept in local `.research/`. Hidden names do not make tracked files private; this folder must remain ignored.
- Public release signing, notarization, secure update delivery, independent review, and broader deployment hardening remain release prerequisites.
