// Modified for Canopy.
//! The window's own title bar: a layout can hide the system's
//! (`"titlebar": "none"` on its root) and draw one from mods. Nodes with
//! `window_drag` move the window when dragged and maximize it on a
//! double-click; a mod's own control opts out by stopping the mouse-down.
//!
//! macOS: the title bar is always see-through and the app owns dragging.
//! With `system`, the app draws an empty strip for the window buttons; with
//! `none`, they sit over the layout where it says (`traffic_lights`).
//! Linux: decorations are asked for live; when the compositor leaves them to
//! us (as on GNOME Wayland), the app draws resize edges and system title bars.
//! Custom title bars draw their own buttons (`window()`).
//! Windows: the system's title bar is always hidden. With `system`, the app
//! draws a strip in the theme with Windows' own buttons (snap layouts work);
//! with `none`, mods draw the buttons, as on Linux.

use super::*;
use crate::layout::Titlebar;

/// How far the pointer moves before a press on a drag node moves the window.
const DRAG_START: f32 = 4.0;
/// The strip a `system` title bar leaves on macOS, px at zoom 1.
#[cfg(target_os = "macos")]
const MAC_STRIP: f32 = 28.0;
/// Where the window buttons sit by default (macOS), px.
#[cfg(target_os = "macos")]
const MAC_LIGHTS: [f32; 2] = [12.0, 7.0];
/// The grab band along the edges when the app resizes the window (Linux).
#[cfg(not(any(target_os = "macos", windows)))]
const EDGE: f32 = 6.0;
/// Windows' caption height and button width, px at zoom 1.
#[cfg(windows)]
const WIN_STRIP: f32 = 32.0;
#[cfg(windows)]
const WIN_BUTTON: f32 = 46.0;

impl AppWindow {
    pub(super) fn titlebar(&self) -> Titlebar {
        // Linux still asks for system decorations when the canvas supplies the client controls.
        // Windows keeps its separate caption strip.
        if cfg!(not(target_os = "macos")) && self.has_canvas() {
            return Titlebar::System;
        }
        self.chrome.titlebar.unwrap_or_default()
    }

