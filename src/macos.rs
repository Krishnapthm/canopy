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
    let view = cx.try_global::<ActiveView>().map(|v| &v.0).cloned().unwrap_or_default();
    let mut menus = vec![
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
    ];
    menus.extend(crate::app_menu::groups(&view).into_iter().map(|(name, rows)| {
        let mut items: Vec<_> = rows.into_iter().map(|row| {
            if row.is_null() { return MenuItem::separator(); }
            let label = row[0].as_str().unwrap().to_owned();
            let command = row[1].as_str().unwrap();
            let item = if row[2].as_object().is_some_and(|args| !args.is_empty()) {
                let binding = CommandBinding { value: serde_json::json!({"command":command,"args":row[2]}).as_object().unwrap().clone() };
                MenuItem::action(label, binding)
            } else {
                let action = ShellAction::new(command.to_owned());
                match command {
                    "terminal.copy" => MenuItem::os_action(label, action, OsAction::Copy),
                    "terminal.paste" => MenuItem::os_action(label, action, OsAction::Paste),
                    "terminal.select_all" => MenuItem::os_action(label, action, OsAction::SelectAll),
                    _ => MenuItem::action(label, action),
                }
            };
            item.checked(row[3] == true)
        }).collect();
        if name == "View" { items.extend([MenuItem::separator(), MenuItem::action("Toggle Full Screen", ToggleFullScreen)]); }
        Menu::new(name).items(items)
    }));
    // Keep Help last; macOS also lists open windows in the Window menu.
    menus.insert(menus.len() - 1, Menu::new("Window").items([MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)]));
    cx.set_menus(menus);
}

struct ActiveView(crate::canvas::View);
impl Global for ActiveView {}

/// Updates the native view checkmark when a canvas view is loaded or changed.
pub fn set_view(view: &crate::canvas::View, cx: &mut App) {
    if cx.try_global::<ActiveView>().is_some_and(|v| &v.0 == view) { return; }
    cx.set_global(ActiveView(view.clone()));
    set_menus(cx);
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
