# Browser Pane

A browser pane keeps web pages in tabs on the canvas, right next to the agent working on it. Its main use is your own app: put the dev server beside the agent, watch changes land, and when something looks wrong, click the element and quote it to the agent with its component and source file. Browser panes run on macOS only.

## Open a page

1. Click **Browser** at the top right of the window. A **New tab** pane opens with the address field focused.
2. Type an address such as `localhost:3000` and press Enter.

The pane opens pinned to the top right corner of the window, where it stays while you pan and zoom the canvas. Click **Move to canvas** on its helper bar to put it on the canvas with your other panes, or **Pin to screen** to pin it again.

Only `http` and `https` pages open. A bare local address like `localhost:3000` gets `http://`, and other bare hosts get `https://`.

Agents can open a browser pane too, usually wired to themselves:

```sh
canopy pane new browser localhost:3000 --wire
canopy pane open p2 http://localhost:3000/pricing  # point an existing pane at another page
```

`canopy pane open` also makes the new address the pane's **Home**.

## Tabs

Click **New tab** (the plus button) in the browser pane's header to open an empty
tab and focus its address field. Click a tab to switch to it, click its
**Close tab** button (shown on the selected tab and when you hover another) or
middle-click it to close it. Right-click a tab for **Open in** your browser and
**Close tab**. In [canvas mode](Canvas), `Cmd+W` also closes
the active tab of the selected browser pane.

Tabs are up to 160 px wide and shrink evenly, to 72 px, as more open. Each shows a
globe and its title: Canopy never loads a page's own icon, because that would be a
request outside the page.

Closing the active tab selects the tab to its left, or the next tab when the
first tab closes. Closing the last tab leaves the pane open with an empty tab.
Each tab keeps its own address and page history while its view is open.

## Links from terminals

