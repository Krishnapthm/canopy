# Browser Pane

A browser pane shows a web page beside your agents. Browser panes run on macOS only.

## Open a page

1. Click **Browser** on the toolbar.
2. Type a URL in **Address**.
3. Click **Open**.

Only `http` and `https` pages open. Bare local hosts get `http://`. Other bare hosts get `https://`.

An agent can open a page or change an existing pane's URL:

```sh
canopy pane new browser localhost:3000
canopy pane open p2 http://localhost:3000/pricing  # also makes this URL Home
```

See [Agent Pane Commands](Agent-Pane-Commands) for `--wire`, `read` and `shot`.

Page reads require an agent or script. Browser `read` and `shot` wait for loading and work off screen.
The command times out after 30 seconds.

## Use the page

- At rest, the pane shows its last snapshot.
- Click the page to make it live and use it. Canopy pans to show the whole pane at the canvas zoom.
- Click elsewhere in the window to rest the page. Moving the pane or canvas also takes a snapshot and rests it.

## Pick an element

![A browser pane with the element picker on a button](images/browser-pane-1.png)

1. Click **Pick an element to quote** in the helper bar.
2. Click an element on the page.
3. Select an agent in the [quote menu](Quotes).

The quote points to the element. It does not copy the source code:

```text
@http://localhost:3000/pricing <PricingCard> src/components/PricingCard.tsx:42-61 "<div class=\"card\">"
@https://example.com/ main > section:nth-of-type(2) "Buy now"
```

- React development builds can supply the component, source file and line.
- Canopy searches wired agents' folders, the pane's space folder, then all space folders for the file.
- The fallback file search skips dependency, version control and build folders. It stops after 50,000 entries.
- Canopy scans files up to 512 KiB for the closing tag, with a limit of 400 lines.
- If the scan fails, the quote keeps the start line alone.
- Without source information, the quote uses a CSS selector and visible text, cut at 200 characters.
- Opening tags are cut at 80 characters.

## Helper bar

| Button | What it does |
|---|---|
| **Back** | Returns to the previous page. |
| **Reload** | Reloads the page. |
| **Home** | Opens the URL last entered with **Open URL** or `canopy pane open`. Initially, this is the pane's first URL. |
| **Open URL** | Asks for another address. |
| **Pick an element to quote** | Makes the page live and starts the picker. |
| **Screenshot** | Saves the page to `<temp>/canopy/<pane>.png`. A toast shows the path. |
| **Open in browser** | Opens the URL in your external browser. |
| **Connect**, **Close** | See [Wiring](Wiring) and [Panes](Panes). |

## Privacy

- Canopy loads only pages you or an agent open. There is no homepage.
- Cookies and logins use a private store in memory. Downloads are blocked.
- Links that request a new window open in the same pane.
- The bundled picker starts only when you pick, with telemetry disabled. Its version check, open-file link and Google Fonts import are removed.
- Canopy keeps snapshots in `<app home>/browser/<pane>.png` and removes them when you close the pane.

See [Security and Privacy](Security-and-Privacy) for the app's privacy rules.

## Troubleshooting

- **The page does not become live.** Zoom out until the whole pane fits the canvas, then click the page.
- **The page did not load.** Check that its server runs. Click **Reload** to try again.
