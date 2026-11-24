//! Wire on the canvas: drawing, selecting and removing edges. The edge model is
//! `crate::wire`; docs/wiki/Wiring.md describes both.
use super::*;
use crate::wire;

/// Diameter of the round "+" shown at the middle of each side of a hovered cell.
const HANDLE: f32 = 10.;
/// How long an edge glows after it is pulsed.
const PULSE: Duration = Duration::from_millis(700);
/// How near (screen px) a click must be to an edge to take it.
const GRAB: f32 = 6.;

/// What the canvas remembers about Wire between frames.
#[derive(Default)]
pub(in crate::window) struct WireUi {
    /// The cell whose side "+" handles show.
    pub(super) hover: Option<String>,
    /// A wire being drawn: the cell it starts at, where it starts (world) and the pointer (window).
    pub(super) drag: Option<(String, canvas::Point, Point<Pixels>)>,
    /// Where the button went down on the "+" handle, to tell a click from a drag.
    down: Point<Pixels>,
    /// The handle (or the helper bar's Connect) was clicked, not dragged: the wire follows the
    /// pointer until a click picks a pane.
    pub(super) sticky: bool,
    pub(super) selected: Option<(String, String)>,
    /// When an edge was last pulsed, for its glow.
    pulses: HashMap<(String, String), Instant>,
}

/// One edge as drawn: its cells in the order stored (a is the parent of a born edge), and its curve.
pub(super) struct Edge {
    key: (String, String),
    born: bool,
    path: Vec<canvas::Point>,
}

/// Whether a child's status change glows its edge: the word changed and the edge is drawn.
/// Agent-agnostic: only the core's status word and parent field count.
fn status_pulses(prev: Option<&str>, word: Option<&str>, on_canvas: bool) -> bool {
    on_canvas && prev != word
}

/// How far (screen px) the pointer may move between press and release on a handle and still be a click.
const CLICK: f32 = 4.;

/// A release this close to where the button went down on a handle is a click, not a drag.
fn is_click(down: Point<Pixels>, up: Point<Pixels>) -> bool {
    f32::from(up.x - down.x).hypot(f32::from(up.y - down.y)) < CLICK
}

/// The pointer's line while a wire is drawn: a cubic leaving `a` sideways.
fn bezier(a: canvas::Point, z: canvas::Point) -> [canvas::Point; 2] {
    [[(a[0] + z[0]) / 2., a[1]], [(a[0] + z[0]) / 2., z[1]]]
}

impl AppWindow {
    fn wire_toast(&mut self, text: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":text.into()}}), window, cx);
    }

    /// The drawn rect of a pane the current view shows.
    pub(super) fn wire_cell(&self, doc: &Document, key: &str) -> Option<canvas::Rect> {
        let card = doc.panes.iter().find(|c| c.id == key && doc.shown(c))?;
        self.wire_cell_rect(card)
    }

    pub(super) fn wire_cell_rect(&self, card: &canvas::Pane) -> Option<canvas::Rect> {
        let state = &self.canvas_state;
        match &card.kind {
            canvas::Kind::File { path } => return state.previews.contains_key(path).then_some(card.rect),
            canvas::Kind::Pdf { .. } => return state.pdfs.contains_key(&card.id).then_some(card.rect),
            canvas::Kind::Sub { parent, sub } => return (state.doc.view != canvas::View::Terminals
                && state.subs.get(parent).is_some_and(|subs| subs.iter().any(|item| &item.id == sub)))
                .then_some(card.rect),
            canvas::Kind::Image { .. } | canvas::Kind::Browser { .. } | canvas::Kind::Sim { .. }
            | canvas::Kind::Mod { .. } => return Some(card.rect),
            canvas::Kind::Terminal { .. } => {}
        }
        if !canvas::view_allows(&state.doc.view, self.live.get(&card.id).is_some_and(canvas_agent)) { return None; }
        // Match the drawn cell: an expanded one has its own size, a pill its measured one.
        let size = match card.size() {
            Some(_) => [card.rect[2].max(canvas::PILL_WIDTH), card.rect[3].max(canvas::HEADER)],
            None => state.pill_sizes.borrow().get(&card.id).map(|v| v.3).unwrap_or([240., 36.]),
        };
        Some([card.rect[0], card.rect[1], size[0], size[1]])
    }

