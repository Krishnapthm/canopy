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
        // Something else on the canvas has the keyboard (a rename, the thread input, the files panel).
        if !self.focus.is_focused(window) { return false; }
        if key.key == "escape" && self.wire_cancel(window, cx) { return true; }
        if key.key == "escape" && self.canvas_state.browser_tab.is_some() { self.collapse_browser(window, cx); return true; }
        // A lone modifier tap (GPUI sends releasing Option as an "alt" key) is never swallowed:
        // its release must still reach the modifier listeners.
        if matches!(key.key.as_str(), "shift" | "control" | "alt" | "platform" | "function") { return false; }
        // Space held: drag pans (the key-up clears it).
        if key.key == "space" && !key.modifiers.modified() {
            if !self.canvas_state.space_down { self.canvas_state.space_down = true; cx.notify(); }
            return true;
        }
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
        // The Linux Main Menu has a regular GPUI binding so users can override it.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if key.key == "f10" && !key.modifiers.modified() {
            return false;
        }
        let m = key.modifiers;
        !(m.control || m.alt || m.platform)
    }

    /// The room being shown; on Home, the selected space card's space.
    pub(super) fn keyboard_space(&self) -> Option<String> {
        let doc = &self.canvas_state.doc;
        doc.room.clone().or_else(|| self.canvas_state.selected.last().filter(|k| doc.spaces.iter().any(|s| &s.space == *k)).cloned())
    }

    /// What the keyboard can select in the current place: its cells and previews, and on Home the space cards.
    pub(super) fn keyboard_cells(&self, cx: &App) -> Vec<(String, canvas::Rect)> {
        let doc = &self.canvas_state.doc;
        doc.panes.iter().filter(|c| doc.shows(c.space.as_ref()) && (c.boxed() || canvas::view_allows(&doc.view, matches!(c.kind, canvas::Kind::Sub { .. }) || self.live.get(&c.id).is_some_and(canvas_agent))))
            .map(|c| (c.id.clone(), self.pill_rect(c, cx)))
            .chain(doc.spaces.iter().filter(|_| doc.room.is_none()).map(|s| (s.space.clone(), s.rect))).collect()
    }

    fn keyboard_select(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.selected = vec![key.clone()];
        self.canvas_state.wire.selected = None;
        if let Some(pane) = self.model.pane_of(&key) { self.model.focus_pane(&pane); }
        if let Some(id) = self.keyboard_space() {
            self.model.active = Some(id.clone());
            self.canvas_state.doc.panel.space = Some(id);
        }
        self.focus.focus(window, cx);
        self.tile_canvas(false, window, cx);
        cx.notify();
    }

    /// Handle the canvas keyboard commands, reusing the existing cell and Wire actions.
    pub(in crate::window) fn canvas_keyboard(&mut self, name: &str, args: &Value, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_canvas() { return self.fail("canvas is not shown"); }
        let selected = self.canvas_state.selected.last().cloned();
        match name {
            "canvas.mode" => {
                // Leaves the entered cell (it stays selected), or gives the canvas the keyboard.
                self.canvas_state.mode = true;
                self.canvas_state.space_down = false;
                if selected.is_none() {
                    let cells = self.keyboard_cells(cx);
                    if let Some((key, _)) = cells.first() {
                        self.keyboard_select(key.clone(), window, cx);
                    }
                }
                self.focus.focus(window, cx);
            }
            "canvas.leave" => {
                // Esc ends sync input first. Then, like Figma: cancel a connection, else clear the selection, else go Home.
                if !self.canvas_state.synced.borrow().is_empty() {
                    self.canvas_state.synced.borrow_mut().clear();
                } else if self.canvas_state.connecting.take().is_none() {
                    if !self.canvas_state.selected.is_empty() || self.canvas_state.wire.selected.is_some() {
                        self.canvas_state.selected.clear();
                        self.canvas_state.wire.selected = None;
                    } else if self.canvas_state.doc.room.is_some() {
                        return self.go_room(None, window, cx);
                    }
                }
                self.canvas_state.mode = true;
                self.focus.focus(window, cx);
            }
            "canvas.select_all" => {
                if !self.focus.is_focused(window) { return; }
                let doc = &self.canvas_state.doc;
                self.canvas_state.selected = doc.panes.iter().filter(|c| c.terminal().is_some() && doc.shows(c.space.as_ref())).map(|c| c.id.clone()).collect();
            }
            "canvas.enter" => {
                if let Some(from) = self.canvas_state.connecting.clone() {
                    let Some(to) = selected else { return self.fail("select the other cell first"); };
                    let result = self.run_as("wire.connect", json!({"a":from,"b":to}), true, window, cx);
                    if let Some(error) = result["error"].as_str() { return self.fail(error); }
                    self.canvas_state.connecting = None;
                    self.canvas_state.wire.selected = Some((from, to));
                } else if let Some(key) = selected {
                    if self.canvas_state.doc.room.is_none() && self.canvas_state.doc.spaces.iter().any(|s| s.space == key) {
                        // Enter on a space card goes inside it; the keyboard stays on the canvas.
                        self.go_room(Some(key), window, cx);
                        return;
                    }
                    if let Some((parent, id)) = crate::wire::sub_parts(&key) {
                        let path = self.canvas_state.subs.get(parent).into_iter().flatten().find(|s| s.id == id).and_then(|s| s.transcript.clone());
                        let Some(path) = path else { return self.fail("sub-agent transcript is not available yet"); };
                        self.open_preview(json!({"path":path}), window, cx);
                    } else if self.canvas_state.doc.panes.iter().any(|c| c.id == key && c.terminal().is_some()) {
                        if !self.live.contains_key(&key) { return self.fail("no running terminal"); }
                        if self.canvas_state.popped.contains_key(&key) {
                            self.pop_canvas(key.clone(), true, window, cx);
                        } else { self.set_expanded(&key, true, window, cx); }
                        if let Some(pane) = self.model.pane_of(&key) { self.model.focus_pane(&pane); }
                        if let Some(t) = self.focused_cell() { t.focus.focus(window, cx); }
                    } else if let Some(path) = self.canvas_state.doc.panes.iter().find(|p| p.id == key).and_then(|p| p.file()).map(str::to_owned) {
                        self.open_preview(json!({"path":path}), window, cx);
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
                // In top strip tab order: next / prev step, a number goes into that space,
                // recent / oldest walk the spaces by when they were last entered.
                let ids = super::strip_spaces(&self.model);
                let n = ids.len();
                if n == 0 { return self.fail("no spaces yet"); }
                let room = self.canvas_state.doc.room.clone();
                let i = room.as_ref().and_then(|id| ids.iter().position(|s| s == id));
                let id = match args["direction"].as_str() {
                    Some("next") => ids[i.map_or(0, |i| (i + 1) % n)].clone(),
                    Some("prev") => ids[i.map_or(n - 1, |i| (i + n - 1) % n)].clone(),
                    Some(d @ ("recent" | "oldest")) => {
                        let Some(id) = super::recent_space(&self.canvas_state.recent, &ids, room.as_ref(), d == "oldest") else { return };
                        id
                    }
                    Some(k) if ids.iter().any(|s| s == k) => k.to_owned(),
                    Some(k) => match k.parse::<usize>() {
                        Ok(k) if (1..=n).contains(&k) => ids[k - 1].clone(),
                        _ => return self.fail(format!("no space {k}")),
                    },
                    None => return self.fail("use next, prev, recent, oldest or a number"),
                };
                self.go_room(Some(id), window, cx);
                return;
            }
            "canvas.home" => {
                self.go_room(None, window, cx);
                return;
            }
            "canvas.collapse" => if let Some(key) = selected { self.set_expanded(&key, false, window, cx); },
            "canvas.connect" => {
                let Some(key) = selected.filter(|k| self.canvas_state.doc.panes.iter().any(|c| c.id == *k)) else { return self.fail("select a cell first"); };
                self.canvas_state.mode = true;
                self.canvas_state.connecting = Some(key);
            }
            "canvas.nudge" => {
                let Some(d) = args["direction"].as_str().and_then(direction) else { return self.fail("use left, right, up or down"); };
                let Some(key) = selected else { return self.fail("select a cell first"); };
                self.end_canvas_drag(cx);
                self.begin_layout();
                if self.canvas_state.nudge(&key, d, self.person.canvas_gap) {
                    // Tile view puts the tile back instead of pushing neighbours.
                    if self.canvas_state.tile.is_some() { self.tile_canvas(false, window, cx); }
                    else { self.solve_layout(Some(&key), None, cx); }
                    self.canvas_state.end_drag();
                    self.canvas_changed(cx);
                } else { self.canvas_state.end_drag(); }
            }
            "canvas.new" | "canvas.agents" => {
                let space = self.canvas_state.doc.room.clone();
                let mut at = json!({});
                if name == "canvas.new" {
                    // Cascade: a fixed step down and right of the last new pane, within the view.
                    let [w, h] = self.bounds.borrow().get("canopy-canvas").map_or([1200., 800.], |b| [f32::from(b.size.width), f32::from(b.size.height)]);
                    let camera = self.canvas_state.doc.camera;
                    let (a, b) = (camera.to_world([0., 0.]), camera.to_world([w, h]));
                    let doc = &self.canvas_state.doc;
                    let taken: Vec<_> = doc.panes.iter().filter(|c| !c.boxed() && doc.shows(c.space.as_ref())).map(|c| [c.rect[0], c.rect[1]]).collect();
                    let last = self.canvas_state.cascade.as_ref().filter(|(room, _)| *room == space).map(|(_, p)| *p);
                    let point = canvas::cascade(last, [a[0], a[1], b[0] - a[0], b[1] - a[1]], self.default_pane_size(window), &taken, CASCADE_STEP);
                    self.canvas_state.cascade = Some((space.clone(), point));
                    at["point"] = json!(point);
                    at["cascade"] = json!(true);
                }
                if let Some(id) = space { at["space"] = json!(id); }
                if name == "canvas.agents" {
                    if let Some(machine) = at["space"].as_str().and_then(|id| self.model.space(id)).and_then(|s| s.machine.as_ref()) {
                        at["machine"] = json!(machine);
                    }
                    self.open_menu("canvas-agents", at, None, window, cx);
                } else {
                    // `like`: start what the selected cell runs (in its space, so its folder), else a plain terminal.
                    if args["like"] == true {
                        let rec = selected.as_deref().and_then(|key| self.live.get(key));
                        if let Some(run) = rec.and_then(|rec| super::like_run(rec, &crate::programs::startable())) { at["argv"] = json!(run); }
                    }
                    self.create_canvas(at, window, cx);
                }
            }
            _ => unreachable!(),
        }
        self.sync(window, cx);
        if (self.canvas_state.mode || name == "canvas.leave") && !self.overlays.iter().any(|o| o.kind.modal_keys()) { self.focus.focus(window, cx); }
        cx.notify();
    }
}

/// How far each ⌘T pane steps down and right from the last (world px).
const CASCADE_STEP: f32 = 32.;

impl State {
    fn nudge(&mut self, key: &str, direction: canvas::Point, gap: f32) -> bool {
        let rect = self.doc.panes.iter_mut().find(|c| c.id == key).map(|c| &mut c.rect);
        let Some(rect) = rect else { return false };
        rect[0] += direction[0] * gap;
        rect[1] += direction[1] * gap;
        true
    }
}
