# Browser Pane

A browser pane shows a web page on the canvas, right next to the agent working on it. Its main use is your own app: put the dev server beside the agent, watch changes land, and when something looks wrong, click the element and quote it to the agent with its component and source file. Browser panes run on macOS only.

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

## Use the page

A browser pane is either resting or live:

- **Resting**, it shows a snapshot of the page. This keeps the canvas fast when you have several browsers open.
- **Click the page** to make it live. Scroll, click and type as in any browser. The page stays live while you pan, zoom or resize the pane. The canvas clips the page at its edges, and panes, helper bars and menus in front of the page remain visible and clickable.
- **Click outside the pane** to rest the page and take a fresh snapshot.

## The header

A browser pane's header is its controls, left to right:

- **Back**, **Forward** and **Reload**.
- The **address field**. Type an address and press Enter to open it (`localhost:3000` gets `http://`); this also sets the pane's **Home**. Press Escape to put the current address back. While you are not typing, the field follows the page.
- **⋯**, the menu below.

Drag the pane by the header's edges, or by the [helper bar](Toolbar-and-Helper-Bar).

### The ⋯ menu

| Item | What it does |
|---|---|
| **Open in default browser** | Opens the current address in your usual browser. |
| **Copy link** | Copies the address and shows a toast. |
| **Save screenshot** | Saves the page to `<temp>/canopy/<pane>.png` and shows the path in a toast. |
| **Viewport** | Choose **Responsive**, **1280 × 800**, **1024 × 768**, **Mobile 375 × 812**, or **Tablet 768 × 1024**. A check marks the saved choice. Responsive pages reflow as the pane resizes. Each fixed size sets the page's CSS viewport and scales it to fit, centered in the pane. |
| **Expand** | See below. |
| **Pop out** / **Bring back** | See below. |
| **Keep cookies** | Choose **Until quit** or **Forever**. A check marks the current choice. |
| **Clear browsing data** | After a confirmation, deletes the cookies, storage and cache of every browser pane. |

### Expand

**Expand** makes the page live and fills the whole canvas area with it, at 100% zoom. The top strip gets a tab for it, right after its space's tab (after Home for a pane on Home), with the page title and a close button. Nothing else on the canvas is drawn meanwhile. **Collapse** with the tab's close button, the same command again, Escape (when the canvas has the keyboard, not the page), a click on any space tab or Home, or by closing the pane. The page then rests in its pane as usual.

### Pop out

**Pop out** moves the page into its own window and loads the pane's current address. The pane shows "Popped out" and a **Bring back** button. The window has no element picker, and what you do in it is not tracked: bringing it back, or closing the window, leaves the pane at the address it had when you popped it, and the page loads again when you click it. Closing the pane closes its window.

## Cookies and site data

Browser panes and pop-outs share a website data store on disk for this Canopy home. Persistent cookies need macOS 14 or later.

- **Keep cookies: Until quit** (default) deletes the store when Canopy quits and again on the next start, including after a crash.
- **Keep cookies: Forever** keeps the store across restarts.
- **Clear browsing data** clears cookies, site storage and cache now, including for pop-outs. Sites will ask you to sign in again.

Change **Keep cookies** from a browser pane's **⋯** menu or under **Browser** in **Settings** > **Look**.

## Web Inspector

Select a browser pane and press `Ctrl+Shift+I` or `Cmd+Option+I` to open Web Inspector in its own window. Canopy grows the pane to a usable size and centers it at 100% zoom, or zooms out enough to fit a smaller canvas. Pinned panes stay where they are. Bring a popped-out page back before inspecting it.

If this version of WebKit cannot detach the inspector, it stays docked.

## Closing a page

Closing a pane, or the space it lives in, stops its page: media is paused and the page is unloaded.

A page you rested keeps running out of sight, so a video can keep playing. When a resting page that is off screen (panned away, or in another space) still has audio or video playing, Canopy shows a toast, "*page* is still playing", with a jump action that goes to the pane. It shows once, until you click the page again.

## Pick an element to quote

![A browser pane with the element picker on a button](images/browser-pane-1.png)

1. Click **Pick an element to quote** in the helper bar. The page becomes live.
2. Click the element you mean.
3. Pick an agent in the [quote menu](Quotes), optionally with a message.

The quote points at the element rather than copying its HTML. For a React development build, it names the component and its source file and lines, so the agent can go straight to the code:

```text
@http://localhost:3000/pricing <PricingCard> src/components/PricingCard.tsx:42-61 "<div class=\"card\">"
```

For other pages, it uses a CSS selector and the element's visible text:

```text
@https://example.com/ main > section:nth-of-type(2) "Buy now"
```

### How Canopy finds the source file

React development builds report each component's source file and starting line. Canopy then looks for that file, in this order: the folders of agents wired to the pane, the pane's space folder, then every space's folder. It scans the file for the element's closing tag to find the end line.

Some limits apply:

- The fallback search skips dependency, version control and build folders, and stops after 50,000 entries.
- Canopy scans files up to 512 KiB, and at most 400 lines for the closing tag. If it cannot find the end, the quote gives the start line only.
- Without source information, the visible text is cut at 200 characters. Opening tags are cut at 80 characters.

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
| **Home** | Goes to the pane's home address: the first address it opened, or the last one set with **Open URL** or `canopy pane open`. |
| **Open URL** | Focuses the address field so you can enter another address. |
| **Pick an element to quote** | Starts the element picker. |
| **Screenshot** | Saves the page to `<temp>/canopy/<pane>.png` and shows the path in a toast. |
| **Open in browser** | Opens the current address in your usual browser. |
| **Expand**, **Pop out** | The same as in the ⋯ menu; Pop out reads **Bring back** while the page is popped out. |
| **Connect**, **Close** | Draws a wire, or closes the pane. |

## Privacy

Browser panes are built for local development, and they keep to themselves:

- New tab is drawn by Canopy. The browser loads a page when you or an agent opens an address.
- Downloads are blocked.
- Links that ask for a new window open in the same pane.
- The element picker is a bundled copy of an open-source tool. It loads only when you pick, with its telemetry turned off and its version check, open-file link and web font import removed.
- Canopy keeps the last snapshot of each pane in `<app home>/browser/<pane>.png` so the canvas can draw it after a restart, and deletes it when you close the pane.

See [Security and Privacy](Security-and-Privacy) for the app as a whole.

## Troubleshooting

- **The page is blank or did not load.** Check that the server is running, then click **Reload**.
- **The quote has no component or file.** The page is not a React development build, or Canopy could not find the file in any space folder. Wire the browser pane to an agent whose folder holds the source.

## See also

- [Point Agents at Things](Point-Agents-at-Things#a-complete-loop-fix-a-ui-bug): the full fix-a-UI-bug loop.
- [Agent Pane Commands](Agent-Pane-Commands): every `canopy pane` verb.