    /// A child's status word changed: glow its born edge to `parent` if both cells are on the canvas.
    pub(in crate::window) fn pulse_status(&mut self, child: &str, parent: &str, prev: Option<&str>, word: Option<&str>, cx: &mut Context<Self>) {
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let on_canvas = [child, parent].iter().all(|k| doc.panes.iter().filter(|p| !p.boxed()).any(|c| c.id == *k));
        if status_pulses(prev, word, on_canvas) {
            // Spent glows go first, so closed children leave nothing behind.
            self.canvas_state.wire.pulses.retain(|_, t| t.elapsed() < PULSE);
            self.canvas_state.wire.pulses.insert((parent.to_owned(), child.to_owned()), Instant::now());
            cx.notify();
        }
    }

    /// Every edge between shown cells: drawn wires, and born ones (sub-agents, `hc run` children).
    pub(super) fn wire_edges(&self) -> Vec<Edge> {
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let mut out: Vec<Edge> = vec![];
        let all = wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        for w in all.iter().filter(|w| w.born).chain(all.iter().filter(|w| !w.born)) {
            if out.iter().any(|e| (e.key.0 == w.a && e.key.1 == w.b) || (e.key.0 == w.b && e.key.1 == w.a)) { continue; }
            if let (Some(a), Some(b)) = (self.wire_cell(doc, &w.a), self.wire_cell(doc, &w.b)) {
                out.push(Edge { key: (w.a.clone(), w.b.clone()), born: w.born, path: canvas::curve(a, b) });
            }
        }
        out
    }

    /// A window position in canvas-local pixels.
    pub(super) fn wire_local(&self, position: Point<Pixels>) -> canvas::Point {
        let origin = self.bounds.borrow().get("canopy-canvas").map(|b| b.origin).unwrap_or_default();
        [f32::from(position.x - origin.x), f32::from(position.y - origin.y)]
    }

    /// The topmost shown cell under a window position, with `slop` extra screen pixels around it.
    pub(super) fn wire_cell_at(&self, position: Point<Pixels>, slop: f32) -> Option<String> {
        if !self.bounds.borrow().get("canopy-canvas")?.contains(&position) || self.canvas_overlay_at(position) { return None; }
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let camera = self.canvas_state.doc.camera;
        let p = camera.to_world(self.wire_local(position));
        let s = slop / camera.zoom;
        doc.panes.iter().filter(|c| doc.shown(c)).filter(|c| {
            self.wire_cell_rect(c).is_some_and(|r| p[0] >= r[0] - s && p[0] <= r[0] + r[2] + s && p[1] >= r[1] - s && p[1] <= r[1] + r[3] + s)
        }).max_by_key(|c| super::paint_order(c)).map(|c| c.id.clone())
    }

    fn wire_edge_at(&self, position: Point<Pixels>) -> Option<Edge> {
        let camera = self.canvas_state.doc.camera;
        let p = self.wire_local(position);
        self.wire_edges().into_iter().find(|e| canvas::path_distance(&e.path.iter().map(|v| camera.to_screen(*v)).collect::<Vec<_>>(), p) <= GRAB)
    }

