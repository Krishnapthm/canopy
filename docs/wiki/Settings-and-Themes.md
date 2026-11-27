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

## Quit and Keep Terminals Running

terminals and agents run in [herdr-core](herdr-core), a separate process. quitting Canopy closes its windows and leaves that work running. open Canopy again to return to those terminals.

on macOS, **Run in Background** is on by default. after you quit with live terminals, the Dock tile shows how many are running. click the tile or choose **Open Canopy** from its menu to reopen your windows. the tile goes away when the last terminal stops or the core exits.

to stop the core and every terminal and agent in it, open the background tile's menu, choose **Quit Completely**, and confirm. the Dock's regular **Quit** removes the tile and leaves the core running. you can also quit Canopy and run `hc server stop` from another terminal app using the same core session.

while Canopy is open, **Run in Background** shows its menu bar icon. choose **Show Canopy** to bring the window forward or **Quit Canopy** to exit the app.

the menu also lists the agent programs in your open spaces. for the local Codex CLI account, it shows used percentages and time until reset when the CLI provides them. open the menu again after **Refresh Usage** to see the latest result. cached results are rechecked when they are over a minute old and you open the menu. other programs show **Unavailable** when Canopy cannot read their usage.

turn off **Run in Background** in **Settings** > **Look** > **App**, or set `run_in_background` to `false` in `settings.json`, to remove the menu bar icon and skip the Dock tile after Quit.

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

Each layout keeps its own light and dark theme. Terminal programs that support color scheme reports can follow the terminal background when the theme changes. Programs with a fixed theme keep their own choice.
The four built-in Canopy and Herdr themes are **Canopy Dark**, **Canopy Light**, **Herdr Dark** and **Herdr Light**.
The default themes are **Canopy Dark** and **Canopy Light**.

Canopy keeps personal theme files in `themes/` in the app home.

## Errors in settings files

- **Invalid JSON:** A toast names the file, line and column. Canopy keeps the last good values for `settings.json`, `keys.json` and `programs.json`.
- **Invalid range:** A rejected value in `settings.json` raises an error toast. Canopy keeps the last good settings.
- **Unknown setting:** A warning toast names the key and may suggest a correction. The rest of `settings.json` still applies.
- **Retired setting:** Settings Canopy no longer reads load without a warning, so an older `settings.json` keeps working. `canvas_resize_terminals` is one: a terminal's grid now always refits when you resize its pane and never changes when you zoom (see [Terminals](Terminals)).

Errors appear in `settings.error`, `keys.error` or `programs.error` in the app home.
Correct the file and save it. A successful reload removes its `.error` file.
