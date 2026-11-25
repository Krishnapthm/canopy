# Settings and Themes

Canopy has two places for preferences. The **Settings** window covers the common ones: appearance, themes, sounds and shortcuts. Everything else, such as canvas spacing or terminal sizes, lives in `settings.json` in your app home, which you (or an agent) can edit directly. Both apply changes immediately.

## Open settings

On macOS, choose **Settings** in the **Canopy** menu or press **Cmd+,**.
On Linux, choose **Settings** in the [Main Menu](Linux-Desktop) or press **Ctrl+,**.
On Windows, press **Ctrl+,**.
Changes save immediately.

![The Settings window](images/settings-and-themes-1.png)

Use **Settings** > **Shortcuts** to [change shortcuts](Keys-and-Mouse).

## Edit settings.json

1. Open `settings.json` in your [app home](Files-and-Folders).
2. Edit the JSON.
3. Save the file.

Changes apply live, except `core`, which takes effect at the next start.
Run this command for every setting, its default and its explanation:

```sh
canopy config
```

See [Canvas](Canvas) and [Terminals](Terminals) for their settings.

## Keep Canopy running on macOS

**Run in Background** keeps Canopy available while its windows are hidden, so you can leave your agents working and return to them later. It is on by default.

Press `Cmd+Q` or choose **Quit Canopy** to hide the app. Click the Canopy icon in the macOS menu bar and choose **Show Canopy** to bring the window forward, or **Quit Canopy** to exit. **Quit** in the Dock also exits the app.

The menu also lists the agent programs in your open spaces. For the local Codex CLI account, it shows used percentages and time until reset when the CLI provides them. Open the menu again after **Refresh Usage** to see the latest result. Canopy rechecks a cached result when it is over a minute old and you open the menu. Other programs show **Unavailable** when Canopy cannot read their usage.

Open **Settings** > **Look** > **App** to turn off **Run in Background**, or set `run_in_background` to `false` in `settings.json`. **Quit Canopy** then exits the app, and Canopy removes its menu bar icon.

## Alerts

Use **Settings** > **Alerts** to control how Canopy gets your attention while agents work. **Desktop notifications** is on by default in the macOS app. In `settings.json`, set `desktop_notifications` to `false` to turn it off. **Play sounds** controls Canopy's alert sounds independently.

Click **Notification style** to open macOS notification settings. Choose **Banners** for notifications that disappear automatically or **Alerts** for notifications that stay until you dismiss them.

To include an agent's message, enable terminal notifications in that agent's own settings. See [Agents](Agents#know-when-an-agent-needs-you) for when Canopy alerts you and where a click takes you.

## Browser

Open **Settings** > **Look** and scroll to **Browser**. The same settings are available in `settings.json`.

| Setting | Values |
|---|---|
| `browser_keep_cookies` | `until_quit` (default) clears cookies and site data at quit and on the next start. `forever` keeps them. |
| `browser_open_links` | `built_in` (initial setting) opens HTTP and HTTPS links clicked in terminals in a new browser tab. `default` opens them in the system browser. |

See [Cookies and site data](Browser-Pane#cookies-and-site-data) for data controls and [Links from terminals](Browser-Pane#links-from-terminals) for where new tabs open.

## Themes

1. Select **Look** in **Settings**.
2. Select **System**, **Light** or **Dark** under **Appearance**.
3. Click a theme under **Dark** or **Light**.

Each layout keeps its own light and dark theme.
The four built-in Canopy and Herdr themes are **Canopy Dark**, **Canopy Light**, **Herdr Dark** and **Herdr Light**.
The default themes are **Canopy Dark** and **Canopy Light**.

Canopy keeps personal theme files in `themes/` in the app home.

## Errors in settings files

- **Invalid JSON:** A toast names the file, line and column. Canopy keeps the last good values for `settings.json`, `keys.json` and `programs.json`.
- **Invalid range:** A rejected value in `settings.json` raises an error toast. Canopy keeps the last good settings.
- **Unknown setting:** A warning toast names the key and may suggest a correction. The rest of `settings.json` still applies.

Errors appear in `settings.error`, `keys.error` or `programs.error` in the app home.
Correct the file and save it. A successful reload removes its `.error` file.