    /// Mouse moves on the canvas: shows the "+" handles of the cell under the pointer and
    /// follows a wire being drawn. True when the move was part of drawing.
    pub(in crate::window) fn wire_pointer(&mut self, e: &MouseMoveEvent, cx: &mut Context<Self>) -> bool {
        let wire = &mut self.canvas_state.wire;
        if let Some(drag) = wire.drag.as_mut().filter(|_| wire.sticky) {
            drag.2 = e.position;
            cx.notify();
            return true;
        }
        if let Some((from, start, _)) = self.canvas_state.wire.drag.take() {
            // The button came up outside the window: nothing was released on a cell, so cancel.
            if e.pressed_button.is_some() { self.canvas_state.wire.drag = Some((from, start, e.position)); }
            cx.notify();
            return true;
        }
        let busy = e.pressed_button.is_some() || self.canvas_state.drag.is_some() || self.canvas_state.rest.is_some() || self.canvas_state.cell_resize.is_some() || self.canvas_state.preview_drag.is_some() || self.canvas_state.space_drag.is_some() || self.canvas_state.marquee.is_some();
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let point = self.wire_local(e.position);
        let bar_height = 28. * doc.camera.zoom.clamp(0.75, 1.0) + 2.;
        let bar_hover = if busy { None } else {
            doc.panes.iter().filter(|p| doc.shown(p) && !matches!(p.kind, canvas::Kind::Sub { .. }))
                .filter(|p| {
                    self.wire_cell_rect(p).is_some_and(|r| {
                        let camera = self.canvas_state.doc.camera;
                        let [x, y] = camera.to_screen([r[0], r[1]]);
                        point[0] >= x && point[0] <= x + r[2] * camera.zoom
                            && point[1] >= y - 8. - bar_height && point[1] <= y + r[3] * camera.zoom
                    })
                // Topmost as drawn: every pane kind paints in one z order.
                }).max_by_key(|p| super::paint_order(p)).map(|p| p.id.clone())
        };
        if bar_hover != self.canvas_state.bar_hover {
            self.canvas_state.bar_hover = bar_hover;
            cx.notify();
        }
        let hover = if busy { None } else { self.wire_cell_at(e.position, HANDLE) };
        if hover != self.canvas_state.wire.hover {
            self.canvas_state.wire.hover = hover;
            cx.notify();
        }
        false
    }

    /// A left press on empty canvas: takes an edge under it (selects it).
    pub(in crate::window) fn wire_click(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(edge) = self.wire_edge_at(position) else {
            self.canvas_state.wire.selected = None;
            return false;
        };
        self.canvas_state.selected.clear();
        self.canvas_state.wire.selected = Some(edge.key.clone());
        self.focus.focus(window, cx);
        cx.notify();
        true
    }

