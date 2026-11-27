//! Shared pane helper bars. See docs/wiki/Toolbar-and-Helper-Bar.md.
use super::*;

/// One button of a pane's helper bar: an icon running an app command.
pub(super) struct BarAction {
    pub icon: SharedString,
    pub label: SharedString,
    pub command: String,
    pub args: Value,
}

/// The helper bar buttons for a pane, left to right. Sub-agent cells have none. Sim
/// panes also pin to the screen (or unpin), and dock in the Files panel, before Connect.
/// Docked, the bar is the panel header's: it offers Move to canvas, and leaves out Expand and
/// Close (the tab closes the pane).
pub(super) fn pane_actions(pane: &canvas::Pane, hint: Option<&mods::PaneHint>, docked: bool, mode: Option<FileMode>) -> Vec<BarAction> {
    let mut actions = kind_actions(pane, hint);
    match &pane.kind {
        canvas::Kind::File { path } if file_view::can_preview(Path::new(path)) => {
            actions.insert(0, file_view::mode_action(path, file_view::file_host_mode(path, mode.unwrap_or_else(|| FileMode::default_for(Path::new(path)))), Some(&pane.id)));
        }
        canvas::Kind::Browser { source: Some(path), .. } => {
            actions.insert(0, file_view::mode_action(path, FileMode::Preview, Some(&pane.id)));
        }
        _ => {}
    }
    if let Some(path) = pane.path().filter(|path| cfg!(target_os = "macos") && preview_external_path(Path::new(path))) {
        let at = actions.iter().position(|a| a.command == "canvas.connect").unwrap_or(actions.len());
        actions.insert(at, BarAction { icon: "icons/external-link.svg".into(), label: "Open in Preview".into(), command: "preview.external".into(), args: json!({"path":path}) });
    }
    if matches!(pane.kind, canvas::Kind::Sim { .. }) {
        let at = actions.iter().position(|a| a.command == "canvas.connect").unwrap_or(actions.len());
        let dock = |icon: &str, label: &str, docked: bool| BarAction { icon: icon.to_owned().into(), label: label.to_owned().into(), command: "canvas.pane_dock".into(), args: json!({"docked":docked}) };
        let extra = if docked {
            vec![dock("icons/layout-grid.svg", "Move to canvas", false)]
        } else {
            let (icon, label, pinned) = if pane.pinned.is_some() { ("icons/pin-off.svg", "Unpin", false) } else { ("icons/pin.svg", "Pin to screen", true) };
            vec![BarAction { icon: icon.into(), label: label.into(), command: "canvas.pane_pin".into(), args: json!({"pinned":pinned}) }, dock("icons/panel-right.svg", "Open in panel", true)]
        };
        actions.splice(at..at, extra);
        if docked {
            actions.retain(|a| !matches!(a.command.as_str(), "canvas.sim_zoom" | "canvas.close"));
        }
    }
    actions
}

