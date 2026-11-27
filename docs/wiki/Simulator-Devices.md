# Simulator Devices

A simulator pane puts a live iOS Simulator on the canvas, inside Xcode's device bezel. You and your agents use the same device: an agent can install the app it just built, tap through a flow and take screenshots, while you watch every step and take over with the mouse whenever you like. When something looks wrong, draw on the screen and send the marked-up picture straight to the agent.

![A live iPhone simulator pane wired to an agent](images/simulator-devices-1.png)

Simulator panes need macOS with Xcode and at least one simulator installed. For a walkthrough of an agent testing an app this way, see [Let Agents Show You](Let-Agents-Show-You#test-the-ios-app-on-a-simulator).

## Open a simulator

1. Click **Simulator** at the top right of the window. It opens as a tab in the [files panel](Files-Panel).
2. Click a simulator in the list. Canopy boots it if it is shut down.

The simulator opens docked in the files panel. Click **Move to canvas** on its helper bar to put it on the canvas, where it moves and scales with your other panes. On the canvas, **Open in panel** docks it again, and **Pin to screen** pins it to the top right corner at the same size while you pan and zoom (**Unpin** puts it back). Drag a pinned or canvas simulator's header to move it and any edge or corner to resize it.

The **Simulator** button shows only when Xcode's `xcrun simctl` is installed.

An agent opens one next to itself from its terminal. Panes an agent opens this way land on the canvas, not in the panel:

```sh
canopy pane devices                  # every simulator: udid, name, iOS version, booted or shutdown
canopy pane new sim booted --wire    # the booted simulator, wired to you
canopy pane new sim <udid>           # a specific simulator, booted if it is off
canopy pane new sim booted --url http://127.0.0.1:8000/   # open a page in its Safari
canopy pane open p3 http://127.0.0.1:8000/pricing         # open another page on the same device
```

These commands wait until the device is ready, then print its name, iOS version and state, or the reason it failed. Without `--url` they also print the path of a fresh screenshot. With one, the page is still drawing when the command returns, so `canopy pane read` gives a screenshot of it from the live screen. An already booted device answers in about a second. A shut down one adds Xcode's boot time, roughly ten seconds. Canopy runs Xcode's `xcrun simctl` itself, so this works from an agent's sandboxed shell where `simctl` cannot reach the simulator service.

`--url` takes any `http://` or `https://` address or an app link such as `myapp://settings`. The simulator cannot read the Mac's files: serve a local folder over loopback first, for example `python3 -m http.server 8000 -b 127.0.0.1`. The reply means the device accepted the address. Simulators report no page-load event, so read the pane to see the page.

## Use the device

- Click to tap. Drag to swipe. Scroll with two fingers on a trackpad.
- After a click, your typing goes to the simulator (US keyboard layout).
- Press `Cmd+V` to type the clipboard. Press `Cmd+Shift+H` for the Home screen.
- Press `Esc` to give the keyboard back to the canvas.
- Click the bezel's side buttons to press them.

The pane header shows the iOS version and the state: **Live**, **Connecting**, **Screenshot** or **Recording**.

## Resize the pane

For a docked simulator, drag the Files panel's edge to change the available width. Use **Move to canvas** to resize the pane directly. Drag any edge or corner of the canvas pane. When you release it, the pane fits the device and its bezel. Use the zoom buttons on the helper bar for smaller steps, **Actual size** to show the device at its real size, or **Default size** to reset the pane to 340 × 700 at 100% zoom. **Actual size** makes the pane one canvas pixel per device point, so an iPhone 16 screen is 393 points wide, and eases the camera to 100% so that is also its size on your screen. **Actual size** is available after the live screen connects.

## Shut down or close

- Click **Shut down** in the controls below the phone to turn off the booted device and return to the device picker. A toast confirms when Xcode finishes. An agent running `canopy do canvas.sim_shutdown --pane p3` gets the same result in its reply.
- Right-click the pane and choose **Shut down and close** to turn off the device and remove the pane in one step.
- Click **Close** to remove the pane and leave the device running. To close a selected pane from the keyboard, press `Delete`. If the simulator has keyboard focus, press `Esc` first.

## Bottom bar

Hover over or select a pane with a booted device to show its controls in a row below the phone.
The bar hides while annotating. In the device's own window, the bottom bar also offers **Bring back**.

| Button | What it does |
|---|---|
| **Home** | Goes to the Home screen. |
| **Refresh** | Tries the live view again. |
| **Screenshot** | Saves a PNG to the Desktop. The toast shows the file. |
| **Record** | Records a video to the Desktop. Click **Stop recording** to stop. |
| **Annotate** | Freezes the screen so you can pin comments and draw. |
| **Shut down** | Turns off the device and shows the device picker. |

## Annotate

To point out a problem on a booted simulator, click **Annotate** in the bar below the device. The screen freezes inside its bezel at the current size. Clicks and drags on the still draw or pin comments. If the device is in its own window, click **Bring back** first. The tool is the same as the one in the [Browser Pane](Browser-Pane#annotate), with the same bar at the bottom of the pane.

- With **Select**, click a point or drag over an area. A numbered blue pin drops there and a single-line comment box opens beside it. Press `Enter` to **Add** the comment, `Cmd+Enter` to **Send** everything queued, or `Shift+Enter` for a new line. Click a pin to edit its comment.
- **Pen**, **Box** and **Text** draw on the still. Colour and stroke appear only while one of them is on. `Cmd+Z` and `Shift+Cmd+Z` undo and redo.
- Hold `Space` to use the live simulator. Release it to capture a fresh still with your existing annotations in the same device coordinates.

**Send** goes to the sole wired agent. If multiple agents are wired or only unwired agents are available, use the **Choose agent** icon beside **Connect** in the helper bar. The menu lists wired agents first. The agent receives the annotated screenshot, then the device name, runtime and numbered comments with their coordinates in device points. Text labels are listed in the message.

Click **Stop annotating** or press `Esc` to leave. With marks or comments on the still, choose **Keep editing** or **Discard**.

## Helper bar

| Button | What it does |
|---|---|
| **Zoom out**, **Zoom in** | Scales the device down by 0.8 or up by 1.25. |
| **Actual size**, **Default size** | Shows the live device at its point size at 100% zoom, or resets the pane to its opening size. |
| **Pop out** | Moves the device to its own window. **Bring back** returns it. |
| **Choose simulator** | Shows every simulator. Xcode's version is at the top. |
| **Open in Device Hub** | Opens Xcode's Device Hub. |
| **Pin to screen** or **Move to canvas** | Switches between a fixed screen position and moving with the canvas. |
| **Connect** | Starts a wire from the pane. |
| **Close** | Removes the pane and leaves the device running. |

## Agents drive the device

Agents use device points, with `0 0` at the top left.

```sh
canopy pane tap p3 120 400                # tap
canopy pane swipe p3 200 700 200 200 300  # swipe in 300 ms
canopy pane type p3 hello                 # type text
canopy pane press p3 home                 # also lock, power, siri, volume-up, volume-down, action
canopy pane read p3                       # device, state, size, a fresh screenshot path, the last 12 actions
```

An agent can drive a pane that is off screen: Canopy connects to the device for 15 seconds for each command. Agents install and launch apps with Xcode's own `xcrun simctl`.

## How it works

While the pane is on screen, Canopy starts a small helper in a sandbox with no network access.
The helper shares the simulator's screen with Canopy directly, so a still screen costs nothing.
The helper stops 5 seconds after the pane leaves the screen (so panning past it does not restart it), and at once when the pane closes or Canopy quits.

## Troubleshooting

- **The toolbar has no Simulator button.** Xcode's `simctl` is missing. Install Xcode and open it once.
- **"No simulators."** No simulator exists. Add one in Xcode's Device Hub.
- **The header says Screenshot, not Live.** The live view failed, and the toast says why. Click **Refresh** in the bottom bar to try again.
- **"This simulator no longer exists."** The simulator was deleted in Xcode. Click **Choose another**.
- **The device does not rotate.** Rotation is not supported.

## See also

- [Quotes](Quotes)
- [Wiring](Wiring)
- [Agent Pane Commands](Agent-Pane-Commands)
