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

/// A copy of `pane` sized so the geometry code (placement, zoom, fixed viewports) sees its body as
/// exactly `body`, like a pinned pane whose body starts at the origin.
fn fit_to_body(mut pane: canvas::Pane, body: Size<Pixels>) -> canvas::Pane {
    let (w, h) = (f32::from(body.width), f32::from(body.height));
    let header = canvas::header_height(&pane.kind);
    pane.rect = [-1., -(header + 1.), w + 2., h + header + 2.];
    pane.pinned = Some([0., 0., w + 2., h + header + 2.]);
    pane
}

impl AppWindow {
    /// The docked pane as the panel body draws it, with that body's window bounds. None unless
    /// the pane is the panel's active tab and its body has been measured.
    pub(super) fn docked_view(&self, id: &str) -> Option<(canvas::Pane, Bounds<Pixels>)> {
        let state = &self.canvas_state;
        if state.file.as_ref().and_then(PanelTab::pane).map(String::as_str) != Some(id) { return None; }
        let body = *self.bounds.borrow().get(&format!("panel-body:{id}"))?;
        let pane = state.doc.panes.iter().find(|p| p.id == id)?.clone();
        Some((fit_to_body(pane, body.size), body))
    }

    /// Docks a pane in the Files panel and shows it there.
    pub(super) fn dock_pane(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        // A live page rests first, so it wakes again at its new place.
        self.rest_live_browser(id, cx);
        let state = &mut self.canvas_state;
        state.doc.dock(id);
        state.file = Some(PanelTab::Pane(id.to_owned()));
        state.selected = vec![id.to_owned()];
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// Puts a docked pane back on the canvas at its saved rect (or the view's center), and makes
    /// room for it.
    pub(super) fn undock_pane(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.rest_live_browser(id, cx);
        let center = self.view_center(window);
        let state = &mut self.canvas_state;
        let was_active = state.file.as_ref().and_then(PanelTab::pane).is_some_and(|p| p == id);
        state.doc.undock(id, center);
        if was_active { state.file = state.doc.panel.tabs.last().cloned(); }
        self.raise_pane(id, cx);
        self.solve_layout(Some(id), None, cx);
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// Closes a panel tab: a file tab goes away, a pane tab closes its pane.
    pub(super) fn close_panel_tab(&mut self, tab: &PanelTab, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        if tab.file().is_some_and(|path| state.dirty.as_ref() == Some(path)) { state.dirty = None; }
        match tab {
            PanelTab::File(_) => state.file = canvas::close_panel_tab(&mut state.doc.panel.tabs, state.file.as_ref(), tab),
            PanelTab::Pane(id) => self.close_boxed_panes(&[id.clone()], cx),
        }
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
    }

    /// The name of the terminal a pane is wired to, if any.
    fn wired_agent(&self, id: &str) -> Option<String> {
        let doc = &self.canvas_state.doc;
        let edges = crate::wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        let other = edges.iter().find_map(|w| if w.a == id { Some(&w.b) } else if w.b == id { Some(&w.a) } else { None })?;
        doc.panes.iter().any(|p| p.id == *other && p.terminal().is_some()).then(|| self.pill_name(other))
    }

    /// A pane tab's icon, title, and whether it is wired (the tab then shows the wire's dot).
    pub(super) fn docked_tab(&self, id: &str, cx: &App) -> (&'static str, String, bool) {
        let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return ("icons/x.svg", id.to_owned(), false) };
        let (icon, title) = match &pane.kind {
            canvas::Kind::Browser { .. } => ("icons/globe.svg", self.browser_title(pane).0),
            _ => ("icons/smartphone.svg", self.sim_title(pane, cx).0),
        };
        (icon, title, self.wired_agent(id).is_some())
    }

    /// The panel's header and body for the docked pane `id`: its helper bar over the same body
    /// the canvas card draws. The body's bounds are recorded as `panel-body:<id>`.
    pub(super) fn docked_panel(&self, id: &str, cx: &mut Context<Self>) -> Option<(AnyElement, AnyElement)> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id)?;
        let measured = self.bounds.borrow().get(&format!("panel-body:{id}")).map(|b| b.size);
        let view = fit_to_body(pane.clone(), measured.unwrap_or(size(px(400.), px(500.))));
        let camera = canvas::Camera::default();
        let (chrome, body, title) = match &pane.kind {
            canvas::Kind::Browser { .. } => (Some(self.browser_chrome(id, browser::Host::Pane, cx)), self.browser_body(&view, camera, cx), self.browser_title(pane).0),
            canvas::Kind::Sim { .. } => (None, self.sim_card_body(&view, camera, cx), self.sim_title(pane, cx).0),
            _ => return None,
        };
        let wired = self.wired_agent(id);
        let actions = helper_bar::pane_actions(pane, None, true);
        #[cfg(target_os = "macos")]
        let actions = self.sim_relabel(id, actions, cx);
        let actions = self.browser_relabel(id, actions);
        let label = wired.as_ref().map_or(title, |agent| format!("Wired to {agent}"));
        let header = div().flex().flex_col().flex_none()
            .children(chrome)
            .child(div().flex().items_center().gap(px(6.)).px(px(12.)).h(px(36.)).border_b_1().border_color(c("border"))
                .child(div().flex_1().min_w_0().truncate().text_size(px(12.)).text_color(c("muted-foreground")).child(label))
                .child(self.bar(id, actions, 1., wired.is_some(), cx)))
            .into_any_element();
        let bounds = self.bounds.clone();
        let key = format!("panel-body:{id}");
        let select = id.to_owned();
        let body = div().relative().flex_1().min_h_0().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.select_browser_pane(&select);
                cx.notify();
            }))
            .child(canvas(move |b, _, _| { bounds.borrow_mut().insert(key.clone(), b); }, |_, _, _, _| {}).absolute().size_full())
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
