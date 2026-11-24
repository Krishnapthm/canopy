# Xcode simctl and mcpbridge (Xcode 27.2, build 27B52, macOS 27.2)

Checked 2026-11-18 on the installed Xcode, not from memory.

- `xcrun simctl list -j devices` prints `{"devices": {<runtime id>: [device]}}`.
  Runtime ids look like `com.apple.CoreSimulator.SimRuntime.iOS-26-5`. Each
  device has `udid`, `name`, `state` (`Shutdown`, `Booted`, ...),
  `isAvailable`, `deviceTypeIdentifier`, `dataPath`, `logPath`, and sometimes
  `lastUsedAt`, `dataPathSize`, `logPathSize`. `list -j` without a type has
  `devicetypes`, `runtimes`, `devices`, `pairs`.
- `simctl boot <udid>`; `simctl shutdown <udid> | all`.
- `simctl io <udid> screenshot [--type=png|tiff|bmp|gif|jpeg] [--display=..] [--mask=ignored|alpha|black] <file>`.
  `-` is NOT stdout on Xcode 27.2: it writes a file named `-` in the current
  folder and prints "Wrote screenshot to: <cwd>/-". With no `--display` it
  prints a "No display specified. Defaulting to display" note to stderr.
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

Checked 2026-11-18 for sim panes (iPhone 17, iOS 26.5):

- `simctl bootstatus <udid> -b` boots if needed and waits until booted
  (about 9 s cold); on a booted device it prints "Device already booted,
  nothing to do." and exits 0. Plain `simctl boot` on a booted device exits
  149 ("Unable to boot device in current state: Booted").
- `io <udid> screenshot --type=png --mask=alpha <file>` writes a 1206x2622
  PNG with alpha (transparent rounded corners) in about 0.5 s.
- `list -j devices available` takes about 0.4 s.
- simctl has no tap, swipe or key input (`help` lists none; `io` offers
  enumerate, poll, recordVideo, screenshot). `recordVideo --mask=alpha` is
  rendered black.
- Device Hub's bundle id is `com.apple.dt.Devices`: `open -b com.apple.dt.Devices`.

Checked 2026-11-18 for live sim panes (macOS 27.2.1, Xcode 27.2, CoreSimulator 1174.21,
iPhone 17, iOS 26.5), with a probe and then `canopy --sim-helper` under its sandbox:

- The main display is the IO port descriptor whose `state.displayClass` is 0 (two class 1
  descriptors have no surface). Its `framebufferSurface` is a 1206x2622 IOSurface, pixel
  format `BGRA` (0x42475241), 4 bytes per element.
- `registerScreenCallbacksWithUUID:callbackQueue:frameCallback:surfacesChangedCallback:propertiesChangedCallback:`
  works. surfacesChanged fires once at registration. frameCallback fired 0 times in 2 s on an
  idle home screen and 40 to 50 times a second while swiping or typing.
- DTUHID (`com.apple.coredevice.feature.remote.hid.digitizer` through
  `xpc_endpoint_create_mach_port_4sim` and `xpc_connection_enable_sim2host_4sim`) answers a
  barrier at once and takes everything: `IndigoDigitizerEvent` touch (points 0..1 from the top
  left, eventType 0 start, 1 move, 2 end) paged the home screen and opened Search,
  `IndigoKeyboardButtonEvent` usages 0x0b 0x0c typed "hi", `IndigoButtonEvent` page 0x0C usage
  0x40 went Home. Legacy Indigo was not tested.
- Rotation through `com.apple.coredevice.feature.remote.hid.vendordefined`
  (`IndigoVendorDefinedEvent`, orientation-picker-control) is accepted (the barrier answers) but
  does not rotate this runtime, in Safari either. idb uses it only when the runtime reports
  device motion, else the legacy Purple port. Not supported in Canopy yet.
- `bootstrap_check_in` of a fresh name in Canopy plus `bootstrap_look_up` in the helper hands
  over `IOSurfaceCreateMachPort`; `IOSurfaceLookupFromMachPort` gives the same 1206x2622 BGRA
  surface. This works under sandbox-exec with deny default, reads limited to the system, Xcode
  and `~/Library/Developer/CoreSimulator`, and mach-lookup limited to
  `com.apple.CoreSimulator.CoreSimulatorService` and the rendezvous name.
- Bezels: a device type's `profile.plist` names `chromeIdentifier` (iPhone 17:
  `com.apple.dt.devicekit.chrome.phone11`) and `framebufferMask`; `capabilities.plist` has
  `ScreenDimensionsCapability` main-screen width 1206, height 2622, scale 3. The chrome is
  `/Library/Developer/DeviceKit/Chrome/phone11.devicechrome/Contents/Resources`: `chrome.json`
  and PDFs. `PhoneComposite.pdf` is 436x908 points, opaque, the 402x874 screen centered;
  buttons are 16 points wide (`Vol BTN` 64 tall, `X_Power BTN` 101), placed by `inputs[].offsets`
  (normal and rollover) from the anchor edge of a frame 9 points wider each side.
  `xcrun simctl list -j devicetypes` gives each type's `bundlePath`.
- `simctl io <udid> recordVideo --codec=h264 --force <file>` records until SIGINT, then writes
  the movie and exits 0.

Verified by the orchestrator for P3 on Xcode 27.2 (27B52), `xcrun` version 72.

- `xcrun simctl shutdown <udid>` on an already stopped device exits 149. Stderr starts with
  `An error was encountered processing the command (domain=com.apple.CoreSimulator.SimError, code=405):`
  and ends with `Unable to shutdown device in current state: Shutdown`. Canopy treats that result as success.
- `xcrun simctl shutdown <unknown udid>` exits 148 with `Invalid device: <udid>` on stderr.
  Canopy reports this as an error.
- `xcrun simctl list devicetypes -j` and the device type's `profile.plist` have no screen size.
  **Actual size** uses the live framebuffer dimensions divided by the device scale from
  `capabilities.plist`. The action is disabled before the live frame and scale are available.
