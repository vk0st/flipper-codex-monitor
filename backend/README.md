# Codex Monitor Backend

Rust backend for the Flipper `Codex Monitor` app.

It keeps a local `codex app-server --listen stdio://` process alive, polls `account/rateLimits/read`, converts the aggregate Codex limits into a fixed 21-byte packet, and sends that packet to the Flipper over BLE serial once per second.

## Run

```powershell
cargo run -- --smoke-test
cargo run
```

Expected smoke-test output includes your Codex login state and the current aggregate limits:

```text
Logged in using ChatGPT
Codex limits: 5H 26%, 1W 48%, status 0
```

## BLE Notes

Launch `Codex Monitor` on the Flipper first. The app advertises as `Codex <flipper-name>`, and the backend connects to that profile automatically. The default `Flipper <name>` BLE device is not the data channel used by this app.

On Linux, if service discovery times out, pair with the `Codex ...` device manually through `bluetoothctl`, then restart the backend.