Hold `Cmd` and click an HTTP or HTTPS link in a terminal. With **Open links**
set to **Built-in browser** (`built_in`), the link opens in a new tab in the
browser pane you used last in the current place. If that pane is unavailable, Canopy opens a browser pane
beside the terminal, or centers it when the terminal is off the canvas.
Choose **Default browser** (`default`) to open links in your system browser.
See [Settings and Themes](Settings-and-Themes#browser).

## Use the page

A browser pane is either resting or live:

- **Resting**, it shows a snapshot of the page. This keeps the canvas fast when you have several browsers open.
- **Click the page** to make it live. Scroll, click and type as in any browser. The page stays live while you pan, zoom or resize the pane. The canvas clips the page at its edges, and panes, helper bars and menus in front of the page remain visible and clickable.
- **Click outside the pane** to rest the page and take a fresh snapshot.

## The header

A browser pane's header has two rows. The pop-out window draws the same header.

**Row 1** holds the tabs and window controls, left to right:

- The tab list and **New tab**.
- **⋯**, the menu below.
- **Pop out**, **Expand** and **Close** (see below). In the pop-out, **Close** brings the page back, and **Pop out** and **Expand** are not shown. Its menu has no Viewport rows: the page fills the window.

**Row 2** holds navigation:

- **Back**, **Forward** and **Reload**.
- The **address field**. Type an address and press Enter to open it (`localhost:3000` gets `http://`); this also sets the pane's **Home**. Press Escape to put the current address back. While you are not typing, the field follows the page.
- **Annotate** (`canvas.browser_annotate`), a button with its label that starts annotating the page.

Drag the pane by the empty part of row 1, or by the [helper bar](Toolbar-and-Helper-Bar).

### The ⋯ menu

| Item | What it does |
|---|---|
| **Open in** *your browser* | Opens the current address in your system browser (the app that opens `https` links, or **Open in default browser** if Canopy cannot tell). |
| **Copy link** | Copies the address and shows a toast. |
| **Save screenshot** | Saves the page to `<temp>/canopy/<pane>.png` and shows the path in a toast. |
| **Open HTML file…** | Asks you to pick an `.html` or `.htm` file and opens it in a new tab as a `file:` page. This is the only way a `file:` page opens: the address field and agents are refused `file:` addresses. |
| **Viewport** | Choose **Responsive**, **1280 × 800**, **1024 × 768**, **Mobile 375 × 812**, or **Tablet 768 × 1024**. A check marks the saved choice. Responsive pages reflow as the pane resizes. Each fixed size sets the page's CSS viewport and scales it to fit, centered in the pane. |
| **Links** | **Open links in built-in browser**: checked, links clicked in terminals open in a browser tab; unchecked, in your system browser. The same as the **Open links** setting. |
| **Keep cookies** | Choose **Until quit** or **Forever**. A check marks the current choice. |
| **Clear browsing data** | After a confirmation, deletes the cookies, storage and cache of every browser pane. |

### Expand

**Expand** makes the page live and fills the whole canvas area with it, at 100% zoom. The top strip gets a tab for it, right after its space's tab (after Home for a pane on Home), with the page title and a close button. Nothing else on the canvas is drawn meanwhile. **Collapse** with the tab's close button, the same command again, Escape (when the canvas has the keyboard, not the page), a click on any space tab or Home, or by closing the pane. The page then rests in its pane as usual.

### Pop out

**Pop out** moves the whole pane into its own window: every tab, not just the active one. The window has the same header and a title bar drawn around the traffic lights; drag it by the empty part of row 1. Its size is the pane's size at 100% zoom, at least 640 × 480. The pane shows "Popped out" and a **Bring back** button.

Tabs, Back, Forward, Reload, the address field, the ⋯ menu and the Web Inspector all work on the window's pages. Sign-ins carry over, because the pane and the window share one website data store. The window annotates the same way the pane does.

**Bring back** (the pane's button, the window's **Close**, or its red traffic light) destroys the window's pages and puts the pane back as it was: the same tabs and active tab at the addresses they last had. Pages load again when you click them. Closing the pane closes its window.

## Cookies and site data

Browser panes and pop-outs share a website data store on disk for this Canopy home. Persistent cookies need macOS 14 or later.

- **Keep cookies: Until quit** (default) deletes the store when Canopy quits and again on the next start, including after a crash.
- **Keep cookies: Forever** keeps the store across restarts.
- **Clear browsing data** clears cookies, site storage and cache now, including in pop-outs. Sites will ask you to sign in again.

Change **Keep cookies** from a browser pane's **⋯** menu or under **Browser** in **Settings** > **Look**.

## Web Inspector

Select a browser pane and press `Ctrl+Shift+I` or `Cmd+Option+I` to open Web Inspector in its own window. Canopy grows the pane to a usable size and centers it at 100% zoom, or zooms out enough to fit a smaller canvas. Pinned panes stay where they are. On a popped-out pane it opens beside the window, which does not move.

If this version of WebKit cannot detach the inspector, it stays docked.

## Closing a page

Closing a pane, or the space it lives in, stops its page: media is paused and the page is unloaded.

A page you rested keeps running out of sight, so a video can keep playing. When a resting page that is off screen (panned away, or in another space) still has audio or video playing, Canopy shows a toast, "*page* is still playing", with a jump action that goes to the pane. It shows once, until you click the page again.

## Annotate

When a layout bug is easier to point at than describe, click **Annotate** in the header or the helper bar. It is one tool for pointing at an element, drawing on the page and sending the result to an agent. Canopy takes a still of the page while you work, so the page does not move under your pen. A popped-out page is annotated the same way in its own window.

The bar sits at the bottom of the pane. It has four icon tools and an **x** to stop:

| Tool | What it does |
|---|---|
| **Select** | Hover to see the element under the pointer, outlined with its tag and class. Click it to drop a numbered blue pin and open the comment box next to it. Drag from text to comment on a range, or drag elsewhere to comment on an area. |
| **Pen** | Draws freehand on the still. |
| **Box** | Draws a rectangle. |
| **Text** | Click, type a label, then press `Enter`. |

Colour and stroke width appear in the bar only while **Pen**, **Box** or **Text** is on (**Text** has no stroke). **Send** appears once something is queued. The bar does not move. `Cmd+Z` undoes and `Shift+Cmd+Z` redoes.

### The comment box

The comment box is a single line that grows when you press `Shift+Enter` for a new line. Press `Enter` to **Add** the comment to the queue, or `Cmd+Enter` to **Send** everything queued. Hover the round button beside the box to see both choices as a small menu. `Esc` closes the box and keeps the annotation. Click a pin later to edit its comment. Saving an edited comment with no text deletes it.

Hold `Space` to scroll or interact with the live page. Release it to take a fresh still. Element pins follow their elements. Area and text annotations keep their original page coordinates.

### Choose the agent

With exactly one agent wired to the pane, **Send** goes to it and there is nothing to choose. Otherwise a button with a bot icon appears in the header next to **Annotate**, labelled **Choose agent** until you pick one. It lists agents wired to the pane first, then the other running agents. **Send** with no agent chosen asks you to choose one first.

The agent receives the annotated screenshot, then the URL, viewport and numbered comments. Element comments include the selector, visible text, a short HTML excerpt and styles. Text labels are listed with their coordinates in the message.

Click the **x** or press `Esc` with no comment box open to leave. With marks or comments on the still, choose **Keep editing** or **Discard**.

Run `canvas.browser_pick` to start annotating with **Select** already on. It is the same tool, so there is no separate picker.

### How Canopy finds the source file

React development builds report each component's source file and starting line. An element comment lists them as `Component: PricingCard (src/components/PricingCard.tsx:42)`. Canopy turns the file the page reports into a path on disk by looking in this order: the folders of agents wired to the pane, the pane's space folder, then every space's folder. If it finds no file, the message keeps the path the page gave.

The fallback search skips dependency, version control and build folders, and stops after 50,000 entries.

## What agents can do with a browser pane

An agent wired to a browser pane can check its own work without asking you to look:

```sh
canopy pane read p2   # the URL, title and text of the page
canopy pane shot p2   # a PNG screenshot of the page
```

Reading an empty pane returns `New tab`. When the pane has a URL, both commands wait for the page to finish loading, time out after 30 seconds, and work even when the pane is off screen. See [Let Agents Show You](Let-Agents-Show-You#show-me-the-running-app).

## Helper bar

| Button | What it does |
|---|---|
| **Back** | Goes to the previous page. |
| **Reload** | Reloads the page. |
| **Home** | Goes to the pane's home address: the first address it opened, or the last one set with `canopy pane open`. |
| **Annotate** | Freezes the page for marks and comments. |
| **Screenshot** | Saves the page to `<temp>/canopy/<pane>.png` and shows the path in a toast. |
| **Open in** *your browser* | Opens the current address in your system browser. |
| **Expand**, **Pop out** | The same as in the header; Pop out reads **Bring back** while the pane is popped out. |
| **Pin to screen** / **Move to canvas** | Pins the pane to the window, or puts it back on the canvas. |
| **Connect**, **Close** | Draws a wire, or closes the pane. |

## Privacy

Browser panes are built for local development, and they keep to themselves:

- New tab is drawn by Canopy. The browser loads a page when you or an agent opens an address.
- Downloads are blocked.
- Links that ask for a new window open in the same pane.
- Annotate reads each element's React source through a bundled copy of an open-source tool. It starts only when you click an element, with its telemetry and its own toolbar turned off, and its version check, open-file link and web font import removed.
- Canopy keeps the last snapshot of each tab in `<app home>/browser/<pane>-<tab>.png` so the canvas can draw it after a restart. Closing the tab or pane deletes the snapshot.

See [Security and Privacy](Security-and-Privacy) for the app as a whole.

## Troubleshooting

- **The page is blank or did not load.** Check that the server is running, then click **Reload**.
- **The quote has no component or file.** The page is not a React development build, or Canopy could not find the file in any space folder. Wire the browser pane to an agent whose folder holds the source.

## See also

- [Point Agents at Things](Point-Agents-at-Things#a-complete-loop-fix-a-ui-bug): the full fix-a-UI-bug loop.
- [Agent Pane Commands](Agent-Pane-Commands): every `canopy pane` verb.