fn kind_actions(pane: &canvas::Pane, hint: Option<&mods::PaneHint>) -> Vec<BarAction> {
    match &pane.kind {
        canvas::Kind::Terminal { size } => vec![
            BarAction { icon: "icons/square-terminal.svg".into(), label: "Enter".into(), command: "canvas.enter".into(), args: json!({}) },
            if size.is_some() {
                BarAction { icon: "icons/minimize-2.svg".into(), label: "Collapse".into(), command: "canvas.collapse".into(), args: json!({}) }
            } else {
                BarAction { icon: "icons/maximize-2.svg".into(), label: "Expand".into(), command: "canvas.expand".into(), args: json!({}) }
            },
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ],
        canvas::Kind::File { path } => vec![
            BarAction { icon: "icons/panel-right.svg".into(), label: "Open as tab".into(), command: "preview.open".into(), args: json!({"path":path}) },
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ],
        canvas::Kind::Image { .. } => vec![
            BarAction { icon: "icons/quote.svg".into(), label: "Quote image".into(), command: "canvas.pane_quote".into(), args: json!({}) },
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ],
        canvas::Kind::Pdf { highlights, .. } => {
            let mut actions: Vec<_> = [
                ("icons/zoom-out.svg", "Zoom out", "canvas.pdf_zoom_out"),
                ("icons/scan.svg", "Reset zoom", "canvas.pdf_zoom_reset"),
                ("icons/zoom-in.svg", "Zoom in", "canvas.pdf_zoom_in"),
            ].into_iter().map(|(icon, label, command)| BarAction { icon: icon.into(), label: label.into(), command: command.into(), args: json!({}) }).collect();
            actions.extend([
                BarAction { icon: "icons/search.svg".into(), label: "Find".into(), command: "canvas.pdf_find".into(), args: json!({}) },
                BarAction { icon: "icons/highlighter.svg".into(), label: "Highlight text".into(), command: "canvas.pdf_highlight".into(), args: json!({}) },
                BarAction { icon: "icons/chevron-up.svg".into(), label: "Previous page".into(), command: "canvas.pdf_prev".into(), args: json!({}) },
                BarAction { icon: "icons/chevron-down.svg".into(), label: "Next page".into(), command: "canvas.pdf_next".into(), args: json!({}) },
            ]);
            if !highlights.is_empty() {
                actions.push(BarAction { icon: "icons/eraser.svg".into(), label: "Clear highlights".into(), command: "canvas.pdf_clear".into(), args: json!({}) });
            }
            actions.extend([
                BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
                BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
            ]);
            actions
        }
        canvas::Kind::Mod { .. } => hint.into_iter().flat_map(|h| &h.actions).map(|a| BarAction {
            icon: a.icon.clone().into(), label: a.label.clone().into(), command: a.command.clone(), args: a.args.clone(),
        }).chain([
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ]).collect(),
        canvas::Kind::Sim { udid } if udid.is_empty() => vec![
            BarAction { icon: "icons/smartphone.svg".into(), label: "Choose simulator".into(), command: "canvas.sim_device".into(), args: json!({}) },
            BarAction { icon: "icons/external-link.svg".into(), label: "Open in Device Hub".into(), command: "canvas.sim_hub".into(), args: json!({}) },
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({"pane":pane.id}) },
        ],
        canvas::Kind::Sim { .. } => [
            ("icons/zoom-out.svg", "Zoom out", "canvas.sim_zoom", json!({"step":"out"})),
            ("icons/scan.svg", "Actual size", "canvas.sim_zoom", json!({"step":"actual"})),
            ("icons/zoom-in.svg", "Zoom in", "canvas.sim_zoom", json!({"step":"in"})),
            ("icons/rotate-cw.svg", "Default size", "canvas.sim_zoom", json!({"step":"default"})),
            ("icons/picture-in-picture-2.svg", "Pop out", "canvas.sim_pop", json!({})),
            ("icons/smartphone.svg", "Choose simulator", "canvas.sim_device", json!({})),
            ("icons/external-link.svg", "Open in Device Hub", "canvas.sim_hub", json!({})),
            ("icons/link.svg", "Connect", "canvas.connect", json!({})),
            ("icons/x.svg", "Close", "canvas.close", json!({"pane":pane.id})),
        ].into_iter().map(|(icon, label, command, args)| BarAction { icon: icon.into(), label: label.into(), command: command.into(), args }).collect(),
        canvas::Kind::Browser { .. } | canvas::Kind::Sub { .. } => vec![],
    }
}

/// Shuts down the device belonging to this bar's pane.
#[cfg(target_os = "macos")]
pub(super) fn sim_shutdown_action(pane: &str) -> BarAction {
    BarAction {
        icon: "icons/square.svg".into(), label: "Shut down".into(),
        command: "canvas.sim_shutdown".into(), args: json!({"pane":pane}),
    }
}

/// Device actions shared by the simulator pane and its pop-out.
#[cfg(any(target_os = "macos", test))]
pub(super) fn sim_device_actions() -> Vec<BarAction> {
    [
        ("icons/house.svg", "Home", "canvas.sim_home"),
        ("icons/rotate-cw.svg", "Refresh", "canvas.sim_refresh"),
        ("icons/camera.svg", "Screenshot", "canvas.sim_screenshot"),
        ("icons/video.svg", "Record", "canvas.sim_record"),
        ("icons/pencil.svg", "Annotate", "canvas.sim_annotate"),
    ].into_iter().map(|(icon, label, command)| BarAction { icon: icon.into(), label: label.into(), command: command.into(), args: json!({}) }).collect()
}

/// Whether a pane's Connect button shows as active: its wire follows the pointer, or any edge
/// (drawn or born) touches it.
fn connect_active(id: &str, wiring: Option<&str>, edges: &[canvas::Wire]) -> bool {
    wiring == Some(id) || edges.iter().any(|w| w.a == id || w.b == id)
}

