# Codex Monitor FAP

External Flipper Zero app for displaying Codex account limits.

The screen has two rows:

```text
5H  [bar] used% reset-time
1W  [bar] used% reset-day/time
```

The app does not talk to Codex directly. It receives a packed `CodexLimitsPacket` from the desktop backend over the Flipper BLE serial characteristic.

## Build

```powershell
ufbt
```

## Install And Launch

```powershell
ufbt launch FLIP_PORT=COM3
```

The BLE profile advertises as `Codex <flipper-name>`. Keep the app open while the backend is running.

