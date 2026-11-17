//! Shared pane helper bars. See docs/panes.md.
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
        canvas::Kind::Mod { .. } => hint.into_iter().flat_map(|h| &h.actions).map(|a| BarAction {
            icon: a.icon.clone().into(), label: a.label.clone().into(), command: a.command.clone(), args: a.args.clone(),
        }).chain([
            BarAction { icon: "icons/link.svg".into(), label: "Connect".into(), command: "canvas.connect".into(), args: json!({}) },
            BarAction { icon: "icons/x.svg".into(), label: "Close".into(), command: "canvas.close".into(), args: json!({}) },
        ]).collect(),
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
            || state.space_drag.is_some() || state.marquee.is_some() || state.rest.is_some() || (state.wire.drag.is_some() && !state.wire.sticky) {
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
            let [x, y] = doc.camera.to_screen([rect[0], rect[1]]);
            let mut bar = div().absolute().left(px(x)).top(px(y - 8. - bar_height))
                .flex().p(px(2. * s)).gap(px(2. * s)).rounded(px(ui::RADIUS))
                .bg(c("popover")).border_1().border_color(c("border")).shadow_md();
            for BarAction { icon, label, command, args } in actions {
                let active = command == "canvas.connect" && connect_active(&id, wiring, &edges);
                let target = id.clone();
                let group = SharedString::from(format!("bar-{id}-{command}"));
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
            out.push(bar.into_any_element());
        }
        out
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
        pane.kind = canvas::Kind::Sub { parent: "t1".into(), sub: "a".into() };
        assert!(pane_actions(&pane, None).is_empty());
    }

    #[test]
    fn connect_is_active_while_wiring_or_wired() {
        let edges = [Wire { a: "t1".into(), b: "t2".into(), born: true }];
        assert!(connect_active("t2", None, &edges), "a born edge counts");
        assert!(connect_active("t3", Some("t3"), &edges), "its wire follows the pointer");
        assert!(!connect_active("t3", Some("t1"), &edges));
    }
}
