//! Canvas mode owns the keyboard until the person leaves it. No timer or key sequences.
use super::*;

fn direction(value: &str) -> Option<canvas::Point> {
    match value {
        "left" => Some([-1., 0.]), "right" => Some([1., 0.]),
        "up" => Some([0., -1.]), "down" => Some([0., 1.]), _ => None,
    }
}

impl AppWindow {
    /// Intercept mode keys before GPUI actions or terminal input can consume them. Unbound keys
    /// are swallowed unless they carry ctrl, alt or cmd: those go on to the normal keymap (quit,
    /// the reset key), and the canvas holds the focus, so no terminal sees them.
    pub(in crate::window) fn canvas_key(&mut self, key: &gpui::Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.canvas_state.mode || !self.has_canvas() || self.overlays.iter().any(|o| o.kind.modal_keys()) { return false; }
        self.focus.focus(window, cx);
        if let Some(action) = crate::keys::canvas_action(key, cx) {
            if let Some(a) = action.as_any().downcast_ref::<ShellAction>() {
                self.run_action(a.id(), window, cx);
            } else if let Some(a) = action.as_any().downcast_ref::<crate::keys::CommandBinding>() {
                match a.command() {
                    Ok((name, args)) => self.run_key(&name, args, window, cx),
                    Err(error) => self.command_error("Key binding", &error, window, cx),
                }
            } else { window.dispatch_action(action, cx); }
            return true;
        }
        let m = key.modifiers;
        !(m.control || m.alt || m.platform)
    }

    /// The space of the selection, otherwise the current canvas space.
    pub(super) fn keyboard_space(&self) -> Option<String> {
        let state = &self.canvas_state;
        if let Some(key) = state.selected.last() {
            let space = state.doc.space_of(key).or_else(|| {
                let r = state.doc.previews.iter().find(|p| &p.path == key)?.rect;
                canvas::space_at(&state.doc.spaces, [r[0] + r[2] / 2., r[1] + r[3] / 2.])
            });
            return space.map(|s| s.space.clone());
        }
        state.doc.panel.space.clone().or_else(|| self.model.active.clone())
    }

    fn keyboard_cells(&self, cx: &App) -> Vec<(String, canvas::Rect)> {
        let doc = &self.canvas_state.doc;
        doc.cards.iter().filter(|c| canvas::view_allows(&doc.view, c.sub.is_some() || self.live.get(&c.terminal).is_some_and(canvas_agent)))
            .map(|c| (c.key(), self.pill_rect(c, cx)))
            .chain(doc.previews.iter().map(|p| (p.path.clone(), p.rect))).collect()
    }

