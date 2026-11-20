# Simulator screen size (Xcode 27.2, macOS 27.2.1)

Checked on the installed Xcode, not from memory. Used by `canvas.sim_size` to draw a phone at its physical size.

- Device pixel density: the device type's `capabilities.plist` (in its bundle's
  `Contents/Resources`), key `ScreenDimensionsCapability`. For iPhone 18 Pro Max:
  `main-screen-pitch` 460 (pixels per inch), `main-screen-width` 1320,
  `main-screen-scale` 3. The key is optional in the code: a device without it
  cannot be shown at actual size.
- `profile.plist` has no `mainScreen*` keys on this Xcode; do not read size from it.
- Display density: `CGDisplayScreenSize(displayID)` (CoreGraphics) returns the
  display's physical size in millimetres. A display 2048 points wide reports
  601.4 x 340.2 mm here: 86.5 points per inch. A display that reports 0 gives
  no answer, so actual size fails with a toast.
- gpui's `window.display(cx).id()` is the CGDirectDisplayID on macOS.
- Math: display points per device point = display ppi x device scale / pitch.
  1320 px at 460 ppi is 2.87 in, 248 points at 86.5 points per inch.
