# iOS simulator panes

A simulator pane shows a live iOS Simulator on the canvas, in Xcode's bezel. The
person watches every tap and can take over with the mouse. It needs a Mac with
Xcode. Canopy runs `xcrun simctl` itself, so these commands work even when your
shell is sandboxed and `simctl` cannot reach the simulator service.

## Pick the device

```sh
canopy pane devices
# 37770DF7-3AF6-4126-B49D-CF5DA970B8E5	iPhone 17 (iOS 26.5)	booted
# 8CEAEA44-B608-4B44-8356-C4F4E83E813F	iPhone 17 Pro (iOS 26.5)	shutdown
```

1. The person named a device: use its UDID.
2. Exactly one is booted: use `booted`.
3. Several are booted and nothing says which: list their names and ask once.
4. None is booted: the request to show something already allows starting one.
   Pick the device that matches what they asked for (an iPhone for "the phone"),
   else the first iPhone in the list, and say which you chose. Booting takes
   Xcode about ten seconds.

Never shut down a device you did not boot unless the person asks.

## Open it

```sh
canopy pane new sim booted --wire                                  # the booted device, beside you
canopy pane new sim <udid> --url http://127.0.0.1:8000/ --wire     # boots it if needed, opens the page
canopy pane open p4 http://127.0.0.1:8000/pricing                  # another page on the same device
canopy pane open p4 myapp://settings                               # an app link
canopy pane open p4 <udid>                                         # show another device in this pane
```

The reply waits for the device and prints its name, iOS version, state and
UDID. Without `--url` it adds a screenshot path. With a URL, Safari is still
drawing when the reply arrives, so run `canopy pane read p4` for a fresh
screenshot. Simulators report no page-load event. If a newer open or device selection
supersedes yours, Canopy reports cancellation.

- `--url` and `open` take `http://`, `https://` or an app link. A file path or
  `file:` URL is refused: the device cannot read the Mac's disk. Serve the folder
  on loopback (`python3 -m http.server 8000 -b 127.0.0.1`) and open its address.
  `127.0.0.1` on the simulator is the Mac.
- `canopy pane new sim` with no device shows a picker for the person.
- Panes you open land on the canvas; `--place panel` docks one in the Files panel.

## Read and drive it

```sh
canopy pane read p4                         # state, size in points, a fresh screenshot file, recent activity
canopy pane tap p4 201 437                  # tap at device points (0,0 is the top left)
canopy pane swipe p4 200 700 200 200 300    # swipe from one point to another in 300 ms
canopy pane type p4 "hello"                 # type on the hardware keyboard (US layout)
canopy pane press p4 home                   # home, lock, power, siri, volume-up, volume-down, action
canopy pane shot p4 /tmp/p4.png             # the pane as drawn on the canvas
```

Open the screenshot file from `read` before and after you act; points come from
its `points` attribute. Tap and type only when a URL or app link cannot do the
job. Install and launch apps with Xcode's tools (`xcodebuild`,
`xcrun simctl install booted <App.app>`, `xcrun simctl launch booted <bundle id>`).

## Stop it

- `canopy pane close p4` removes the pane and leaves the device running.
- `canopy do canvas.sim_shutdown '{"pane":"p4"}'` turns the device off (only when
  asked) and shows the picker; add `"close":true` to remove the pane too. The
  reply waits for Xcode and says `shutdown` or why it failed.

The person can annotate the screen; it arrives as `@<png path>` with numbered
comments whose points are relative to the screen's top left.
