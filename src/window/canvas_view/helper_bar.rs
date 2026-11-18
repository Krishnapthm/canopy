//! Shared pane helper bars. See docs/panes.md.
use super::*;

/// One button of a pane's helper bar: an icon running an app command.
pub(super) struct BarAction {
    pub icon: &'static str,
    pub label: &'static str,
    pub command: &'static str,
    pub args: Value,
}

/// The helper bar buttons for a pane, left to right. Sub-agent cells have none.
pub(super) fn pane_actions(pane: &canvas::Pane) -> Vec<BarAction> {
    match &pane.kind {
        canvas::Kind::Terminal { size } => vec![
            BarAction { icon: "icons/square-terminal.svg", label: "Enter", command: "canvas.enter", args: json!({}) },
            if size.is_some() {
                BarAction { icon: "icons/minimize-2.svg", label: "Collapse", command: "canvas.collapse", args: json!({}) }
            } else {
                BarAction { icon: "icons/maximize-2.svg", label: "Expand", command: "canvas.expand", args: json!({}) }
            },
            BarAction { icon: "icons/link.svg", label: "Connect", command: "canvas.connect", args: json!({}) },
            BarAction { icon: "icons/x.svg", label: "Close", command: "canvas.close", args: json!({}) },
        ],
        canvas::Kind::File { path } => vec![
            BarAction { icon: "icons/panel-right.svg", label: "Open as tab", command: "preview.open", args: json!({"path":path}) },
            BarAction { icon: "icons/link.svg", label: "Connect", command: "canvas.connect", args: json!({}) },
            BarAction { icon: "icons/x.svg", label: "Close", command: "canvas.close", args: json!({}) },
        ],
        canvas::Kind::Sub { .. } => vec![],
    }
}

impl AppWindow {
    /// Helper bars for the hovered pane and every selected pane of the shown
    /// place: a pill of icon buttons above each pane's top-left corner.
    pub(super) fn helper_bars(&self, doc: &Document, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let state = &self.canvas_state;
        if state.drag.is_some() || state.cell_resize.is_some() || state.preview_drag.is_some()
            || state.space_drag.is_some() || state.marquee.is_some() || state.rest.is_some() || state.wire.drag.is_some() {
            return vec![];
        }
        let mut targets = vec![];
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
            let actions = pane_actions(pane);
            if actions.is_empty() { continue; }
            let rect = if pane.file().is_some() { pane.rect } else {
                let Some(rect) = self.wire_cell(doc, &id) else { continue };
                rect
            };
            let [x, y] = doc.camera.to_screen([rect[0], rect[1]]);
            let mut bar = div().absolute().left(px(x)).top(px(y - 8. - bar_height))
                .flex().p(px(2. * s)).gap(px(2. * s)).rounded(px(ui::RADIUS))
                .bg(c("popover")).border_1().border_color(c("border")).shadow_md();
            for BarAction { icon, label, command, args } in actions {
                let target = id.clone();
                let group = SharedString::from(format!("bar-{id}-{command}"));
                let hint = state.actions.as_ref().and_then(|keys| keys.iter()
                    .find(|(section, _, action)| (section == "canvas-mode" || section == "layout:main") && action == command)
                    .map(|(_, key, _)| key.clone())).unwrap_or_default();
                bar = bar.child(div().id(group.clone()).group(group.clone()).role(gpui::Role::Button).aria_label(label)
                    .h(px(24. * s)).min_w(px(24. * s)).when(hint.is_empty(), |d| d.w(px(24. * s)))
                    .flex().items_center().justify_center().gap(px(2. * s)).rounded(px(ui::RADIUS_SM)).cursor_pointer()
                    .hover(|d| d.bg(c("hover")))
                    .child(svg().path(icon).size(px(14. * s)).text_color(c("muted-foreground"))
                        .group_hover(group, |d| d.text_color(c("foreground"))))
                    .when(!hint.is_empty(), |d| d.child(div().text_size(px(11.)).text_color(c("muted-foreground")).child(hint)))
                    .tooltip(strip_tooltip(label.to_string(), Some(command)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.focus.focus(window, cx);
                        this.canvas_state.selected = vec![target.clone()];
                        this.run_key(command, args.clone(), window, cx);
                    })));
            }
            out.push(bar.into_any_element());
        }
        out
    }
}

#[cfg(test)]
mod helper_bar_tests {
    use super::pane_actions;
    use crate::canvas;
    use serde_json::json;

    #[test]
    fn actions_follow_the_pane_kind() {
        let mut pane = canvas::Pane { id: "t1".into(), kind: canvas::Kind::Terminal { size: None }, rect: [0., 0., 220., 36.], z: 1, space: None };
        let commands = |pane: &canvas::Pane| pane_actions(pane).iter().map(|a| a.command).collect::<Vec<_>>();
        assert_eq!(commands(&pane), ["canvas.enter", "canvas.expand", "canvas.connect", "canvas.close"]);
        *pane.size_mut().unwrap() = Some([800., 500.]);
        assert_eq!(commands(&pane)[1], "canvas.collapse");
        pane.kind = canvas::Kind::File { path: "/tmp/a.md".into() };
        assert_eq!(commands(&pane), ["preview.open", "canvas.connect", "canvas.close"]);
        assert_eq!(pane_actions(&pane)[0].args, json!({"path":"/tmp/a.md"}));
        pane.kind = canvas::Kind::Sub { parent: "t1".into(), sub: "a".into() };
        assert!(pane_actions(&pane).is_empty());
    }
}