    /// A right press on empty canvas: the edge's menu (a drawn edge can be removed, a born one
    /// has no actions). True when an edge was under it.
    pub(in crate::window) fn wire_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(edge) = self.wire_edge_at(position) else { return false };
        self.canvas_state.wire.selected = Some(edge.key.clone());
        if !edge.born { self.open_menu("canvas-wire", json!({"a":edge.key.0,"b":edge.key.1}), Some(position), window, cx); }
        cx.notify();
        true
    }

    /// The left button came up: a wire being drawn connects to the cell under it, or is dropped.
    /// A release on the handle it started from, without moving, keeps the wire (see `wire_pick`).
    pub(in crate::window) fn wire_release(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let wire = &mut self.canvas_state.wire;
        if wire.drag.is_none() || wire.sticky { return false; }
        if is_click(wire.down, position) {
            wire.sticky = true;
            self.sync(window, cx);
            cx.notify();
            return true;
        }
        let Some((from, ..)) = self.canvas_state.wire.drag.take() else { return false };
        if let Some(to) = self.wire_cell_at(position, 0.).filter(|to| *to != from) {
            if let Err(why) = self.wire_connect(&from, &to, cx) { self.wire_toast(why, window, cx); }
        }
        cx.notify();
        true
    }

    /// A left press while a wire follows the pointer (capture phase, so an entered terminal cannot
    /// swallow it): the cell under it is wired to, anywhere else cancels. True when it was taken.
    pub(in crate::window) fn wire_pick(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.canvas_state.wire.sticky { return false; }
        let drag = self.canvas_state.wire.drag.take();
        self.canvas_state.wire.sticky = false;
        if let Some(to) = drag.and_then(|(from, ..)| self.wire_cell_at(position, 0.).filter(|to| *to != from).map(|to| (from, to))) {
            if let Err(why) = self.wire_connect(&to.0, &to.1, cx) { self.wire_toast(why, window, cx); }
        }
        self.sync(window, cx);
        cx.notify();
        true
    }

    /// Escape drops a wire that follows the pointer. True when there was one.
    pub(in crate::window) fn wire_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let wire = &mut self.canvas_state.wire;
        if !wire.sticky { return false; }
        (wire.drag, wire.sticky) = (None, false);
        self.sync(window, cx);
        cx.notify();
        true
    }

    /// The helper bar's Connect: like a clicked "+" handle, a wire leaves the pane's right side
    /// and follows the pointer until a click picks a pane (`wire_pick`) or Escape drops it.
    pub(super) fn wire_start(&mut self, key: &str, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        // A docked pane has no cell: its wire starts where the Connect button was clicked.
        let anchor = match self.wire_cell(doc, key) {
            Some(r) => [r[0] + r[2], r[1] + r[3] / 2.],
            None if doc.docked(key) => doc.camera.to_world(self.wire_local(position)),
            None => return,
        };
        self.canvas_state.selected.clear();
        // The canvas takes the keyboard so Escape can cancel.
        self.canvas_state.mode = true;
        self.focus.focus(window, cx);
        self.canvas_state.wire.drag = Some((key.to_owned(), anchor, position));
        self.canvas_state.wire.sticky = true;
        self.sync(window, cx);
        cx.notify();
    }

    /// Delete on a selected edge removes it (a selected cell takes the key first).
    pub(in crate::window) fn wire_delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keyboard = self.focus.is_focused(window) && self.canvas_state.selected.is_empty();
        let Some((a, b)) = self.canvas_state.wire.selected.clone().filter(|_| keyboard) else { return false };
        if let Err(why) = self.wire_remove(&a, &b, cx) { self.wire_toast(why, window, cx); }
        true
    }

    fn wire_connect(&mut self, a: &str, b: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let doc = &mut self.canvas_state.doc;
        if a == b { return Err("A pane cannot be wired to itself".into()); }
        if ![a, b].iter().all(|k| doc.panes.iter().any(|c| c.id == *k)) { return Err("No such pane".into()); }
        if [a, b].iter().all(|k| doc.panes.iter().any(|c| c.id == *k && c.boxed())) { return Err("Wire a pane to a terminal or agent".into()); }
        if !doc.same_space(a, b) { return Err("Same space only for now".into()); }
        if !doc.connect(a, b) { return Err("Already wired".into()); }
        self.canvas_changed(cx);
        Ok(())
    }

    fn wire_remove(&mut self, a: &str, b: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let before = self.canvas_state.doc.wires.len();
        self.canvas_state.doc.wires.retain(|w| !w.joins(a, b));
        if before == self.canvas_state.doc.wires.len() { return Err("That wire ends with the cell that started it".into()); }
        self.canvas_state.wire.selected = None;
        self.canvas_changed(cx);
        Ok(())
    }

    /// The `wire.*` commands.
    pub(in crate::window) fn wire_command(&mut self, name: &str, a: &Value, cx: &mut Context<Self>) {
        let (Some(from), Some(to)) = (a["a"].as_str(), a["b"].as_str()) else { return self.fail(format!("{name} needs a and b")) };
        let result = if name == "wire.connect" { self.wire_connect(from, to, cx) } else { self.wire_remove(from, to, cx) };
        if let Err(why) = result { self.fail(why); }
    }

    /// The "+" handles of the hovered cell, and the live curve of a wire being drawn: absolute
    /// elements to add after the cells.
    pub(in crate::window) fn wire_handles(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let state = &self.canvas_state;
        let doc = state.shown.as_ref().unwrap_or(&state.doc);
        let camera = doc.camera;
        let key = state.wire.drag.as_ref().map(|d| &d.0).or(state.wire.hover.as_ref());
        let Some(rect) = key.and_then(|k| self.wire_cell(doc, k).map(|r| (k.clone(), r))) else { return vec![] };
        let (key, r) = rect;
        let mut out = vec![];
        let (mid_x, mid_y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
        for (i, side) in [[mid_x, r[1]], [r[0] + r[2], mid_y], [mid_x, r[1] + r[3]], [r[0], mid_y]].into_iter().enumerate() {
            let [x, y] = camera.to_screen(side);
            let (key, start) = (key.clone(), side);
            let line = |w: f32, h: f32| div().absolute().left(px((HANDLE - w) / 2. - 1.)).top(px((HANDLE - h) / 2. - 1.)).w(px(w)).h(px(h)).bg(c("foreground"));
            out.push(div().id(SharedString::from(format!("wire-handle-{key}-{i}"))).absolute().left(px(x - HANDLE / 2.)).top(px(y - HANDLE / 2.))
                .size(px(HANDLE)).rounded_full().bg(c("surface")).border_1().border_color(c("muted-foreground")).cursor_pointer()
                .hover(|d| d.border_color(c("foreground")))
                .child(line(6., 1.)).child(line(1., 6.))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    this.canvas_state.selected.clear();
                    // Focus the canvas so Escape can cancel a wire that ends up following the pointer.
                    this.canvas_state.mode = true;
                    this.focus.focus(window, cx);
                    this.canvas_state.wire.drag = Some((key.clone(), start, e.position));
                    this.canvas_state.wire.down = e.position;
                    cx.stop_propagation();
                    cx.notify();
                })).into_any_element());
        }
        out
    }

    /// Resting edges paint under panes. A selected wire, a wire with both panes selected,
    /// and a wire being drawn paint above them. Only an active glow requests animation frames.
    pub(super) fn wire_layer(&self, viewport: canvas::Point, above: bool, edges: &[Edge]) -> AnyElement {
        let state = &self.canvas_state;
        let camera = state.doc.camera;
        let mut paint = vec![];
        for edge in edges {
            let selected = state.wire.selected.as_ref() == Some(&edge.key);
            let both_ends_selected = state.selected.contains(&edge.key.0) && state.selected.contains(&edge.key.1);
            if above != (selected || both_ends_selected) || !canvas::edge_visible(camera, &edge.path, viewport) { continue; }
            let pulse = state.wire.pulses.get(&edge.key).copied().filter(|t| t.elapsed() < PULSE);
            let tint = state.doc.effective_color(&edge.key.0).map(|hue| c(&hue.tint_token()).opacity(if selected { 1. } else { 0.6 }));
            let muted = c("muted-foreground");
            let color = tint.unwrap_or_else(|| if selected { c("foreground").opacity(0.7) } else { muted.opacity(if edge.born { 0.5 } else { 0.7 }) });
            paint.push((edge.path.clone(), edge.born, color, tint.unwrap_or(muted), pulse));
        }
        let drag = state.wire.drag.as_ref().filter(|_| above);
        let live = drag.map(|(from, start, pointer)| {
            let color = state.doc.effective_color(from).map_or_else(|| c("ring"), |hue| c(&hue.tint_token()));
            (*start, camera.to_world(self.wire_local(*pointer)), color)
        });
        let target = drag.and_then(|(from, _, pointer)| self.wire_cell_at(*pointer, 0.).filter(|k| k != from))
            .and_then(|k| self.wire_cell(state.shown.as_ref().unwrap_or(&state.doc), &k));
        canvas(|_, _, _| {}, move |b, _, window, cx| {
            let p = |v: [f32; 2]| point(b.origin.x + px(v[0]), b.origin.y + px(v[1]));
            let mut animating = false;
            for (path, born, color, dot_color, pulse) in &paint {
                let screen: Vec<_> = path.iter().map(|v| camera.to_screen(*v)).collect();
                let glow = pulse.map(|t| 1. - t.elapsed().as_secs_f32() / PULSE.as_secs_f32()).filter(|g| *g > 0.);
                animating |= glow.is_some();
                let color = if glow.is_some() { c("foreground") } else { *color };
                let width = if glow.is_some() { 2.5 } else if *born { 1. } else { 1.5 };
                // The sampled curve (canvas::curve).
                let mut line = PathBuilder::stroke(px(width));
                let Some((first, rest)) = screen.split_first() else { continue };
                line.move_to(p(*first));
                for v in rest { line.line_to(p(*v)); }
                if let Ok(line) = line.build() { window.paint_path(line, color); }
                let dot = |window: &mut Window, v: [f32; 2], r: f32| window.paint_quad(fill(Bounds::new(p([v[0] - r, v[1] - r]), size(px(2. * r), px(2. * r))), *dot_color).corner_radii(px(r)));
                if let Some(end) = screen.last() { dot(window, *end, 3.); }
                if !*born && let Some(start) = screen.first() { dot(window, *start, 3.); }
            }
            if let Some((start, world, color)) = live {
                let (sa, sz) = (camera.to_screen(start), camera.to_screen(world));
                let mut path = PathBuilder::stroke(px(1.5));
                path.move_to(p(sa));
                let [c1, c2] = bezier(sa, sz);
                path.cubic_bezier_to(p(sz), p(c1), p(c2));
                if let Ok(path) = path.build() { window.paint_path(path, color); }
            }
            if let Some(r) = target {
                let [x, y] = camera.to_screen([r[0], r[1]]);
                window.paint_quad(outline(Bounds::new(p([x, y]), size(px(r[2] * camera.zoom), px(r[3] * camera.zoom))), c("ring"), BorderStyle::Solid).corner_radii(px(8. * camera.zoom)));
            }
            if animating && !cx.reduce_motion() { window.request_animation_frame(); }
        }).absolute().size_full().into_any_element()
    }
}

