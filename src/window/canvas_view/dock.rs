//! Browser and simulator panes docked in the Files panel. docs/wiki/Files-Panel.md explains them.
//!
//! A pane is docked exactly when a `PanelTab::Pane` names it (`Document::docked`). A docked pane
//! keeps its rect for the trip back to the canvas but is skipped by drawing, layout and wires. The
//! panel draws the same body `pane_card` would, under a header made of the shared helper bar.
use super::*;
use crate::canvas::PanelTab;

/// Where `canvas.pane_new` puts a pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Home {
    Panel,
    Canvas,
    Pinned,
}

impl Home {
    /// Reads `place` (and the older `pinned: true`). Browsers and simulators open docked unless
    /// told otherwise; every other kind opens on the canvas and cannot be docked.
    pub(super) fn parse(a: &Value, kind: &str) -> Result<Self, String> {
        let dockable = matches!(kind, "browser" | "sim");
        let home = match &a["place"] {
            Value::Null if a["pinned"].as_bool() == Some(true) => Home::Pinned,
            Value::Null if dockable => Home::Panel,
            Value::Null => Home::Canvas,
            Value::String(place) => match place.as_str() {
                "panel" => Home::Panel,
                "canvas" => Home::Canvas,
                "pinned" => Home::Pinned,
                other => return Err(format!("canvas.pane_new: place must be panel, canvas or pinned, not {other}")),
            },
            _ => return Err("canvas.pane_new: place must be a string".into()),
        };
        if home == Home::Panel && !dockable {
            return Err("canvas.pane_new: only browser and sim panes open in the panel".into());
        }
        Ok(home)
    }
}

impl AppWindow {
    /// The docked pane's layout in the panel body, with that body's window bounds. None unless
    /// the pane is the panel's active tab and its body has been measured.
    pub(super) fn docked_view(&self, id: &str) -> Option<(canophy::Layout, Bounds<Pixels>)> {
        let body = self.docked_bounds(id)?;
        Some((canophy::Layout::docked([f32::from(body.size.width), f32::from(body.size.height)]), body))
    }

    /// The measured body of the panel's active pane.
    pub(super) fn docked_bounds(&self, id: &str) -> Option<Bounds<Pixels>> {
        if !self.has_canvas() || self.canvas_state.active_panel_pane() != Some(id) { return None; }
        self.bounds.borrow().get(&format!("panel-body:{id}")).copied()
            .filter(|b| b.size.width > px(0.) && b.size.height > px(0.))
    }

