# Xcode simctl and mcpbridge (Xcode 27.2, build 27B52, macOS 27.2)

Checked 2026-11-18 on the installed Xcode, not from memory.

- `xcrun simctl list -j devices` prints `{"devices": {<runtime id>: [device]}}`.
  Runtime ids look like `com.apple.CoreSimulator.SimRuntime.iOS-26-5`. Each
  device has `udid`, `name`, `state` (`Shutdown`, `Booted`, ...),
  `isAvailable`, `deviceTypeIdentifier`, `dataPath`, `logPath`, and sometimes
  `lastUsedAt`, `dataPathSize`, `logPathSize`. `list -j` without a type has
  `devicetypes`, `runtimes`, `devices`, `pairs`.
- `simctl boot <udid>`; `simctl shutdown <udid> | all`.
- `simctl io <udid> screenshot [--type=png|tiff|bmp|gif|jpeg] [--display=..] [--mask=ignored|alpha|black] <file|->`.
  `-` writes PNG to stdout. With no `--display` it prints a "No display
  specified. Defaulting to display" note to stderr.
- On a **shut down** device, `io screenshot` does not fail fast: it waits and
  exits 60 with "Timeout waiting for screen surfaces". Check `state` first.
- An unknown udid prints `Invalid device: <udid>`.
- Device Hub lives at `Xcode.app/Contents/Applications/DeviceHub.app`
  (no Simulator.app). Its only URL scheme is `devices:`
  (`com.apple.dt.Devices.devices`).
- `xcrun mcpbridge` is a STDIO bridge to Xcode's MCP tool service
  (`MCP_XCODE_PID`, `MCP_XCODE_SESSION_ID` env vars).
  `xcrun mcpbridge run-agent [--dry-run] [--no-xcode-tools] <agent> [args]`
  launches an agent with Xcode's config. `xcrun agent skills export [dir]`
  exports Xcode's skills.
