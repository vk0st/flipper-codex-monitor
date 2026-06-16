# Roadmap

Codex Monitor is an early, focused OSS project. The goal is to keep the scope narrow: show Codex limit state on dedicated hardware without exposing credentials or adding a heavy desktop UI.

## Near Term

- Keep BLE startup and reconnect behavior reliable on Windows.
- Add release artifacts for the backend and FAP so users do not need a full local toolchain.
- Expand tests around Codex app-server JSON shapes, stale data, and limit-reached states.
- Document first-time pairing and recovery flows with screenshots.

## Next

- Add mocked Codex app-server fixtures for contributor-friendly integration tests.
- Add mocked BLE packet tests for the Flipper packet contract.
- Improve Linux pairing documentation once more devices are tested.
- Add a small startup helper for users who want the backend to run after reboot.

## Non Goals

- Reintroducing CPU/RAM/GPU telemetry from PC Monitor.
- Sending Codex credentials or raw account data to the Flipper.
- Building a full desktop dashboard.