    fn keyboard_select(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.selected = vec![key.clone()];
        self.canvas_state.wire.selected = None;
        if let Some(pane) = self.model.pane_of(&key) { self.model.focus_pane(&pane); }
        let space = self.keyboard_space();
        if let Some(id) = &space { self.model.active = Some(id.clone()); }
        self.canvas_state.doc.panel.space = space;
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Handle the canvas keyboard commands, reusing the existing cell and Wire actions.
    pub(in crate::window) fn canvas_keyboard(&mut self, name: &str, args: &Value, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_canvas() { return self.fail("canvas is not shown"); }
        let selected = self.canvas_state.selected.last().cloned();
        match name {
            "canvas.mode" => {
                self.canvas_state.mode = true;
                self.canvas_state.space_down = false;
                if selected.is_none() {
                    let cells = self.keyboard_cells(cx);
                    let current = self.keyboard_space();
                    if let Some((key, _)) = cells.iter().find(|(k, _)| self.canvas_state.doc.space_of(k).map(|s| &s.space) == current.as_ref()).or(cells.first()) {
                        self.keyboard_select(key.clone(), window, cx);
                    }
                }
                self.focus.focus(window, cx);
            }
            "canvas.leave" => {
                if self.canvas_state.connecting.take().is_none() { self.canvas_state.mode = false; }
                self.focus.focus(window, cx);
            }
            "canvas.enter" => {
                if let Some(from) = self.canvas_state.connecting.clone() {
                    let Some(to) = selected else { return self.fail("select the other cell first"); };
                    let result = self.run_as("wire.connect", json!({"a":from,"b":to}), true, window, cx);
                    if let Some(error) = result["error"].as_str() { return self.fail(error); }
                    self.canvas_state.connecting = None;
                    self.canvas_state.wire.selected = Some((from, to));
                } else if let Some(key) = selected {
                    if let Some((parent, id)) = crate::wire::sub_parts(&key) {
                        let path = self.canvas_state.subs.get(parent).into_iter().flatten().find(|s| s.id == id).and_then(|s| s.transcript.clone());
                        let Some(path) = path else { return self.fail("sub-agent transcript is not available yet"); };
                        self.open_preview(json!({"path":path}), window, cx);
                    } else if self.canvas_state.doc.cards.iter().any(|c| c.terminal == key && c.sub.is_none()) {
                        if !self.live.contains_key(&key) { return self.fail("no running terminal"); }
                        if self.canvas_state.popped.contains_key(&key) {
                            self.pop_canvas(key.clone(), true, window, cx);
                        } else { self.set_expanded(&key, true, window, cx); }
                        if let Some(pane) = self.model.pane_of(&key) { self.model.focus_pane(&pane); }
                        if let Some(t) = self.focused_cell() { t.focus.focus(window, cx); }
                    } else {
                        self.open_preview(json!({"path":key}), window, cx);
                    }
                    self.canvas_state.mode = false;
                } else { self.canvas_state.mode = false; }
            }
            "canvas.go" => {
                let Some(d) = args["direction"].as_str().and_then(direction) else { return self.fail("use left, right, up or down"); };
                let cells = self.keyboard_cells(cx);
                let rects: Vec<_> = cells.iter().map(|(_, r)| *r).collect();
                let next = selected.and_then(|key| cells.iter().position(|(k, _)| *k == key))
                    .and_then(|i| canvas::nearest(&rects, rects[i], d)).or_else(|| self.canvas_state.selected.is_empty().then_some(0));
                if let Some((key, _)) = next.and_then(|i| cells.get(i)) { self.keyboard_select(key.clone(), window, cx); }
            }
            "canvas.space" => {
                let doc = &self.canvas_state.doc;
                let n = doc.spaces.len();
                if n > 0 {
                    let i = self.keyboard_space().and_then(|id| doc.spaces.iter().position(|s| s.space == id));
                    let j = match args["direction"].as_str() {
                        Some("next") => i.map_or(0, |i| (i + 1) % n),
                        Some("prev") => i.map_or(n - 1, |i| (i + n - 1) % n),
                        _ => return self.fail("use next or prev"),
                    };
                    let id = doc.spaces[j].space.clone();
                    let first = self.keyboard_cells(cx).into_iter().find(|(k, _)| doc.space_of(k).is_some_and(|s| s.space == id));
                    self.canvas_state.selected.clear();
                    self.canvas_state.doc.panel.space = Some(id.clone());
                    self.model.active = Some(id);
                    if let Some((key, _)) = first { self.keyboard_select(key, window, cx); }
                    self.focus.focus(window, cx);
                }
            }
            "canvas.collapse" => if let Some(key) = selected { self.set_expanded(&key, false, window, cx); },
            "canvas.connect" => {
                let Some(key) = selected.filter(|k| self.canvas_state.doc.cards.iter().any(|c| c.key() == *k)) else { return self.fail("select a cell first"); };
                self.canvas_state.mode = true;
                self.canvas_state.connecting = Some(key);
            }
            "canvas.thread" => {
                let edge = self.canvas_state.wire.selected.clone().or_else(|| {
                    crate::wire::all(&self.canvas_state.doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned))
                        .into_iter().find(|w| selected.as_ref().is_some_and(|k| w.a == *k || w.b == *k)).map(|w| (w.a, w.b))
                });
                let Some((a,b)) = edge else { return self.fail("select a wired cell or wire first"); };
                self.run("wire.open", json!({"a":a,"b":b}), window, cx);
            }
            "canvas.nudge" => {
                let Some(d) = args["direction"].as_str().and_then(direction) else { return self.fail("use left, right, up or down"); };
                let Some(key) = selected else { return self.fail("select a cell first"); };
                self.canvas_state.end_drag();
                self.begin_layout();
                if self.canvas_state.nudge(&key, d) {
                    self.solve_layout(Some(&key), None, cx);
                    self.canvas_state.end_drag();
                    self.canvas_changed(cx);
                } else { self.canvas_state.end_drag(); }
            }
            "canvas.new" | "canvas.agents" => {
                let space = self.keyboard_space();
                let rect = selected.as_deref().and_then(|key| self.keyboard_cells(cx).into_iter().find(|(k, _)| k == key)).map(|(_,r)|r)
                    .or_else(|| space.as_ref().and_then(|id| self.canvas_state.doc.spaces.iter().find(|s| &s.space == id)).map(|s| s.rect));
                let point = rect.map(|r| [r[0] + canvas::GAP, r[1] + canvas::GAP]);
                let mut at = json!({});
                if let Some(point) = point { at["point"] = json!(point); }
                if let Some(id) = space { at["space"] = json!(id); }
                if name == "canvas.agents" {
                    if let Some(machine) = at["space"].as_str().and_then(|id| self.model.space(id)).and_then(|s| s.machine.as_ref()) {
                        at["machine"] = json!(machine);
                    }
                    self.open_menu("canvas-agents", at, None, window, cx);
                } else { self.create_canvas(at, window, cx); }
            }
            _ => unreachable!(),
        }
        self.sync(window, cx);
        if (self.canvas_state.mode || name == "canvas.leave") && !self.overlays.iter().any(|o| o.kind.modal_keys()) { self.focus.focus(window, cx); }
        cx.notify();
    }
}

impl State {
    fn nudge(&mut self, key: &str, direction: canvas::Point) -> bool {
        let rect = self.doc.cards.iter_mut().find(|c| c.key() == key).map(|c| &mut c.rect)
            .or_else(|| self.doc.previews.iter_mut().find(|p| p.path == key).map(|p| &mut p.rect));
        let Some(rect) = rect else { return false };
        rect[0] += direction[0] * canvas::GAP;
        rect[1] += direction[1] * canvas::GAP;
        true
    }
}