pub(super) fn bar_shell(s: f32) -> Div {
    div().flex().p(px(2. * s)).gap(px(2. * s)).rounded(px(ui::RADIUS))
        .bg(c("popover")).border_1().border_color(c("border")).shadow_md()
}

impl AppWindow {
    /// Keep floating bars out of the way during canvas gestures.
    pub(super) fn bars_hidden(&self) -> bool {
        let state = &self.canvas_state;
        state.drag.is_some() || state.cell_resize.is_some() || state.preview_drag.is_some()
            || state.space_drag.is_some() || state.marquee.is_some() || state.rest.is_some() || (state.wire.drag.is_some() && !state.wire.sticky)
            || state.region.is_some()
    }

    /// Describe the bars in placement priority: active wire, hover, then pane paint order.
    pub(super) fn helper_bars(&self, doc: &Document, cx: &mut Context<Self>) -> Vec<chrome::Control> {
        if self.bars_hidden() { return vec![]; }
        let state = &self.canvas_state;
        let wiring = state.wire.drag.as_ref().filter(|_| state.wire.sticky).map(|d| d.0.as_str());
        let edges = crate::wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        let mut targets: Vec<_> = doc.panes.iter().filter(|pane| {
            doc.shown(pane) && (wiring == Some(pane.id.as_str())
                || state.bar_hover.as_ref() == Some(&pane.id)
                || state.selected.contains(&pane.id))
        }).collect();
        targets.sort_by(|a, b| {
            let priority = |p: &canvas::Pane| (wiring == Some(p.id.as_str()),
                state.bar_hover.as_ref() == Some(&p.id), paint_order(p));
            priority(b).cmp(&priority(a)).then_with(|| a.id.cmp(&b.id))
        });

        targets.into_iter().filter_map(|pane| {
            let id = &pane.id;
            let host = self.pane_screen_bounds(pane)?;
            // The bar follows the pane's scale only between 75% and 100%, so it stays usable.
            let scale = canophy::layout(doc.camera, pane).scale.clamp(0.75, 1.0);
            let actions = self.helper_actions(pane, cx);
            if actions.is_empty() { return None; }
            let bar = self.bar(id, actions, scale, connect_active(id, wiring, &edges), cx)
                .when(canvas::header_height(&pane.kind) == 0., |d| {
                    d.items_center().child(self.note_move_grip(id, scale, cx)).child(self.pane_color_swatch(id, cx))
                })
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation());
            Some(chrome::Control { pane: id.clone(), host, port: None, element: bar.into_any_element() })
        }).collect()
    }

    fn helper_actions(&self, pane: &canvas::Pane, cx: &App) -> Vec<BarAction> {
        let hint = pane.mod_name().and_then(|name| self.folders.get(name)).and_then(|f| f.hint.pane.as_ref());
        let actions = pane_actions(pane, hint, false, pane.file().map(|path| self.canvas_state.file_mode(&FileHost::Card { pane: pane.id.clone() }, path)));
        #[cfg(target_os = "macos")]
        let actions = self.sim_relabel(&pane.id, actions, cx);
        #[cfg(not(target_os = "macos"))]
        let _ = cx;
        actions
    }

    /// Reuse helper actions when the pane is too crowded for a bar.
    pub(super) fn pane_helper_menu_items(&self, id: &str, cx: &App) -> Vec<Value> {
        self.canvas_state.doc.panes.iter().find(|p| p.id == id)
            .map(|p| self.helper_actions(p, cx).into_iter()
                .map(|a| json!([a.label.to_string(), a.command, a.args])).collect())
            .unwrap_or_default()
    }

    fn note_move_grip(&self, id: &str, s: f32, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let id = id.to_owned();
        div().id(SharedString::from(format!("note-grip-{id}"))).size(px(24. * s))
            .flex().items_center().justify_center().cursor(CursorStyle::OpenHand)
            .rounded(px(ui::RADIUS_SM)).hover(|d| d.bg(c("hover")))
            .child(svg().path("icons/grip-vertical.svg").size(px(14. * s)).text_color(c("muted-foreground")))
            .tooltip(strip_tooltip("Drag to move".into(), None))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                this.raise_pane(&id, cx);
                this.focus.focus(window, cx);
                this.canvas_state.selected = vec![id.clone()];
                this.canvas_state.wire.selected = None;
                this.start_preview_drag(&id, e.position);
                cx.notify();
                cx.stop_propagation();
            }))
    }

    /// One helper bar pill at scale `s`, positioned by its caller. `connected` highlights Connect.
    pub(super) fn bar(&self, id: &str, actions: Vec<BarAction>, s: f32, connected: bool, cx: &mut Context<Self>) -> Div {
        let state = &self.canvas_state;
        let mut bar = bar_shell(s).flex_nowrap();
        for BarAction { icon, label, command, args } in actions {
            let active = connected && command == "canvas.connect";
            let enabled = if command == "canvas.sim_zoom" && args["step"] == "actual" {
                #[cfg(target_os = "macos")]
                { state.sims.get(id).and_then(|v| v.live.as_ref()).is_some_and(|s| s.read(cx).live_points().is_some()) }
                #[cfg(not(target_os = "macos"))]
                { false }
            } else { true };
            let target = id.to_owned();
            let group = SharedString::from(format!("bar-{id}-{command}-{label}"));
            let hint = state.actions.as_ref().and_then(|keys| keys.iter()
                .find(|(section, _, action)| (section == "canvas-mode" || section == "layout:main") && action == &command)
                .map(|(_, key, _)| key.clone())).unwrap_or_default();
            bar = bar.child(div().id(group.clone()).group(group.clone()).role(gpui::Role::Button).aria_label(label.clone())
                .flex_none().h(px(24. * s)).min_w(px(24. * s)).when(hint.is_empty(), |d| d.w(px(24. * s)))
                .flex().items_center().justify_center().gap(px(2. * s)).rounded(px(ui::RADIUS_SM))
                .when(enabled, |d| d.cursor_pointer().hover(|d| d.bg(c("hover"))))
                .when(!enabled, |d| d.opacity(0.5).aria_description("Available when the simulator is live"))
                .when(active, |d| d.bg(c("hover")))
                .child(svg().path(icon).size(px(14. * s)).text_color(c(if active { "foreground" } else { "muted-foreground" }))
                    .when(enabled, |d| d.group_hover(group, |d| d.text_color(c("foreground")))))
                .when(!hint.is_empty(), |d| d.child(div().text_size(px(11.)).text_color(c("muted-foreground")).child(hint)))
                .tooltip(strip_tooltip(label.to_string(), Some(&command)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                    if !enabled { return; }
                    // Connect draws a wire with the pointer; the keyboard's `c` runs canvas.connect.
                    if command == "canvas.connect" { return this.wire_start(&target, e.position(), window, cx); }
                    this.focus.focus(window, cx);
                    this.canvas_state.selected = vec![target.clone()];
                    this.run_key(&command, args.clone(), window, cx);
                })));
        }
        bar
    }
}

