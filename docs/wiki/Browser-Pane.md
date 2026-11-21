# Browser Pane

A browser pane shows a web page on the canvas, right next to the agent working on it. Its main use is your own app: put the dev server beside the agent, watch changes land, and when something looks wrong, click the element and quote it to the agent with its component and source file. Browser panes run on macOS only.

## Open a page

1. Click **Browser** at the top right of the window.
2. Type an address in **Address**, such as `localhost:3000`.
3. Click **Open**.

The page opens pinned to the top right corner of the window, where it stays while you pan and zoom the canvas. Click **Move to canvas** on its helper bar to put it on the canvas with your other panes, or **Pin to screen** to pin it again.

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
- **Click the page** to make it live. Canopy pans the canvas so the whole pane is visible at the current zoom, and you can scroll, click and type as in any browser.
- **Click anywhere else** in the window to rest it again. Moving the pane or the canvas also rests it, after taking a fresh snapshot.

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

Both wait for the page to finish loading, time out after 30 seconds, and work even when the pane is off screen. See [Let Agents Show You](Let-Agents-Show-You#show-me-the-running-app).

## Helper bar

| Button | What it does |
|---|---|
| **Back** | Goes to the previous page. |
| **Reload** | Reloads the page. |
| **Home** | Goes to the pane's home address: the first address it opened, or the last one set with **Open URL** or `canopy pane open`. |
| **Open URL** | Asks for another address. |
| **Pick an element to quote** | Starts the element picker. |
| **Screenshot** | Saves the page to `<temp>/canopy/<pane>.png` and shows the path in a toast. |
| **Open in browser** | Opens the current address in your usual browser. |
| **Connect**, **Close** | Draws a wire, or closes the pane. |

## Privacy

Browser panes are built for local development, and they keep to themselves:

- Canopy loads only pages you or an agent open. There is no start page.
- Cookies and logins are kept in a private store in memory and forgotten when Canopy quits. Downloads are blocked.
- Links that ask for a new window open in the same pane.
- The element picker is a bundled copy of an open-source tool. It loads only when you pick, with its telemetry turned off and its version check, open-file link and web font import removed.
- Canopy keeps the last snapshot of each pane in `<app home>/browser/<pane>.png` so the canvas can draw it after a restart, and deletes it when you close the pane.

See [Security and Privacy](Security-and-Privacy) for the app as a whole.

## Troubleshooting

- **Clicking the page does not make it live.** Zoom out until the whole pane fits on screen, then click it.
- **The page is blank or did not load.** Check that the server is running, then click **Reload**.
- **The quote has no component or file.** The page is not a React development build, or Canopy could not find the file in any space folder. Wire the browser pane to an agent whose folder holds the source.

## See also

- [Point Agents at Things](Point-Agents-at-Things#a-complete-loop-fix-a-ui-bug): the full fix-a-UI-bug loop.
- [Agent Pane Commands](Agent-Pane-Commands): every `canopy pane` verb.