#[cfg(test)]
mod sticky_tests {
    use super::{Modifiers, MouseButton, TestAppContext, json, point, px};
    use crate::window::tests::{in_own_process, open_window, running_alone};
    const TEST: &str = "window::canvas_view::wire_view::sticky_tests::sticky_wire";

    #[test]
    fn a_clicked_handle_keeps_the_wire_until_a_cell_is_clicked() { in_own_process(TEST); }

    #[gpui::test]
    fn sticky_wire(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "wire-input", "", json!({"id":"root","type":"canvas"}));
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| {
            this.model.add_space("root", "Canvas", &crate::window::home(), crate::model::Tab::new("tab1", "", "pane1", "t1"));
            this.model.space_mut("root").unwrap().tabs.push(crate::model::Tab::new("tab2", "", "pane2", "t2"));
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{"size":[260,160]},"rect":[100,120,260,160],"z":0},
                {"id":"t2","kind":"terminal","props":{"size":[260,160]},"rect":[450,120,260,160],"z":1},
                {"id":"p1","kind":"image","props":{"path":"/missing.png"},"rect":[450,350,300,180],"z":2}
            ])).unwrap();
            for t in ["t1", "t2"] { this.live.insert(t.into(), json!({"state":"live", "pid":1})); }
            this.sync(window, cx);
            cx.notify();
        });
        cx.run_until_parked();

        let origin = view.update(cx, |this, _| this.bounds.borrow()["canopy-canvas"].origin);
        let at = |x, y| origin + point(px(x), px(y));
        let (handle, terminal, image) = (at(360., 200.), at(550., 205.), at(550., 410.));
        let none = Modifiers::default();
        let left = MouseButton::Left;

        cx.simulate_mouse_move(at(230., 190.), None, none);
        cx.simulate_click(handle, none);
        assert!(view.read_with(cx, |this, _| this.canvas_state.wire.sticky));
        cx.simulate_mouse_move(terminal, None, none);
        assert_eq!(view.read_with(cx, |this, _| this.canvas_state.wire.drag.as_ref().map(|d| d.2)), Some(terminal));
        cx.simulate_click(terminal, none);
        assert!(view.read_with(cx, |this, _| this.canvas_state.wire.drag.is_none() && !this.canvas_state.wire.sticky));
        assert!(view.read_with(cx, |this, _| this.canvas_state.doc.wires.iter().any(|w| w.a == "t1" && w.b == "t2")));

        cx.simulate_mouse_move(at(230., 190.), None, none);
        cx.simulate_mouse_down(handle, left, none);
        cx.simulate_mouse_move(image, left, none);
        cx.simulate_mouse_up(image, left, none);
        assert!(view.read_with(cx, |this, _| this.canvas_state.wire.drag.is_none()));
        assert!(view.read_with(cx, |this, _| this.canvas_state.doc.wires.iter().any(|w| w.a == "t1" && w.b == "p1")));
    }
}

#[cfg(test)]
mod pulse_tests {
    use super::status_pulses;

    #[test]
    fn only_a_changed_word_on_a_drawn_edge_glows() {
        assert!(status_pulses(Some("working"), Some("done"), true));
        assert!(status_pulses(None, Some("working"), true));
        assert!(!status_pulses(Some("working"), Some("working"), true));
        assert!(!status_pulses(Some("working"), Some("done"), false));
    }
}
