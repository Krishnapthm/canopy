# Simulator Devices

A simulator pane puts a live iOS Simulator on the canvas, inside Xcode's device bezel. You and your agents use the same device: an agent can install the app it just built, tap through a flow and take screenshots, while you watch every step and take over with the mouse whenever you like. When something looks wrong, draw on the screen and send the marked-up picture straight to the agent.

![A live iPhone simulator pane wired to an agent](images/simulator-devices-1.png)

Simulator panes need macOS with Xcode and at least one simulator installed. For a walkthrough of an agent testing an app this way, see [Let Agents Show You](Let-Agents-Show-You#test-the-ios-app-on-a-simulator).

## Open a simulator

1. Click **Simulator** at the top right of the window. It opens as a tab in the [files panel](Files-Panel).
2. Click a simulator in the list. Canopy boots it if it is shut down.

The simulator opens docked in the files panel. Click **Move to canvas** on its helper bar to put it on the canvas, where it moves and scales with your other panes. On the canvas, **Open in panel** docks it again, and **Pin to screen** pins it to the top right corner at the same size while you pan and zoom (**Unpin** puts it back). Drag a pinned or canvas simulator's header to move it and its corner to resize it.

The **Simulator** button shows only when Xcode's `xcrun simctl` is installed.

An agent opens one next to itself from its terminal:

```sh
canopy pane new sim booted --wire    # the booted simulator, wired to you
canopy pane new sim <udid>           # a specific simulator
```

## Use the device

- Click to tap. Drag to swipe. Scroll with two fingers on a trackpad.
- After a click, your typing goes to the simulator (US keyboard layout).
- Press `Cmd+V` to type the clipboard. Press `Cmd+Shift+H` for the Home screen.
- Press `Esc` to give the keyboard back to the canvas.
- Click the bezel's side buttons to press them.

A simulator pane scales with the canvas like every pane. Zooming in stops before the biggest pane would leave the window, so the device always stays whole on screen. Resize it with its corner handle or the zoom buttons below.

The pane header shows the iOS version and the state: **Live**, **Connecting**, **Screenshot** or **Recording**.

## Bottom bar

Hover over or select a pane with a live device to show its device actions inside the pane.
The bar hides while annotating. In the device's own window, the bottom bar also offers **Bring back**.

| Button | What it does |
|---|---|
| **Home** | Goes to the Home screen. |
| **Refresh** | Tries the live view again. |
| **Screenshot** | Saves a PNG to the Desktop. The toast shows the file. |
| **Record** | Records a video to the Desktop. Click **Stop recording** to stop. |
| **Annotate** | Freezes the screen for marks and comments. |

## Annotate

To point out a problem on a booted simulator, click **Annotate** in the bottom bar. If the device is in its own window, click **Bring back** first.

- With **Select**, click a point or drag over an area, add a comment, then click **Save**. Click its numbered pin to edit or delete it.
- Draw with **Pen**, **Arrow**, or **Box**. To place a **Text** label, click, type the label, then press `Enter`. Choose a colour and width, or use **Undo** and **Redo**. On narrow panes, open **Drawing tools** for these controls.
- Use **Area screenshot** to attach a crop to an area comment. Use **Hide marks** to see the still beneath them, or **Trash** to clear all marks and comments in one undoable step.
- Hold `Space` to use the live simulator. Release it to capture a fresh still with your existing annotations in the same device coordinates.

Click **Send** to submit to the sole wired agent. With multiple wired agents or none, choose an agent from the quote menu. The agent receives the annotated screenshot, any crops, then the device name, runtime, and numbered comments with their coordinates in device points. Text labels are listed in the message.

Click **Stop annotating** or press `Esc` to leave. With marks or comments on the still, choose **Keep editing** or **Discard**.

## Helper bar

| Button | What it does |
|---|---|
| **Zoom out**, **Actual size**, **Zoom in** | Changes the device size. **Actual size** is 1 point per screen pixel at the current zoom. |
| **Pop out** | Moves the device to its own window. **Bring back** returns it. |
| **Choose simulator** | Shows every simulator. Xcode's version is at the top. |
| **Open in Device Hub** | Opens Xcode's Device Hub. |
| **Pin to screen** or **Move to canvas** | Switches between a fixed screen position and moving with the canvas. |
| **Connect** | Starts a wire from the pane. |
| **Close** | Closes the pane. |

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
