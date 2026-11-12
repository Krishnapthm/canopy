use crate::keys::CommandBinding;
use gpui::*;
use gpui_shell::action::ShellAction;

actions!(herdr, [Quit, Hide, HideOthers, ShowAll, Settings, Minimize, Zoom, ToggleFullScreen]);

pub fn init(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Settings, _| crate::mods::request(crate::mods::Request::Run { name: "app.settings".into(), args: serde_json::Value::Null }));
    cx.on_action(|_: &Minimize, cx| with_window(cx, Window::minimize_window));
    cx.on_action(|_: &Zoom, cx| with_window(cx, Window::zoom_window));
    cx.on_action(|_: &ToggleFullScreen, cx| with_window(cx, Window::toggle_fullscreen));
    // Bound before the first keys.json install, so they survive its reloads as kit keys.
    cx.bind_keys([KeyBinding::new("cmd-m", Minimize, None), KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None)]);
    set_menus(cx);
}

fn with_window(cx: &mut App, f: fn(&Window)) {
    if let Some(w) = cx.active_window() {
        let _ = w.update(cx, |_, window, _| f(window));
    }
}

/// The menu bar. Items run app commands, so they show the keys bound to them;
/// call again after the keymap changes.
pub fn set_menus(cx: &mut App) {
    let run = |name: &str, command: &str| MenuItem::action(name.to_owned(), ShellAction::new(command.to_owned()));
    let edit = |name: &str, command: &str, os: OsAction| MenuItem::os_action(name.to_owned(), ShellAction::new(command.to_owned()), os);
    let new_space = CommandBinding { value: serde_json::json!({"command": "canvas.create", "args": {"kind": "space"}}).as_object().unwrap().clone() };
    cx.set_menus([
        Menu::new("Canopy").items([
            MenuItem::action("Settings…", Settings),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Canopy", Hide),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Quit Canopy", Quit),
        ]),
        Menu::new("File").items([
            run("New Terminal", "canvas.create"),
            run("Open File...", "preview.open"),
            MenuItem::action("New Space…", new_space),
            MenuItem::separator(),
            run("Pop Out Panel", "panel.pop_out"),
            run("Close Panel", "panel.close"),
        ]),
        Menu::new("Edit").items([
            edit("Copy", "terminal.copy", OsAction::Copy),
            edit("Paste", "terminal.paste", OsAction::Paste),
            edit("Select All", "terminal.select_all", OsAction::SelectAll),
            MenuItem::separator(),
            run("Find", "terminal.search"),
        ]),
        Menu::new("View").items([
            run("Agents", "view.agents"),
            run("All", "view.all"),
            run("Terminals", "view.terminals"),
            MenuItem::separator(),
            run("Zoom In", "canvas.zoom_in"),
            run("Zoom Out", "canvas.zoom_out"),
            run("Actual Size", "canvas.zoom_reset"),
            run("Fit All", "canvas.fit"),
            MenuItem::separator(),
            run("Command Palette", "app.palette"),
            MenuItem::action("Toggle Full Screen", ToggleFullScreen),
        ]),
        // Named "Window", macOS also lists the open windows here.
        Menu::new("Window").items([MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)]),
        Menu::new("Help").items([run("Keyboard Shortcuts", "app.help"), run("Welcome", "app.welcome")]),
    ]);
}

pub fn action(name: &str) -> Option<Box<dyn Action>> {
    match name {
        "app.quit" => Some(Box::new(Quit)),
        "app.hide" => Some(Box::new(Hide)),
        "app.hide_others" => Some(Box::new(HideOthers)),
        "app.show_all" => Some(Box::new(ShowAll)),
        "app.settings" => Some(Box::new(Settings)),
        _ => None,
    }
}
