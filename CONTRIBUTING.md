# Contributing

Thanks for helping improve Codex Monitor. The project is intentionally small: a Rust backend, a Flipper FAP, and a fixed BLE packet between them.

## Development Setup

Backend:

```powershell
cargo test --manifest-path backend/Cargo.toml
cargo build --manifest-path backend/Cargo.toml
cargo run --manifest-path backend/Cargo.toml -- --smoke-test
```

Flipper app:

```powershell
cd flipper-app
ufbt
```

## Pull Request Checklist

- Keep the 21-byte `CodexLimitsPacket` contract compatible unless the PR explicitly changes both sides.
- Do not send Codex credentials, account identifiers, prompts, model names, or raw app-server responses to the Flipper.
- Add or update tests for Codex JSON parsing, packet serialization, BLE reconnect behavior, or UI state when relevant.
- Run backend tests and the FAP build before opening a PR.
- Keep docs honest about what is tested, especially for OS-specific BLE behavior.

## Useful Areas

- Windows BLE reliability.
- Linux pairing documentation.
- Mocked Codex app-server fixtures.
- Release packaging.
- Small docs improvements and screenshots.

