//! Shared pane helper bars. See docs/wiki/Toolbar-and-Helper-Bar.md.
use super::*;

/// One button of a pane's helper bar: an icon running an app command.
pub(super) struct BarAction {
    pub icon: SharedString,
    pub label: SharedString,
    pub command: String,
    pub args: Value,
}

/// The helper bar buttons for a pane, left to right. Sub-agent cells have none.
pub(super) fn pane_actions(pane: &canvas::Pane, hint: Option<&mods::PaneHint>) -> Vec<BarAction> {
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
            let mut actions = vec![
                BarAction { icon: "icons/search.svg".into(), label: "Find".into(), command: "canvas.pdf_find".into(), args: json!({}) },
                BarAction { icon: "icons/highlighter.svg".into(), label: "Highlight text".into(), command: "canvas.pdf_highlight".into(), args: json!({}) },
                BarAction { icon: "icons/chevron-up.svg".into(), label: "Previous page".into(), command: "canvas.pdf_prev".into(), args: json!({}) },
                BarAction { icon: "icons/chevron-down.svg".into(), label: "Next page".into(), command: "canvas.pdf_next".into(), args: json!({}) },
            ];
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
        canvas::Kind::Browser { .. } => [
            ("icons/arrow-left.svg", "Back", "canvas.browser_back"),
            ("icons/rotate-cw.svg", "Reload", "canvas.browser_reload"),
            ("icons/house.svg", "Home", "canvas.browser_home"),
            ("icons/globe.svg", "Open URL", "canvas.pane_open"),
            ("icons/crosshair.svg", "Pick an element to quote", "canvas.browser_pick"),
            ("icons/camera.svg", "Screenshot", "canvas.browser_shot"),
            ("icons/external-link.svg", "Open in browser", "canvas.browser_external"),
            ("icons/link.svg", "Connect", "canvas.connect"),
            ("icons/x.svg", "Close", "canvas.close"),
        ].into_iter().map(|(icon, label, command)| BarAction {
            icon: icon.into(), label: label.into(), command: command.into(),
            args: if command == "canvas.pane_open" { json!({"pane":pane.id}) } else { json!({}) },
        }).collect(),
        canvas::Kind::Sim { udid, .. } if udid.is_empty() => vec![
            BarAction { icon: "icons/smartphone.svg".into(), label: "Choose simulator".into(), command: "canvas.sim_device".into(), args: json!({}) },
            BarAction { icon: "icons/external-link.svg".into(), label: "Open in Device Hub".into(), command: "canvas.sim_hub".into(), args: json!({}) },
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ],
        canvas::Kind::Sim { .. } => [
            ("icons/house.svg", "Home", "canvas.sim_home", json!({})),
            ("icons/camera.svg", "Screenshot", "canvas.sim_screenshot", json!({})),
            ("icons/video.svg", "Record", "canvas.sim_record", json!({})),
            ("icons/pencil.svg", "Annotate", "canvas.sim_annotate", json!({})),
            ("icons/zoom-out.svg", "Zoom out", "canvas.sim_zoom", json!({"step":"out"})),
            ("icons/scan.svg", "Actual size", "canvas.sim_size", json!({})),
            ("icons/zoom-in.svg", "Zoom in", "canvas.sim_zoom", json!({"step":"in"})),
            ("icons/picture-in-picture-2.svg", "Pop out", "canvas.sim_pop", json!({})),
            ("icons/smartphone.svg", "Choose simulator", "canvas.sim_device", json!({})),
            ("icons/rotate-cw.svg", "Refresh", "canvas.sim_refresh", json!({})),
            ("icons/external-link.svg", "Open in Device Hub", "canvas.sim_hub", json!({})),
            ("icons/link.svg", "Connect", "canvas.connect", json!({})),
            ("icons/x.svg", "Close", "canvas.close", json!({})),
        ].into_iter().map(|(icon, label, command, args)| BarAction { icon: icon.into(), label: label.into(), command: command.into(), args }).collect(),
        canvas::Kind::Sub { .. } => vec![],
    }
}

/// Whether a pane's Connect button shows as active: its wire follows the pointer, or any edge
/// (drawn or born) touches it.
fn connect_active(id: &str, wiring: Option<&str>, edges: &[canvas::Wire]) -> bool {
    wiring == Some(id) || edges.iter().any(|w| w.a == id || w.b == id)
}