    /// Docks a pane in the Files panel and shows it there when the editor has no unsaved changes.
    pub(super) fn dock_pane(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        // A live page rests first, so it wakes again at its new place.
        if self.canvas_state.browser_tab.as_deref() == Some(id) {
            self.collapse_browser(window, cx);
        } else {
            self.rest_live_browser(id, cx);
        }
        let state = &mut self.canvas_state;
        state.doc.dock(id);
        if state.dirty.is_none() {
            state.file = Some(PanelTab::Pane(id.to_owned()));
            state.selected = vec![id.to_owned()];
        }
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// Puts a docked pane back on the canvas at its saved rect (or the view's center), and makes
    /// room for it.
    pub(super) fn undock_pane(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.rest_live_browser(id, cx);
        if let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id)
            && pane.space != self.canvas_state.doc.room {
            self.go_room(pane.space.clone(), window, cx);
        }
        let center = self.view_center(window);
        let state = &mut self.canvas_state;
        let was_active = state.active_panel_pane() == Some(id);
        state.step(canophy::Intent::Undock { id: id.to_owned(), center });
        state.panel_expanded = false;
        if was_active { state.file = state.doc.panel.tabs.last().cloned(); }
        self.raise_pane(id, cx);
        self.solve_layout(Some(id), None, cx);
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// Closes a panel tab: a file tab goes away, a pane tab closes its pane.
    pub(in crate::window) fn close_panel_tab(&mut self, tab: &PanelTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = tab.file().filter(|path| self.canvas_state.dirty.as_ref() == Some(*path)) {
            self.open_overlay(mods::new_overlay_id(), json!({"mod":"confirm","kind":"dialog","width":400,
                "props":{"title":"Discard unsaved changes?","detail":"Your changes to this file will be lost.","destructive":true,
                "accept":"Discard changes","command":{"name":"panel.close_tab","args":{"path":path,"confirmed":true,"generation":self.canvas_state.dirty_generation}}}}), window, cx);
            return;
        }
        let state = &mut self.canvas_state;
        match tab {
            PanelTab::File(path) => {
                state.modes.remove(path);
                if !state.doc.panes.iter().any(|pane| pane.file() == Some(path)) {
                    state.previews.remove(path);
                }
                state.file = canvas::close_panel_tab(&mut state.doc.panel.tabs, state.file.as_ref(), tab);
            }
            PanelTab::Pane(id) => self.close_boxed_panes(&[id.clone()], cx),
        }
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// The first wired agent, in wire order. Boxed panes have only drawn wires.
    fn wired_agent(&self, id: &str) -> Option<String> {
        crate::wire::neighbors(&self.canvas_state.doc.wires, id).into_iter()
            .find(|other| self.live.get(other).is_some_and(canvas_agent))
    }

    /// A pane tab's icon, title and color token for its wired agent's dot.
    pub(super) fn docked_tab(&self, id: &str, cx: &App) -> (&'static str, String, Option<String>) {
        let doc = &self.canvas_state.doc;
        let Some(pane) = doc.panes.iter().find(|p| p.id == id) else { return ("icons/x.svg", id.to_owned(), None) };
        let (icon, title) = match &pane.kind {
            canvas::Kind::Browser { .. } => ("icons/globe.svg", self.browser_title(pane).0),
            canvas::Kind::Sim { .. } => ("icons/smartphone.svg", self.sim_title(pane, cx).0),
            canvas::Kind::Terminal { .. } | canvas::Kind::Sub { .. } | canvas::Kind::File { .. }
            | canvas::Kind::Pdf { .. } | canvas::Kind::Image { .. } | canvas::Kind::Mod { .. } => {
                return ("icons/x.svg", id.to_owned(), None);
            }
        };
        let tint = self.wired_agent(id).map(|agent| {
            doc.effective_color(&agent).map_or_else(|| "foreground".into(), canvas::Hue::tint_token)
        });
        (icon, title, tint)
    }

    /// The docked pane's navigation or helper bar sits above the same body the canvas card draws.
    /// The body's bounds are recorded as `panel-body:<id>`.
    pub(super) fn docked_panel(&self, id: &str, cx: &mut Context<Self>) -> Option<(AnyElement, AnyElement)> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id)?;
        let measured = self.bounds.borrow().get(&format!("panel-body:{id}")).map_or([400., 500.], |b| [f32::from(b.size.width), f32::from(b.size.height)]);
        let layout = canophy::Layout::docked(measured);
        let (chrome, body, title) = match &pane.kind {
            canvas::Kind::Browser { .. } => (Some(self.browser_navigation(id, browser::Host::Pane, cx)), self.browser_body(pane, layout, cx), self.browser_title(pane).0),
            canvas::Kind::Sim { .. } => (None, self.sim_card_body(pane, layout, cx), self.sim_title(pane, cx).0),
            _ => return None,
        };
        let wired = self.wired_agent(id);
        let actions = helper_bar::pane_actions(pane, None, true, None);
        #[cfg(target_os = "macos")]
        let actions = self.sim_relabel(id, actions, cx);
        let label = wired.as_ref().map_or(title, |agent| format!("Wired to {}", self.pill_name(agent)));
        let header = div().flex().flex_col().flex_none().min_w_0().overflow_hidden()
            .children(chrome)
            .when(!actions.is_empty(), |d| d.child(div().flex().items_center().gap(px(6.)).px(px(12.)).h(px(36.)).border_b_1().border_color(c("border"))
                .child(div().flex_1().min_w_0().truncate().text_size(px(12.)).text_color(c("muted-foreground")).child(label))
                .child(self.pane_color_swatch(id, cx))
                .child(div().id("dock-pane-controls").min_w_0().overflow_x_scroll()
                    .child(self.bar(id, actions, 1., wired.is_some(), cx)))))
            .into_any_element();
        let bounds = self.bounds.clone();
        let key = format!("panel-body:{id}");
        let select = id.to_owned();
        let body = div().relative().flex_1().min_h_0().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.select_browser_pane(&select);
                cx.notify();
            }))
            .child(canvas(move |b, _, _| { bounds.borrow_mut().insert(key, b); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .child(div().size_full().child(body))
            .into_any_element();
        Some((header, body))
    }
}

#[cfg(test)]
mod tests {
    use super::Home;
    use serde_json::Value;

    #[test]
    fn browsers_and_sims_dock_by_default() {
        let home = |a: &str, kind| Home::parse(&serde_json::from_str::<Value>(a).unwrap(), kind);
        assert_eq!(home("{}", "browser"), Ok(Home::Panel));
        assert_eq!(home("{}", "sim"), Ok(Home::Panel));
        assert_eq!(home("{}", "file"), Ok(Home::Canvas));
        assert_eq!(home(r#"{"place":"canvas"}"#, "browser"), Ok(Home::Canvas));
        assert_eq!(home(r#"{"place":"pinned"}"#, "sim"), Ok(Home::Pinned));
        assert_eq!(home(r#"{"pinned":true}"#, "browser"), Ok(Home::Pinned));
        assert_eq!(home(r#"{"place":"canvas","pinned":true}"#, "browser"), Ok(Home::Canvas));
        assert!(home(r#"{"place":"panel"}"#, "mod:note").is_err());
        assert!(home(r#"{"place":"dock"}"#, "browser").is_err());
        assert!(home(r#"{"place":3}"#, "browser").is_err());
    }
}