    /// Makes the window match the shown layout's title bar. Cheap; runs on every render.
    pub(super) fn apply_titlebar(&mut self, window: &mut Window) {
        self.fullscreen = window.is_fullscreen();
        let want = (self.titlebar(), self.chrome.traffic_lights);
        if self.titlebar_applied == Some(want) {
            return;
        }
        self.titlebar_applied = Some(want);
        #[cfg(windows)]
        let _ = window;
        #[cfg(target_os = "macos")]
        {
            let [x, y] = match want {
                (Titlebar::Layout, Some(at)) => at,
                _ => MAC_LIGHTS,
            };
            window.set_traffic_light_position(point(px(x), px(y)));
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        window.request_decorations(match want.0 {
            Titlebar::System => WindowDecorations::Server,
            Titlebar::Layout => WindowDecorations::Client,
        });
    }

    /// What mods know about the window: `window()`.
    pub(super) fn window_json(&self, window: &Window) -> Value {
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(windows) {
            "windows"
        } else {
            "linux"
        };
        // Window buttons are the layout's to draw only where the system draws none.
        let buttons = if cfg!(windows) {
            self.titlebar() == Titlebar::Layout
        } else {
            !cfg!(target_os = "macos") && self.titlebar() == Titlebar::Layout
                && matches!(window.window_decorations(), Decorations::Client { .. })
        };
        json!({
            "platform": platform,
            "titlebar": if self.titlebar() == Titlebar::Layout { "none" } else { "system" },
            "buttons": buttons,
            "maximized": window.is_maximized(),
            "fullscreen": window.is_fullscreen(),
            "active": self.window_active,
            "hand": self.canvas_state.hand,
            "keys": self.canvas_state.actions.is_some(),
        })
    }

    /// Makes a node move the window when dragged and maximize it on a double-click.
    pub(super) fn window_drag(&self, el: Div, cx: &mut Context<Self>) -> Div {
        el.on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, window, _| {
            if ev.click_count == 2 {
                this.drag_from = None;
                #[cfg(target_os = "macos")]
                window.titlebar_double_click();
                #[cfg(windows)]
                win::toggle_maximize(window);
                #[cfg(not(any(target_os = "macos", windows)))]
                if window.window_controls().maximize {
                    window.zoom_window();
                }
            } else {
                this.drag_from = Some(ev.position);
            }
        }))
        .on_mouse_down(MouseButton::Right, cx.listener(|_, _ev: &MouseDownEvent, _window, _| {
            #[cfg(not(any(target_os = "macos", windows)))]
            if _window.window_controls().window_menu {
                _window.show_window_menu(_ev.position);
            }
        }))
    }

    /// The root's pointer moves: a press on a drag node that moved far enough moves the window.
    pub(super) fn drag_window(&mut self, ev: &MouseMoveEvent, window: &mut Window) {
        let Some(from) = self.drag_from else { return };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag_from = None;
            return;
        }
        let d = ev.position - from;
        if d.x.abs() > px(DRAG_START) || d.y.abs() > px(DRAG_START) {
            self.drag_from = None;
            #[cfg(windows)]
            win::start_move(window);
            #[cfg(not(windows))]
            window.start_window_move();
        }
    }

    /// Wraps the layout in what the window itself needs: on macOS a strip for the
    /// window buttons (system title bar), on Linux resize edges (client decorations).
    pub(super) fn window_frame(&self, tree: AnyElement, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(target_os = "macos")]
        {
            let _ = window;
            if self.titlebar() == Titlebar::System {
                let strip = self.window_drag(div().flex_none().h(px(MAC_STRIP)).w_full(), cx);
                return div().size_full().flex().flex_col().child(strip).child(div().flex_1().min_h_0().flex().child(tree)).into_any_element();
            }
            tree
        }
        #[cfg(windows)]
        {
            if self.titlebar() == Titlebar::Layout || window.is_fullscreen() {
                return tree;
            }
            let strip = win::strip(window.is_maximized(), cx);
            div().size_full().flex().flex_col().child(strip).child(div().flex_1().min_h_0().flex().child(tree)).into_any_element()
        }
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            if self.chrome.visible_blocks().iter().any(|node| matches!(node.kind, Kind::Canvas)) {
                return linux::resize_grips(tree, window);
            }
            let system = self.titlebar() == Titlebar::System;
            let menu = system.then(|| self.main_menu_button(cx));
            let close = cx.listener(|this, _: &(), window, cx| this.close_window(window, cx));
            linux::frame(tree, "Canopy".into(), system, menu, window, cx, move |window, cx| close(&(), window, cx))
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn main_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let bounds = self.bounds.clone();
        let measure = canvas(move |b, _, _| {
            bounds.borrow_mut().insert("main-menu-trigger".into(), b);
        }, |_, _, _, _| {}).absolute().size_full();
        let button = gpui_base::Button::new("primary-menu")
            .accessibility_label("Main Menu").size(px(32.)).rounded(px(6.))
            .hover(|s| s.bg(c("hover")))
            .tooltip(canvas_view::strip_tooltip("Main Menu".into(), Some("app.menu")))
            .child(svg().path("icons/menu.svg").size(px(16.)).text_color(c("foreground")))
            .on_click(cx.listener(|this, _, window, cx| {
                this.dispatch("app.menu", &json!({}), true, window, cx);
            }));
        div().relative().flex_none().child(measure).child(button)
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .into_any_element()
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn canvas_strip(&self, bar: Div, controls: Option<(Div, Div)>, cx: &mut Context<Self>) -> AnyElement {
        let content = div().flex().flex_1().min_w_0().h_full().items_center().justify_between()
            .when_some(controls, |row, (crumbs, actions)| row.child(crumbs).child(actions));
        linux::CanvasStrip {
            bar,
            content,
            menu: self.main_menu_button(cx),
            owner: cx.entity().downgrade(),
        }.into_any_element()
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn main_menu_anchor(&self) -> Option<Point<Pixels>> {
        if self.titlebar() != Titlebar::System {
            return None;
        }
        self.bounds.borrow().get("main-menu-trigger").map(Bounds::bottom_left)
    }

    /// Flushes state for both system and client close buttons.
    pub(super) fn prepare_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.drain_documents(window, cx);
        self.close_mods_now(std::time::Instant::now() + Duration::from_millis(250), window, cx);
    }

    fn close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        self.close_pop_outs(cx);
        self.prepare_close(window, cx);
        crate::place::save(window, cx);
        window.remove_window();
    }

    pub(super) fn window_command(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "window.close" => self.close_window(window, cx),
            "window.minimize" => window.minimize_window(),
            #[cfg(windows)]
            "window.maximize" => win::toggle_maximize(window),
            #[cfg(not(windows))]
            "window.maximize" => window.zoom_window(),
            _ => {}
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) mod linux {
    use super::*;

    /// Adds native desktop controls to the strip when Canopy supplies the window frame.
    #[derive(IntoElement)]
    pub(super) struct CanvasStrip {
        pub(super) bar: Div,
        pub(super) content: Div,
        pub(super) menu: AnyElement,
        pub(super) owner: WeakEntity<AppWindow>,
    }

    impl RenderOnce for CanvasStrip {
        fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
            let [left, right] = caption_buttons(window, cx, move |window, cx| {
                let _ = self.owner.update(cx, |this, cx| this.close_window(window, cx));
            });
            let mut bar = self.bar.pr(px(12.)).gap(px(4.));
            let menu_on_left = left.is_empty();
            let caption = |buttons: Vec<gpui_base::Button>| (!buttons.is_empty()).then(|| {
                div().flex().flex_none().items_center().gap(px(4.)).children(buttons)
                    .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            });
            bar = bar.children(caption(left));
            if menu_on_left {
                bar = bar.child(self.menu).child(self.content);
            } else {
                bar = bar.child(self.content).child(self.menu);
            }
            bar.children(caption(right))
        }
    }

    /// Builds both sides in desktop order, filtering buttons the compositor handles or disables.
    fn caption_buttons(
        window: &Window,
        cx: &App,
        close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> [Vec<gpui_base::Button>; 2] {
        if !matches!(window.window_decorations(), Decorations::Client { .. }) || window.is_fullscreen() {
            return [Vec::new(), Vec::new()];
        }
        let close = Rc::new(close);
        let controls = window.window_controls();
        let maximized = window.is_maximized();
        let layout = cx.button_layout().unwrap_or_else(WindowButtonLayout::linux_default);
        [layout.left, layout.right].map(|side| {
            side.into_iter().flatten().filter_map(|button| {
                let (label, icon): (_, &[u8]) = match button {
                    WindowButton::Minimize if controls.minimize =>
                        ("Minimize", include_bytes!("../../defaults/mods/tabs/win-min.svg")),
                    WindowButton::Maximize if controls.maximize && maximized =>
                        ("Restore", include_bytes!("../../defaults/mods/tabs/win-restore.svg")),
                    WindowButton::Maximize if controls.maximize =>
                        ("Maximize", include_bytes!("../../defaults/mods/tabs/win-max.svg")),
                    WindowButton::Close =>
                        ("Close Window", include_bytes!("../../defaults/mods/tabs/win-close.svg")),
                    WindowButton::Minimize | WindowButton::Maximize => return None,
                };
                let close = close.clone();
                Some(gpui_base::Button::new((ElementId::from("linux-caption"), button.id()))
                    .flex_none().size(px(28.)).rounded(px(14.)).accessibility_label(label)
                    .tooltip(canvas_view::strip_tooltip(label.into(), None))
                    .hover(|s| s.bg(c("hover")))
                    .child(svg().data(icon).size(px(14.)).text_color(c("foreground")))
                    .on_click(move |_, window, cx| match button {
                        WindowButton::Minimize => window.minimize_window(),
                        WindowButton::Maximize => window.zoom_window(),
                        WindowButton::Close => close(window, cx),
                    }))
            }).collect()
        })
    }

    #[cfg(test)]
    #[gpui::test]
    fn server_decorations_leave_caption_buttons_to_the_desktop(cx: &mut TestAppContext) {
        cx.add_window(|window, cx| {
            assert!(matches!(window.window_decorations(), Decorations::Server));
            assert!(caption_buttons(window, cx, |_, _| {}).iter().all(Vec::is_empty));
            Empty
        });
    }

    /// Adds the app header and the frame controls that the compositor leaves to us.
    pub(crate) fn frame(
        tree: AnyElement,
        title: SharedString,
        system: bool,
        menu: Option<AnyElement>,
        window: &mut Window,
        cx: &mut App,
        close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let client = matches!(window.window_decorations(), Decorations::Client { .. });
        let client_header = system && client && !window.is_fullscreen();
        let tree = if client_header || menu.is_some() {
            let [left, right] = if client_header {
                caption_buttons(window, cx, close)
            } else {
                [Vec::new(), Vec::new()]
            };
            let press = window.use_state(cx, |_, _| std::cell::Cell::new(None::<Point<Pixels>>));
            let center = div().flex_1().min_w_0().h_full().flex().items_center()
                .justify_center().overflow_hidden()
                .when(client_header, |bar| bar.child(div().truncate().text_size(px(13.)).child(title)));
            let header = div().flex().flex_none().h(px(36.)).w_full().items_center()
                .px(px(8.)).gap(px(4.)).bg(c("chrome"))
                .children(left)
                .child(drag_region(center, press.clone()))
                .children(menu)
                .children(right);
            div().size_full().flex().flex_col()
                .on_mouse_move(move |event, window, cx| {
                    let press = press.read(cx);
                    if event.pressed_button != Some(MouseButton::Left) {
                        press.set(None);
                    } else if let Some(from) = press.get() {
                        let delta = event.position - from;
                        if delta.x.abs() > px(DRAG_START) || delta.y.abs() > px(DRAG_START) {
                            press.set(None);
                            window.start_window_move();
                        }
                    }
                })
                .child(header)
                .child(div().flex_1().min_h_0().flex().child(tree))
                .into_any_element()
        } else {
            tree
        };
        resize_grips(tree, window)
    }

    fn drag_region(region: Div, press: Entity<std::cell::Cell<Option<Point<Pixels>>>>) -> Div {
        region
            .on_mouse_down(MouseButton::Left, {
                let press = press.clone();
                move |event, window, cx| {
                    if event.click_count == 2 {
                        press.read(cx).set(None);
                        if window.window_controls().maximize {
                            window.zoom_window();
                        }
                    } else {
                        press.read(cx).set(Some(event.position));
                    }
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let press = press.clone();
                move |_, _, cx| press.read(cx).set(None)
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| press.read(cx).set(None))
            .on_mouse_down(MouseButton::Right, |event, window, _| {
                if window.window_controls().window_menu {
                    window.show_window_menu(event.position);
                }
            })
    }

    pub(super) fn resize_grips(tree: AnyElement, window: &Window) -> AnyElement {
    let Decorations::Client { tiling } = window.window_decorations() else { return tree };
    if window.is_maximized() || window.is_fullscreen() {
        return tree;
    }
    let e = px(EDGE);
    let grip = |id: &'static str, edge: ResizeEdge, cursor: CursorStyle| {
        div()
            .id(id)
            .absolute()
            .occlude()
            .cursor(cursor)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(edge);
            })
    };
    let mut grips: Vec<AnyElement> = Vec::new();
    let (top, bottom, left, right) = (!tiling.top, !tiling.bottom, !tiling.left, !tiling.right);
    if top {
        grips.push(grip("rz-t", ResizeEdge::Top, CursorStyle::ResizeUpDown).top_0().left(e).right(e).h(e).into_any_element());
    }
    if bottom {
        grips.push(grip("rz-b", ResizeEdge::Bottom, CursorStyle::ResizeUpDown).bottom_0().left(e).right(e).h(e).into_any_element());
    }
    if left {
        grips.push(grip("rz-l", ResizeEdge::Left, CursorStyle::ResizeLeftRight).left_0().top(e).bottom(e).w(e).into_any_element());
    }
    if right {
        grips.push(grip("rz-r", ResizeEdge::Right, CursorStyle::ResizeLeftRight).right_0().top(e).bottom(e).w(e).into_any_element());
    }
    let corner = |id, edge, cursor| grip(id, edge, cursor).w(e * 2.).h(e * 2.);
    if top && left {
        grips.push(corner("rz-tl", ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight).top_0().left_0().into_any_element());
    }
    if top && right {
        grips.push(corner("rz-tr", ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft).top_0().right_0().into_any_element());
    }
    if bottom && left {
        grips.push(corner("rz-bl", ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft).bottom_0().left_0().into_any_element());
    }
    if bottom && right {
        grips.push(corner("rz-br", ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight).bottom_0().right_0().into_any_element());
    }
    div().size_full().relative().flex().child(tree).children(grips).into_any_element()
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsZoomed, PostMessageW, SC_MOVE, SW_MAXIMIZE, SW_RESTORE, ShowWindowAsync, WM_SYSCOMMAND};

    fn hwnd(window: &Window) -> Option<HWND> {
        match HasWindowHandle::window_handle(window).ok()?.as_raw() {
            RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
            _ => None,
        }
    }

    /// Hands the press to Windows' own move loop (snapping included). Posted,
    /// not sent: the loop runs after this event, outside GPUI's handler.
    pub(super) fn start_move(window: &Window) {
        let Some(h) = hwnd(window) else { return };
        // SC_MOVE + HTCAPTION: move with the mouse, not the keyboard.
        unsafe {
            ReleaseCapture();
            PostMessageW(h, WM_SYSCOMMAND, (SC_MOVE | 2) as usize, 0);
        }
    }

    /// GPUI's zoom only maximizes.
    pub(super) fn toggle_maximize(window: &Window) {
        let Some(h) = hwnd(window) else { return };
        unsafe {
            ShowWindowAsync(h, if IsZoomed(h) != 0 { SW_RESTORE } else { SW_MAXIMIZE });
        }
    }

    /// The `system` title bar: a strip in the theme, Windows' buttons on the
    /// right. Windows itself handles the areas (drag, snap layouts, double-click).
    pub(super) fn strip(maximized: bool, _cx: &mut Context<AppWindow>) -> AnyElement {
        let button = |id: &'static str, icon: &'static [u8], area: WindowControlArea, hover: Hsla, hover_text: Hsla| {
            div()
                .id(id)
                .group(id)
                .window_control_area(area)
                .w(settings::z(WIN_BUTTON))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .hover(move |s| s.bg(hover))
                .child(svg().data(icon).size(settings::z(16.0)).text_color(c("dim"))
                    .group_hover(id, move |s| s.text_color(hover_text)))
        };
        let red: Hsla = rgb(0xc42b1c).into();
        div()
            .flex_none()
            .h(settings::z(WIN_STRIP))
            .w_full()
            .flex()
            .child(div().flex_1().h_full().window_control_area(WindowControlArea::Drag))
            .child(button("win-min", include_bytes!("../../defaults/mods/tabs/win-min.svg"), WindowControlArea::Min, c("raised"), c("foreground")))
            .child(button("win-max", if maximized { include_bytes!("../../defaults/mods/tabs/win-restore.svg") } else { include_bytes!("../../defaults/mods/tabs/win-max.svg") }, WindowControlArea::Max, c("raised"), c("foreground")))
            .child(button("win-close", include_bytes!("../../defaults/mods/tabs/win-close.svg"), WindowControlArea::Close, red, gpui::white()))
            .into_any_element()
    }
}