impl AppWindow {
    /// Helper bars for the hovered pane and every selected pane of the shown
    /// place: a pill of icon buttons above each pane's top-left corner.
    pub(super) fn helper_bars(&self, doc: &Document, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let state = &self.canvas_state;
        if state.drag.is_some() || state.cell_resize.is_some() || state.preview_drag.is_some()
            || state.space_drag.is_some() || state.marquee.is_some() || state.rest.is_some() || (state.wire.drag.is_some() && !state.wire.sticky)
            || state.region.is_some() {
            return vec![];
        }
        // A wire following the pointer keeps its pane's bar up, to show Connect as active.
        let wiring = state.wire.drag.as_ref().filter(|_| state.wire.sticky).map(|d| d.0.as_str());
        let edges = crate::wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        let mut targets: Vec<String> = wiring.map(str::to_owned).into_iter().collect();
        for id in &state.selected {
            if !targets.contains(id) { targets.push(id.clone()); }
        }
        if let Some(id) = &state.bar_hover {
            targets.retain(|k| k != id);
            targets.push(id.clone());
        }
        let s = doc.camera.zoom.clamp(0.75, 1.0);
        let bar_height = 28. * s + 2.;
        let mut out = vec![];
        for id in targets {
            let Some(pane) = doc.panes.iter().find(|p| p.id == id && doc.shows(p.space.as_ref())) else { continue };
            let hint = pane.mod_name().and_then(|name| self.folders.get(name)).and_then(|f| f.hint.pane.as_ref());
            let actions = pane_actions(pane, hint);
            if actions.is_empty() { continue; }
            let rect = if pane.boxed() { pane.rect } else {
                let Some(rect) = self.wire_cell(doc, &id) else { continue };
                rect
            };
            #[cfg(target_os = "macos")]
            let actions = self.sim_relabel(&id, actions, cx);
            let [x, y] = doc.camera.to_screen([rect[0], rect[1]]);
            out.push(self.bar(&id, actions, s, connect_active(&id, wiring, &edges), cx).absolute().left(px(x)).top(px(y - 8. - bar_height)).into_any_element());
        }
        out
    }

