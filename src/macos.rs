// Modified for Canopy.
use crate::keys::CommandBinding;
use gpui::*;
use gpui_shell::action::ShellAction;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSEventMask, NSFont, NSFontAttributeName, NSImage, NSMenu,
    NSStatusBar, NSStatusBarButton, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSAttributedString, NSData, NSDictionary, NSPoint, NSSize, NSString, ns_string};

actions!(herdr, [Quit, Hide, HideOthers, ShowAll, Settings, Minimize, Zoom, ToggleFullScreen]);

pub fn init(cx: &mut App, run_in_background: bool) {
    let status_item = create_status_item(cx, run_in_background);
    cx.set_global(BackgroundMode { enabled: run_in_background, status_item });
    cx.on_action(|_: &Quit, cx| quit_or_hide(cx));
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

struct BackgroundMode {
    enabled: bool,
    status_item: Option<(Retained<NSStatusItem>, Retained<StatusTarget>)>,
}
impl Global for BackgroundMode {}

/// Update the status item when a valid settings.json change is applied.
pub(crate) fn set_background_enabled(enabled: bool, cx: &mut App) {
    let Some(mode) = cx.try_global::<BackgroundMode>() else { return };
    if mode.enabled == enabled { return; }
    let mode = cx.global_mut::<BackgroundMode>();
    mode.enabled = enabled;
    if let Some((item, _)) = &mode.status_item {
        item.setVisible(enabled);
    }
}

/// Handle Canopy's Quit command and native application menu action.
pub(crate) fn quit_or_hide(cx: &mut App) {
    if cx.try_global::<BackgroundMode>().is_some_and(|mode| mode.enabled) {
        cx.hide();
    } else {
        cx.quit();
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = AsyncApp]
    struct StatusTarget;

    impl StatusTarget {
        #[unsafe(method(statusClick:))]
        fn status_click(&self, button: &NSStatusBarButton) {
            let mtm = self.mtm();
            let menu = NSMenu::new(mtm);
            let programs = crate::mods::usage_programs();
            for (key, name, executable, local) in &programs {
                let rows = if key == "codex" && *local {
                    crate::usage::codex(executable, false)
                } else {
                    vec!["Unavailable".into()]
                };
                unsafe {
                    let label = if key == "codex" && *local { format!("{name} · Local CLI account") } else { name.clone() };
                    menu.addItemWithTitle_action_keyEquivalent(&NSString::from_str(&label), None, ns_string!("")).setEnabled(false);
                    let font = NSFont::monospacedSystemFontOfSize_weight(12., 0.);
                    let attrs = NSDictionary::<NSString, AnyObject>::from_slices(&[NSFontAttributeName], &[&font]);
                    for row in rows {
                        let title = NSString::from_str(&row);
                        let item = menu.addItemWithTitle_action_keyEquivalent(&title, None, ns_string!(""));
                        item.setIndentationLevel(1);
                        item.setAttributedTitle(Some(&NSAttributedString::initWithString_attributes(
                            NSAttributedString::alloc(), &title, Some(&attrs),
                        )));
                        item.setEnabled(false);
                    }
                }
            }
            unsafe {
                if programs.iter().any(|(key, _, _, local)| key == "codex" && *local) {
                    menu.addItemWithTitle_action_keyEquivalent(ns_string!("Refresh Usage"), Some(sel!(refreshUsage:)), ns_string!("")).setTarget(Some(self));
                }
                menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Show Canopy"), Some(sel!(showCanopy:)), ns_string!(""),
                ).setTarget(Some(self));
                menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Quit Canopy"), Some(sel!(quitCompletely:)), ns_string!(""),
                ).setTarget(Some(self));
            }
            menu.popUpMenuPositioningItem_atLocation_inView(None, NSPoint::new(0.0, 0.0), Some(button));
        }

        #[unsafe(method(refreshUsage:))]
        fn refresh_usage(&self, _: &NSObject) {
            if let Some((_, _, executable, _)) = crate::mods::usage_programs().into_iter()
                .find(|(key, _, _, local)| key == "codex" && *local)
            {
                crate::usage::codex(&executable, true);
            }
        }

        #[unsafe(method(showCanopy:))]
        fn show_from_menu(&self, _: &NSObject) {
            self.show_canopy();
        }

        #[unsafe(method(quitCompletely:))]
        fn quit_completely(&self, _: &NSObject) {
            self.ivars().update(|cx| cx.quit());
        }
    }
);

impl StatusTarget {
    fn show_canopy(&self) {
        NSApplication::sharedApplication(self.mtm()).unhide(None);
        self.ivars().update(|cx| {
            cx.activate(true);
            let windows = cx.windows();
            // Bring the main window forward even if a pop-out was last active.
            if let Some(w) = windows.iter().find(|w| w.downcast::<crate::window::AppWindow>().is_some())
                .or_else(|| windows.first())
            {
                let _ = w.update(cx, |_, window, _| window.activate_window());
            }
        });
    }
}

fn create_status_item(
    cx: &App,
    visible: bool,
) -> Option<(Retained<NSStatusItem>, Retained<StatusTarget>)> {
    let mtm = MainThreadMarker::new().expect("macOS status item runs on the main thread");
    let data = NSData::with_bytes(include_bytes!("../assets/icons/canopy-status.pdf"));
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    image.setSize(NSSize::new(18.0, 18.0 * 832.0 / 892.0));
    image.setTemplate(true);
    image.setAccessibilityDescription(Some(ns_string!("Canopy")));
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let button = item.button(mtm)?;
    let target: Retained<StatusTarget> = unsafe {
        msg_send![super(StatusTarget::alloc(mtm).set_ivars(cx.to_async())), init]
    };
    button.setImage(Some(&image));
    button.setToolTip(Some(ns_string!("Canopy")));
    unsafe {
        button.setTarget(Some(&target));
        button.setAction(Some(sel!(statusClick:)));
    }
    button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
    item.setVisible(visible);
    Some((item, target))
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
            MenuItem::action("Settings", Settings),
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
