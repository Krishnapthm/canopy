# Settings and Themes

Canopy has two places for preferences. The **Settings** window covers the common ones: appearance, themes, sounds and shortcuts. Everything else, such as canvas spacing or terminal sizes, lives in `settings.json` in your app home, which you (or an agent) can edit directly. Both apply changes immediately.

## Open settings

Press `Cmd+,` (`Ctrl+,` on Linux and Windows).
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