#[cfg(test)]
mod helper_bar_tests {
    use super::{connect_active, pane_actions, sim_device_actions};
    use crate::canvas::{self, Wire};
    use serde_json::json;

    #[test]
    fn actions_follow_the_pane_kind() {
        let mut pane = canvas::Pane { id: "t1".into(), kind: canvas::Kind::Terminal { size: None }, rect: [0., 0., 220., 36.].into(), z: 1, space: None, color: None, pinned: None };
        let commands = |pane: &canvas::Pane| pane_actions(pane, None, false, None).into_iter().map(|a| a.command).collect::<Vec<_>>();
        assert_eq!(commands(&pane), ["canvas.enter", "canvas.expand", "canvas.connect", "canvas.close"]);
        *pane.size_mut().unwrap() = Some([800., 500.]);
        assert_eq!(commands(&pane)[1], "canvas.collapse");
        pane.kind = canvas::Kind::File { path: "/tmp/a.md".into() };
        assert_eq!(commands(&pane), ["file.mode", "preview.open", "canvas.connect", "canvas.close"]);
        assert_eq!(pane_actions(&pane, None, false, None)[0].args, json!({"path":"/tmp/a.md","mode":"source","pane":pane.id}));
        pane.kind = canvas::Kind::Mod { kind: "mod:note".into(), props: Default::default() };
        let hint = crate::mods::PaneHint { label: "Note".into(), icon: "mods/note/sticky-note.svg".into(), actions: vec![crate::mods::PaneAction { icon: "icons/quote.svg".into(), label: "Clear".into(), command: "canvas.pane_set".into(), args: json!({"props":{"text":""}}) }], ..Default::default() };
        assert_eq!(pane_actions(&pane, Some(&hint), false, None).into_iter().map(|a| a.command).collect::<Vec<_>>(), ["canvas.pane_set", "canvas.connect", "canvas.close"]);
        assert_eq!(commands(&pane), ["canvas.connect", "canvas.close"]);
        pane.kind = serde_json::from_value(json!({"kind":"browser","props":{"url":"http://localhost:3000","home":"http://localhost:3000"}})).unwrap();
        assert!(commands(&pane).is_empty());
        assert!(pane_actions(&pane, None, true, None).is_empty());
        pane.kind = canvas::Kind::Sim { udid: String::new() };
        assert_eq!(commands(&pane), ["canvas.sim_device", "canvas.sim_hub", "canvas.pane_pin", "canvas.pane_dock", "canvas.connect", "canvas.close"]);
        // Pinned, the same button unpins it.
        let pin = |pane: &canvas::Pane| pane_actions(pane, None, false, None).into_iter().find(|a| a.command == "canvas.pane_pin").map(|a| (a.label, a.args)).unwrap();
        assert_eq!(pin(&pane), ("Pin to screen".into(), json!({"pinned":true})));
        pane.pinned = Some([0., 0., 340., 700.]);
        assert_eq!(pin(&pane), ("Unpin".into(), json!({"pinned":false})));
        pane.pinned = None;
        // Docked, the bar moves it to the canvas and has no Expand, Pin or Close.
        let docked = pane_actions(&pane, None, true, None).into_iter().map(|a| (a.label.to_string(), a.command)).collect::<Vec<_>>();
        assert!(docked.contains(&("Move to canvas".to_owned(), "canvas.pane_dock".to_owned())));
        assert!(docked.iter().all(|(_, c)| c != "canvas.pane_pin" && c != "canvas.close"));
        pane.kind = canvas::Kind::Sim { udid: "U1".into() };
        assert_eq!(commands(&pane), ["canvas.sim_zoom", "canvas.sim_zoom", "canvas.sim_zoom", "canvas.sim_zoom", "canvas.sim_pop", "canvas.sim_device", "canvas.sim_hub", "canvas.pane_pin", "canvas.pane_dock", "canvas.connect", "canvas.close"]);
        #[cfg(target_os = "macos")]
        assert_eq!(super::sim_shutdown_action(&pane.id).args["pane"], pane.id);
        assert_eq!(sim_device_actions().into_iter().map(|a| a.command).collect::<Vec<_>>(),
            ["canvas.sim_home", "canvas.sim_refresh", "canvas.sim_screenshot", "canvas.sim_record", "canvas.sim_annotate"]);
        pane.kind = canvas::Kind::Sub { parent: "t1".into(), sub: "a".into() };
        assert!(pane_actions(&pane, None, false, None).is_empty());
    }
    #[test]
    fn viewer_actions_include_clear_only_when_there_are_marks() {
        let mut pane = canvas::Pane { id: "p1".into(), kind: canvas::Kind::Image { path: "/a.png".into() }, rect: [0.; 4].into(), z: 0, space: None, color: None, pinned: None };
        let commands = |pane: &canvas::Pane| pane_actions(pane, None, false, None).into_iter().map(|a| a.command).collect::<Vec<_>>();
        let image = commands(&pane);
        assert_eq!(image[0], "canvas.pane_quote");
        assert_eq!(image.iter().any(|c| c == "preview.external"), cfg!(target_os = "macos"));
        pane.kind = canvas::Kind::Pdf { path: "/a.pdf".into(), highlights: vec![] };
        let pdf = commands(&pane);
        assert_eq!(&pdf[..3], ["canvas.pdf_zoom_out", "canvas.pdf_zoom_reset", "canvas.pdf_zoom_in"]);
        assert_eq!(pdf.iter().any(|c| c == "preview.external"), cfg!(target_os = "macos"));
        assert!(!pdf.iter().any(|c| c == "canvas.pdf_clear"));
        if let canvas::Kind::Pdf { highlights, .. } = &mut pane.kind {
            highlights.push(canvas::PdfMark { page: 1, quote: "a".into(), occurrence: 0 });
        }
        assert!(commands(&pane).iter().any(|c| c == "canvas.pdf_clear"));
    }

    #[test]
    fn connect_is_active_while_wiring_or_wired() {
        let edges = [Wire { a: "t1".into(), b: "t2".into(), born: true }];
        assert!(connect_active("t2", None, &edges), "a born edge counts");
        assert!(connect_active("t3", Some("t3"), &edges), "its wire follows the pointer");
        assert!(!connect_active("t3", Some("t1"), &edges));
    }
}