    /// One helper bar: a pill of icon buttons for pane `id`, at scale `s`. The canvas places
    /// it above the pane; a sim pane's pop-out window draws it too.
    /// One helper bar pill. `connected`: draw Connect as active (see `connect_active`).
    pub(super) fn bar(&self, id: &str, actions: Vec<BarAction>, s: f32, connected: bool, cx: &mut Context<Self>) -> Div {
        let state = &self.canvas_state;
        let mut bar = div().flex().p(px(2. * s)).gap(px(2. * s)).rounded(px(ui::RADIUS))
            .bg(c("popover")).border_1().border_color(c("border")).shadow_md();
        for BarAction { icon, label, command, args } in actions {
            let active = connected && command == "canvas.connect";
            let target = id.to_owned();
            let group = SharedString::from(format!("bar-{id}-{command}-{label}"));
            let hint = state.actions.as_ref().and_then(|keys| keys.iter()
                .find(|(section, _, action)| (section == "canvas-mode" || section == "layout:main") && action == &command)
                .map(|(_, key, _)| key.clone())).unwrap_or_default();
            bar = bar.child(div().id(group.clone()).group(group.clone()).role(gpui::Role::Button).aria_label(label.clone())
                .h(px(24. * s)).min_w(px(24. * s)).when(hint.is_empty(), |d| d.w(px(24. * s)))
                .flex().items_center().justify_center().gap(px(2. * s)).rounded(px(ui::RADIUS_SM)).cursor_pointer()
                .hover(|d| d.bg(c("hover"))).when(active, |d| d.bg(c("hover")))
                .child(svg().path(icon).size(px(14. * s)).text_color(c(if active { "foreground" } else { "muted-foreground" }))
                    .group_hover(group, |d| d.text_color(c("foreground"))))
                .when(!hint.is_empty(), |d| d.child(div().text_size(px(11.)).text_color(c("muted-foreground")).child(hint)))
                .tooltip(strip_tooltip(label.to_string(), Some(&command)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
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
    use super::{connect_active, pane_actions};
    use crate::canvas::{self, Wire};
    use serde_json::json;

    #[test]
    fn actions_follow_the_pane_kind() {
        let mut pane = canvas::Pane { id: "t1".into(), kind: canvas::Kind::Terminal { size: None }, rect: [0., 0., 220., 36.], z: 1, space: None };
        let commands = |pane: &canvas::Pane| pane_actions(pane, None).into_iter().map(|a| a.command).collect::<Vec<_>>();
        assert_eq!(commands(&pane), ["canvas.enter", "canvas.expand", "canvas.connect", "canvas.close"]);
        *pane.size_mut().unwrap() = Some([800., 500.]);
        assert_eq!(commands(&pane)[1], "canvas.collapse");
        pane.kind = canvas::Kind::File { path: "/tmp/a.md".into() };
        assert_eq!(commands(&pane), ["preview.open", "canvas.connect", "canvas.close"]);
        assert_eq!(pane_actions(&pane, None)[0].args, json!({"path":"/tmp/a.md"}));
        pane.kind = canvas::Kind::Mod { kind: "mod:note".into(), props: Default::default() };
        let hint = crate::mods::PaneHint { label: "Note".into(), icon: "mods/note/sticky-note.svg".into(), actions: vec![crate::mods::PaneAction { icon: "icons/quote.svg".into(), label: "Clear".into(), command: "canvas.pane_set".into(), args: json!({"props":{"text":""}}) }] };
        assert_eq!(pane_actions(&pane, Some(&hint)).into_iter().map(|a| a.command).collect::<Vec<_>>(), ["canvas.pane_set", "canvas.connect", "canvas.close"]);
        assert_eq!(commands(&pane), ["canvas.connect", "canvas.close"]);
        pane.kind = canvas::Kind::Browser { url: "http://localhost:3000".into(), home: "http://localhost:3000".into() };
        assert_eq!(commands(&pane), ["canvas.browser_back", "canvas.browser_reload", "canvas.browser_home", "canvas.pane_open", "canvas.browser_pick", "canvas.browser_shot", "canvas.browser_external", "canvas.connect", "canvas.close"]);
        pane.kind = canvas::Kind::Sim { udid: String::new(), mode: canvas::SimMode::Fixed, screen: None };
        assert_eq!(commands(&pane), ["canvas.sim_device", "canvas.sim_hub", "canvas.connect", "canvas.close"]);
        pane.kind = canvas::Kind::Sim { udid: "U1".into(), mode: canvas::SimMode::Fixed, screen: None };
        assert_eq!(commands(&pane)[..4], ["canvas.sim_home", "canvas.sim_screenshot", "canvas.sim_record", "canvas.sim_annotate"]);
        assert_eq!(commands(&pane).last().unwrap(), "canvas.close");
        pane.kind = canvas::Kind::Sub { parent: "t1".into(), sub: "a".into() };
        assert!(pane_actions(&pane, None).is_empty());
    }
    #[test]
    fn viewer_actions_include_clear_only_when_there_are_marks() {
        let mut pane = canvas::Pane { id: "p1".into(), kind: canvas::Kind::Image { path: "/a.png".into() }, rect: [0.; 4], z: 0, space: None };
        let commands = |pane: &canvas::Pane| pane_actions(pane, None).into_iter().map(|a| a.command).collect::<Vec<_>>();
        assert_eq!(commands(&pane), ["canvas.pane_quote", "canvas.connect", "canvas.close"]);
        pane.kind = canvas::Kind::Pdf { path: "/a.pdf".into(), highlights: vec![] };
        assert_eq!(commands(&pane), ["canvas.pdf_find", "canvas.pdf_highlight", "canvas.pdf_prev", "canvas.pdf_next", "canvas.connect", "canvas.close"]);
        if let canvas::Kind::Pdf { highlights, .. } = &mut pane.kind {
            highlights.push(canvas::PdfMark { page: 1, quote: "a".into(), occurrence: 0 });
        }
        assert_eq!(commands(&pane), ["canvas.pdf_find", "canvas.pdf_highlight", "canvas.pdf_prev", "canvas.pdf_next", "canvas.pdf_clear", "canvas.connect", "canvas.close"]);
    }

    #[test]
    fn connect_is_active_while_wiring_or_wired() {
        let edges = [Wire { a: "t1".into(), b: "t2".into(), born: true }];
        assert!(connect_active("t2", None, &edges), "a born edge counts");
        assert!(connect_active("t3", Some("t3"), &edges), "its wire follows the pointer");
        assert!(!connect_active("t3", Some("t1"), &edges));
    }
}
