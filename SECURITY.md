# Security Policy

Codex Monitor crosses several sensitive boundaries: local Codex CLI auth, a Rust desktop backend, BLE transport, and C code on a hardware device. Security reports are welcome.

## Supported Versions

The `main` branch and the latest GitHub release are supported. Older releases may receive fixes when the issue is severe and easy to backport.

## Reporting A Vulnerability

Please do not publish credential leaks, BLE abuse paths, or crash reproducers in a public issue.

Use GitHub private vulnerability reporting if it is available for this repository. If that is unavailable, open a minimal public issue asking for a private maintainer contact and do not include sensitive technical details.

Useful reports include:

- Steps to reproduce.
- Affected OS, Flipper firmware, and Codex CLI version.
- Whether credentials, local files, BLE pairing state, or device stability are affected.
- Any suggested mitigation.

## Security Model

- The backend reads Codex limits locally through the Codex CLI.
- The Flipper receives only percentages, reset labels, and a status byte.
- The BLE packet is fixed at 21 bytes and does not contain Codex tokens, account identifiers, prompts, or model names.

