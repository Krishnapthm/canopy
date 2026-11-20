# Settings and Themes

Use **Settings** to change Canopy's appearance and preferences.
Edit `settings.json` for settings without a control in the window.

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
