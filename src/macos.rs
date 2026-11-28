// Modified for Canopy.
use crate::keys::CommandBinding;
use gpui::*;
use gpui_shell::action::ShellAction;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSColor, NSEventMask, NSEventTrackingRunLoopMode, NSFont, NSImage, NSMenu,
    NSMenuItem, NSProgressIndicator, NSProgressIndicatorStyle, NSTextField, NSView,
    NSStatusBar, NSStatusBarButton, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSPoint, NSRect, NSRunLoop, NSSize, NSString, NSTimer, ns_string};

actions!(herdr, [Quit, Hide, HideOthers, ShowAll, Settings, Minimize, Zoom, ToggleFullScreen]);

pub fn init(cx: &mut App, run_in_background: bool) {
    let status_item = create_status_item(cx, run_in_background);
    cx.set_global(BackgroundMode { enabled: run_in_background, status_item });
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

struct BackgroundMode {
    enabled: bool,
    status_item: Option<(Retained<NSStatusItem>, Retained<StatusTarget>)>,
}
impl Global for BackgroundMode {}

pub(crate) fn background_enabled(cx: &App) -> bool {
    cx.try_global::<BackgroundMode>().is_some_and(|mode| mode.enabled)
}

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
            let mut pending = Vec::new();
            let programs = crate::usage::programs();
            for (provider, name, executable) in &programs {
                crate::usage::request(*provider, executable, false);
                let item = unsafe { menu.addItemWithTitle_action_keyEquivalent(&NSString::from_str(name), None, ns_string!("")) };
                let reading = crate::usage::reading(*provider);
                usage_section(&item, name, &reading, mtm);
                if reading.pending { pending.push((item, *provider, name.clone())); }
            }
            if !programs.is_empty() {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
            unsafe {
                let refresh = menu.addItemWithTitle_action_keyEquivalent(ns_string!("Refresh Usage"), Some(sel!(refreshUsage:)), ns_string!(""));
                refresh.setTarget(Some(self));
                menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Show Canopy"), Some(sel!(showCanopy:)), ns_string!(""),
                ).setTarget(Some(self));
                menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Quit Canopy"), Some(sel!(quitCanopy:)), ns_string!(""),
                ).setTarget(Some(self));
            }
            let timer = if pending.is_empty() { None } else {
                let pending = std::cell::RefCell::new(pending);
                let update = block2::RcBlock::new(move |timer: std::ptr::NonNull<NSTimer>| {
                    let Some(mtm) = MainThreadMarker::new() else { return };
                    let mut pending = pending.borrow_mut();
                    pending.retain(|(item, provider, name)| {
                        let reading = crate::usage::reading(*provider);
                        if !reading.pending { usage_section(item, name, &reading, mtm); }
                        reading.pending
                    });
                    if pending.is_empty() {
                        // AppKit owns the timer throughout this callback.
                        unsafe { timer.as_ref() }.invalidate();
                    }
                });
                // Menu tracking runs in its own AppKit run-loop mode.
                let timer = unsafe { NSTimer::timerWithTimeInterval_repeats_block(0.1, true, &update) };
                unsafe { NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSEventTrackingRunLoopMode); }
                Some(timer)
            };
            menu.popUpMenuPositioningItem_atLocation_inView(None, NSPoint::new(0.0, 0.0), Some(button));
            if let Some(timer) = timer { timer.invalidate(); }
        }

        #[unsafe(method(refreshUsage:))]
        fn refresh_usage(&self, _: &NSObject) {
            for (provider, _, executable) in crate::usage::programs() {
                crate::usage::request(provider, &executable, true);
            }
        }

        #[unsafe(method(showCanopy:))]
        fn show_from_menu(&self, _: &NSObject) {
            self.show_canopy();
        }

        #[unsafe(method(quitCanopy:))]
        fn quit_canopy(&self, _: &NSObject) {
            self.ivars().update(|cx| cx.quit());
        }
    }
);

/// A native menu section with readable labels and a remaining-quota bar for each window.
fn usage_section(item: &NSMenuItem, name: &str, reading: &crate::usage::Reading, mtm: MainThreadMarker) {
    const WIDTH: f64 = 244.;
    let frame = |x, y, width, height| NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
    let height = 28. + 45. * reading.quotas.len() as f64 + if reading.message.is_some() { 20. } else { 0. };
    let view = NSView::initWithFrame(NSView::alloc(mtm), frame(0., 0., WIDTH, height));
    let label = |text: &str, y, size, weight| {
        let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        field.setFont(Some(&NSFont::systemFontOfSize_weight(size, weight)));
        field.setTextColor(Some(&NSColor::labelColor()));
        field.setFrame(frame(14., y, WIDTH - 28., 17.));
        view.addSubview(&field);
    };
    let mut y = height - 23.;
    label(name, y, 13., 0.3);
    for quota in &reading.quotas {
        label(&quota.label, y - 18., 12., 0.);
        label(&quota.detail(), y - 33., 11., 0.);
        let bar = NSProgressIndicator::initWithFrame(NSProgressIndicator::alloc(mtm), frame(14., y - 42., WIDTH - 28., 6.));
        bar.setStyle(NSProgressIndicatorStyle::Bar);
        bar.setIndeterminate(false);
        bar.setMinValue(0.);
        bar.setMaxValue(100.);
        bar.setDoubleValue(f64::from(quota.remaining));
        view.addSubview(&bar);
        y -= 45.;
    }
    if let Some(message) = &reading.message { label(message, 5., 11., 0.); }
    item.setView(Some(&view));
}

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
