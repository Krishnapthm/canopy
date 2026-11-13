//! Wire on the canvas: drawing edges, their marks, the thread in the file panel and the
//! postman. The protocol itself is `crate::wire`; docs/wire.md describes both.
use super::*;
use crate::wire::{self, Line, Stat};
use gpui_base::input::{InputEvent, InputState};

/// Diameter of the round "+" shown at the middle of each side of a hovered cell.
const HANDLE: f32 = 10.;
/// How long an edge glows after a postcard is typed into its end.
const PULSE: Duration = Duration::from_millis(700);
/// How near (screen px) a click must be to an edge to take it.
const GRAB: f32 = 6.;

/// What the canvas remembers about Wire between frames.
#[derive(Default)]
pub(in crate::window) struct WireUi {
    /// The cell whose side "+" handles show.
    hover: Option<String>,
    /// A wire being drawn: the cell it starts at, where it starts (world) and the pointer (window).
    drag: Option<(String, canvas::Point, Point<Pixels>)>,
    pub(super) selected: Option<(String, String)>,
    pulses: HashMap<(String, String), Instant>,
    stats: HashMap<(String, String), Stat>,
    watch: Option<Task<()>>,
    /// The cells a line typed in the open thread goes to, and its input.
    thread: Option<(Vec<String>, Entity<InputState>, Subscription)>,
}

/// One edge as drawn: its cells in the order stored (a is the parent of a born edge), and its route.
struct Edge {
    key: (String, String),
    born: bool,
    path: Vec<canvas::Point>,
}

/// The pointer's line while a wire is drawn: a cubic leaving `a` sideways.
fn bezier(a: canvas::Point, z: canvas::Point) -> [canvas::Point; 2] {
    [[(a[0] + z[0]) / 2., a[1]], [(a[0] + z[0]) / 2., z[1]]]
}

impl AppWindow {
    fn wire_toast(&mut self, text: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":text.into()}}), window, cx);
    }

    /// A cell's name for people: a sub-agent's, else its pill's.
    fn wire_name(&self, key: &str) -> String {
        match wire::sub_parts(key) {
            Some((parent, id)) => self.canvas_state.subs.get(parent).into_iter().flatten().find(|s| s.id == id).map_or_else(|| format!("sub {id}"), |s| s.name.clone()),
            None => self.pill_name(key),
        }
    }

    /// The thread file of the space a cell is in.
    fn wire_thread(&self, key: &str) -> std::path::PathBuf {
        wire::thread_path(self.canvas_state.doc.space_of(key).map(|s| s.space.as_str()))
    }

    /// The drawn rect of a cell the current view shows.
    fn wire_cell(&self, doc: &Document, key: &str) -> Option<canvas::Rect> {
        let card = doc.cards.iter().find(|c| c.key() == key && doc.shows(c.space.as_ref()))?;
        let view = &self.canvas_state.doc.view;
        if card.sub.is_some() {
            return (*view != canvas::View::Terminals).then_some(card.rect);
        }
        if !canvas::view_allows(view, self.live.get(&card.terminal).is_some_and(canvas_agent)) { return None; }
        // Match the drawn cell: an expanded one has its own size, a pill its measured one.
        let size = match card.size {
            Some(_) => [card.rect[2].max(canvas::PILL_WIDTH), card.rect[3].max(canvas::HEADER)],
            None => self.canvas_state.pill_sizes.borrow().get(&card.terminal).map(|v| v.3).unwrap_or([240., 36.]),
        };
        Some([card.rect[0], card.rect[1], size[0], size[1]])
    }

    /// Every edge between shown cells: drawn wires, and born ones (sub-agents, `hc run` children).
    fn wire_edges(&self) -> Vec<Edge> {
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let mut out: Vec<Edge> = vec![];
        let all = wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        // Every shown cell, for edges to go around.
        let cells: Vec<_> = doc.cards.iter().filter_map(|c| self.wire_cell(doc, &c.key())).collect();
        for w in all.iter().filter(|w| w.born).chain(all.iter().filter(|w| !w.born)) {
            if out.iter().any(|e| (e.key.0 == w.a && e.key.1 == w.b) || (e.key.0 == w.b && e.key.1 == w.a)) { continue; }
            if let (Some(a), Some(b)) = (self.wire_cell(doc, &w.a), self.wire_cell(doc, &w.b)) {
                out.push(Edge { key: (w.a.clone(), w.b.clone()), born: w.born, path: canvas::route(a, b, &cells) });
            }
        }
        out
    }

    /// A window position in canvas-local pixels.
    fn wire_local(&self, position: Point<Pixels>) -> canvas::Point {
        let origin = self.bounds.borrow().get("canopy-canvas").map(|b| b.origin).unwrap_or_default();
        [f32::from(position.x - origin.x), f32::from(position.y - origin.y)]
    }

    /// The topmost shown cell under a window position, with `slop` extra screen pixels around it.
    fn wire_cell_at(&self, position: Point<Pixels>, slop: f32) -> Option<String> {
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let camera = doc.camera;
        let p = camera.to_world(self.wire_local(position));
        let s = slop / camera.zoom;
        doc.cards.iter().filter(|c| {
            self.wire_cell(doc, &c.key()).is_some_and(|r| p[0] >= r[0] - s && p[0] <= r[0] + r[2] + s && p[1] >= r[1] - s && p[1] <= r[1] + r[3] + s)
        }).max_by_key(|c| c.z).map(|c| c.key())
    }

    fn wire_edge_at(&self, position: Point<Pixels>) -> Option<Edge> {
        let camera = self.canvas_state.doc.camera;
        let p = self.wire_local(position);
        self.wire_edges().into_iter().find(|e| canvas::path_distance(&e.path.iter().map(|v| camera.to_screen(*v)).collect::<Vec<_>>(), p) <= GRAB)
    }

    /// Mouse moves on the canvas: shows the "+" handles of the cell under the pointer and
    /// follows a wire being drawn. True when the move was part of drawing.
    pub(in crate::window) fn wire_pointer(&mut self, e: &MouseMoveEvent, cx: &mut Context<Self>) -> bool {
        if let Some((from, start, _)) = self.canvas_state.wire.drag.take() {
            // The button came up outside the window: nothing was released on a cell, so cancel.
            if e.pressed_button.is_some() { self.canvas_state.wire.drag = Some((from, start, e.position)); }
            cx.notify();
            return true;
        }
        let busy = e.pressed_button.is_some() || self.canvas_state.drag.is_some() || self.canvas_state.rest.is_some();
        let hover = if busy { None } else { self.wire_cell_at(e.position, HANDLE) };
        if hover != self.canvas_state.wire.hover {
            self.canvas_state.wire.hover = hover;
            cx.notify();
        }
        false
    }

    /// A left press on empty canvas: takes an edge under it (selects it, opens its thread).
    pub(in crate::window) fn wire_click(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(edge) = self.wire_edge_at(position) else {
            self.canvas_state.wire.selected = None;
            return false;
        };
        self.canvas_state.selected.clear();
        self.canvas_state.wire.selected = Some(edge.key.clone());
        self.focus.focus(window, cx);
        self.wire_open(vec![edge.key.0, edge.key.1], window, cx);
        cx.notify();
        true
    }

    /// A right press on empty canvas: the edge's menu. True when an edge was under it.
    pub(in crate::window) fn wire_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(edge) = self.wire_edge_at(position) else { return false };
        self.canvas_state.wire.selected = Some(edge.key.clone());
        let tripped = self.canvas_state.wire.stats.get(&edge.key).is_some_and(|s| s.tripped);
        self.open_menu("canvas-wire", json!({"a":edge.key.0,"b":edge.key.1,"born":edge.born,"tripped":tripped}), Some(position), window, cx);
        cx.notify();
        true
    }

    /// The left button came up: a wire being drawn connects to the cell under it, or is dropped.
    pub(in crate::window) fn wire_release(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((from, ..)) = self.canvas_state.wire.drag.take() else { return false };
        if let Some(to) = self.wire_cell_at(position, 0.).filter(|to| *to != from) {
            if let Err(why) = self.wire_connect(&from, &to, window, cx) { self.wire_toast(why, window, cx); }
        }
        cx.notify();
        true
    }

    /// Delete on a selected edge removes it (a selected cell takes the key first).
    pub(in crate::window) fn wire_delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keyboard = self.focus.is_focused(window) && self.canvas_state.selected.is_empty();
        let Some((a, b)) = self.canvas_state.wire.selected.clone().filter(|_| keyboard) else { return false };
        if let Err(why) = self.wire_remove(&a, &b, window, cx) { self.wire_toast(why, window, cx); }
        true
    }

    fn wire_connect(&mut self, a: &str, b: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let doc = &mut self.canvas_state.doc;
        if a == b { return Err("A cell cannot be wired to itself".into()); }
        if !doc.same_space(a, b) { return Err(if ![a, b].iter().all(|k| doc.cards.iter().any(|c| c.key() == *k)) { "No such cell".into() } else { "Same space only for now".into() }); }
        if !doc.connect(a, b) { return Err("Already wired".into()); }
        self.canvas_changed(cx);
        self.refresh_wires(window, cx);
        Ok(())
    }

    fn wire_remove(&mut self, a: &str, b: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let before = self.canvas_state.doc.wires.len();
        self.canvas_state.doc.wires.retain(|w| !w.joins(a, b));
        if before == self.canvas_state.doc.wires.len() { return Err("That wire ends with the cell that started it".into()); }
        self.canvas_state.wire.selected = None;
        self.canvas_changed(cx);
        self.refresh_wires(window, cx);
        Ok(())
    }

    /// Lets the wire carry postcards again: a `reset` line in the thread, which is what the breaker counts from.
    fn wire_reset(&mut self, a: &str, b: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let reset = Line { kind: "reset".into(), from: a.into(), to: b.into(), ..Default::default() };
        wire::append(&self.wire_thread(a), reset).map_err(|e| format!("Could not reset the wire: {e}"))?;
        self.refresh_wires(window, cx);
        Ok(())
    }

    /// The `wire.*` commands.
    pub(in crate::window) fn wire_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(from), to) = (a["a"].as_str(), a["b"].as_str()) else { return self.fail(format!("{name} needs a")) };
        let result = match (name, to) {
            ("wire.open", to) => {
                if !self.canvas_state.doc.cards.iter().any(|c| c.key() == from) { return self.fail("No such cell"); }
                self.wire_open(std::iter::once(from).chain(to).map(str::to_owned).collect(), window, cx);
                Ok(())
            }
            (_, None) => return self.fail(format!("{name} needs b")),
            ("wire.connect", Some(to)) => self.wire_connect(from, to, window, cx),
            ("wire.remove", Some(to)) => self.wire_remove(from, to, window, cx),
            (_, Some(to)) => self.wire_reset(from, to, window, cx),
        };
        if let Err(why) = result { self.fail(why); }
    }

    /// Opens the space's thread in the file panel; what is typed there goes to `targets`.
    pub(in crate::window) fn wire_open(&mut self, targets: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(first) = targets.first() else { return };
        let path = self.wire_thread(first);
        // An empty file to open: the thread of a wire nobody has used yet.
        if let Err(e) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::OpenOptions::new().create(true).append(true).open(&path)) {
            return self.wire_toast(format!("Could not open the thread: {e}"), window, cx);
        }
        self.wire_names();
        let input = cx.new(|cx| InputState::new(window, cx));
        let sub = cx.subscribe_in(&input, window, |this, input, event, window, cx| {
            if !matches!(event, InputEvent::PressEnter { .. }) { return; }
            let text = input.read(cx).value().to_string();
            if text.trim().is_empty() { return; }
            input.update(cx, |input, cx| input.set_value("", window, cx));
            let targets = this.canvas_state.wire.thread.as_ref().map(|t| t.0.clone());
            if let Some(targets) = targets { this.wire_say(&targets, &text, window, cx); }
        });
        self.canvas_state.wire.thread = Some((targets, input, sub));
        self.open_preview(json!({"path":path.to_string_lossy()}), window, cx);
    }

    /// The line to type under the open thread, when the panel shows one.
    pub(in crate::window) fn wire_footer(&self) -> Option<AnyElement> {
        self.canvas_state.file.as_deref().filter(|p| wire::is_thread(p))?;
        let (targets, input, _) = self.canvas_state.wire.thread.as_ref()?;
        let to = targets.iter().map(|k| self.wire_name(k)).collect::<Vec<_>>().join(" and ");
        Some(div().flex_none().flex().flex_col().gap(px(4.)).px(px(12.)).py(px(8.)).border_t_1().border_color(c("border"))
            .child(div().text_size(px(12.)).text_color(c("muted-foreground")).child(format!("Write to {to} as you (Enter sends)")))
            .child(gpui_base::Input::new(input)).into_any_element())
    }

    /// The person writes on a wire: one postcard from `you` to each cell, then the bell.
    pub(in crate::window) fn wire_say(&mut self, to: &[String], text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(why) = wire::refuse(text, self.person.wire_limit) { return self.wire_toast(why, window, cx); }
        for key in to {
            if let Err(e) = wire::append(&self.wire_thread(key), Line::card(wire::PERSON, key, "tell", None, text)) {
                return self.wire_toast(format!("Could not write the postcard: {e}"), window, cx);
            }
            self.wire_ring(key, true, window, cx);
        }
        self.refresh_wires(window, cx);
    }

    /// Records a quote pasted into an agent as a postcard from `you` (already in its prompt).
    pub(in crate::window) fn wire_log_quote(&mut self, addr: &str, quote: &Value, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let header = text.lines().next().unwrap_or_default();
        let note = quote["message"].as_str().map(str::trim).filter(|m| !m.is_empty()).map(|m| format!("\n{m}")).unwrap_or_default();
        let mut line = Line::card(wire::PERSON, addr, "tell", None, &format!("{header}{note}"));
        line.delivered = true;
        if let Err(e) = wire::append(&self.wire_thread(addr), line) { self.wire_toast(format!("Could not log the quote: {e}"), window, cx); }
        self.refresh_wires(window, cx);
    }

    /// Cell names for thread text, kept where the panel's reader can reach them.
    fn wire_names(&self) {
        wire::set_names(self.canvas_state.doc.cards.iter().map(|c| (c.key(), self.wire_name(&c.key()))).collect());
    }

    /// Types waiting postcards into a cell if its door is open (`announce`: tell the person when not).
    fn wire_ring(&mut self, key: &str, announce: bool, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = wire::sub_parts(key).map_or(key, |(parent, _)| parent).to_owned();
        let Some(rec) = self.live.get(&terminal) else {
            if announce { self.wire_toast(format!("{} is not running: the postcard waits", self.wire_name(key)), window, cx); }
            return;
        };
        let door = wire::Door { terminal: terminal.clone(), state: rec["status"].as_str().filter(|s| !s.is_empty()).map(str::to_owned), queue: crate::programs::queues(&rec["program"]) };
        let (path, key) = (self.wire_thread(key), key.to_owned());
        let names: HashMap<String, String> = self.canvas_state.doc.cards.iter().map(|c| (c.key(), self.wire_name(&c.key()))).collect();
        cx.spawn_in(window, async move |this, cx| {
            let sent = cx.background_executor().spawn(async move {
                wire::ring(&path, &key, &door, &|k| names.get(k).cloned().unwrap_or_else(|| k.to_owned()), &wire::hc_send)
            }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                match &sent {
                    wire::Sent::Failed(_) => this.wire_toast(format!("Wire: {}", sent.say()), window, cx),
                    wire::Sent::NotRunning | wire::Sent::Queued if announce => this.wire_toast(format!("Wire: {}", sent.say()), window, cx),
                    _ => {}
                }
                this.refresh_wires(window, cx);
            });
        }).detach();
    }

    /// The postman: a terminal reached a turn boundary (idle, or an agent just started in it), so
    /// whatever waits for it, or for its sub-agent cells, is typed now. No timer: the core event rings.
    pub(in crate::window) fn wire_postman(&mut self, terminal: &str, window: &mut Window, cx: &mut Context<Self>) {
        let keys: Vec<String> = self.canvas_state.doc.cards.iter().filter(|c| c.key() == terminal || c.parent.as_deref() == Some(terminal)).map(|c| c.key()).collect();
        for key in keys {
            let lines = wire::read(&self.wire_thread(&key));
            if wire::waiting(&lines, &key).next().is_some() { self.wire_ring(&key, false, window, cx); }
        }
    }

    /// Reads every wire's thread again: counts, open asks and trips, a glow where a postcard
    /// was typed, a toast where a wire tripped. Starts the folder watch once.
    pub(in crate::window) fn refresh_wires(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let doc = &self.canvas_state.doc;
        let wires = wire::all(doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        let mut threads: HashMap<std::path::PathBuf, Vec<Line>> = HashMap::new();
        let mut stats = HashMap::new();
        for w in &wires {
            let path = self.wire_thread(&w.a);
            let lines = threads.entry(path.clone()).or_insert_with(|| wire::read(&path));
            stats.insert((w.a.clone(), w.b.clone()), wire::stat(lines, &w.a, &w.b, self.person.wire_budget));
        }
        let mut tripped = vec![];
        for (key, now) in &stats {
            let Some(was) = self.canvas_state.wire.stats.get(key) else { continue };
            if now.delivered > was.delivered { self.canvas_state.wire.pulses.insert(key.clone(), Instant::now()); }
            if now.tripped && !was.tripped { tripped.push(format!("The wire between {} and {} tripped after {} postcards with no word from you: right-click it to reset", self.wire_name(&key.0), self.wire_name(&key.1), self.person.wire_budget)); }
        }
        self.canvas_state.wire.stats = stats;
        self.wire_names();
        for message in tripped { self.wire_toast(message, window, cx); }
        self.watch_wires(window, cx);
        cx.notify();
    }

    /// Waits for the threads folder to change (one waiting task, no timer) and refreshes.
    fn watch_wires(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.wire.watch.is_some() { return; }
        let dir = wire::thread_path(None).parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        if std::fs::create_dir_all(&dir).is_err() { return; }
        self.canvas_state.wire.watch = Some(cx.spawn_in(window, async move |this, cx| loop {
            let path = dir.to_string_lossy().into_owned();
            let changed = cx.background_executor().spawn(async move { crate::files::next_change(path).await }).await;
            let Ok(changed) = changed else { break };
            if !changed.iter().any(|p| p.ends_with(".jsonl")) { continue; }
            if this.update_in(cx, |this, window, cx| this.refresh_wires(window, cx)).is_err() { break; }
        }));
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
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.canvas_state.selected.clear();
                    this.canvas_state.wire.drag = Some((key.clone(), start, e.position));
                    cx.stop_propagation();
                    cx.notify();
                })).into_any_element());
        }
        out
    }

    /// The edges themselves (a paint layer under the cells) and what sits on them: counts, open
    /// asks, reset dots. Animation frames are asked for only while a glow runs.
    pub(in crate::window) fn wire_layer(&self, viewport: canvas::Point, cx: &mut Context<Self>) -> (AnyElement, Vec<AnyElement>) {
        let state = &self.canvas_state;
        let camera = state.doc.camera;
        let mut paint: Vec<(Vec<canvas::Point>, bool, bool, bool, Option<Instant>)> = vec![];
        let mut marks = vec![];
        for edge in self.wire_edges().into_iter().filter(|e| canvas::edge_visible(camera, &e.path, viewport)) {
            let stat = state.wire.stats.get(&edge.key).cloned().unwrap_or_default();
            let selected = state.wire.selected.as_ref() == Some(&edge.key);
            let pulse = state.wire.pulses.get(&edge.key).copied().filter(|t| t.elapsed() < PULSE);
            let [x, y] = camera.to_screen(canvas::along(&edge.path, 0.5));
            paint.push((edge.path.clone(), edge.born, selected || stat.tripped, stat.tripped, pulse));
            if !(stat.waiting > 0 || stat.unanswered || stat.tripped) { continue; }
            let (a, b) = edge.key.clone();
            let mut row = div().absolute().left(px(x - 6.)).top(px(y - 6.)).flex().items_center().gap(px(4.));
            if stat.tripped {
                row = row.child(div().id(SharedString::from(format!("wire-reset-{}-{}", edge.key.0, edge.key.1))).size(px(HANDLE)).rounded_full().border_1().border_color(c("foreground")).bg(c("surface")).cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Err(why) = this.wire_reset(&a, &b, window, cx) { this.wire_toast(why, window, cx); }
                    })));
            } else if stat.unanswered {
                row = row.child(div().size(px(8.)).rounded_full().border_1().border_color(c("foreground")).bg(c("surface")));
            }
            if stat.waiting > 0 {
                row = row.child(div().px(px(4.)).rounded_full().bg(c("surface")).border_1().border_color(c("muted-foreground")).text_size(px(10.)).line_height(px(12.)).text_color(c("foreground")).child(stat.waiting.to_string()));
            }
            marks.push(row.into_any_element());
        }
        // A wire being drawn follows the pointer, and the cell it would join lights up.
        let live = state.wire.drag.as_ref().map(|(_, start, pointer)| (*start, camera.to_world(self.wire_local(*pointer))));
        let target = state.wire.drag.as_ref().and_then(|(from, _, pointer)| self.wire_cell_at(*pointer, 0.).filter(|k| k != from))
            .and_then(|k| self.wire_cell(state.shown.as_ref().unwrap_or(&state.doc), &k));
        let layer = canvas(|_, _, _| {}, move |b, _, window, cx| {
            let p = |v: [f32; 2]| point(b.origin.x + px(v[0]), b.origin.y + px(v[1]));
            let mut animating = false;
            for (path, born, strong, dashed, pulse) in &paint {
                let screen: Vec<_> = path.iter().map(|v| camera.to_screen(*v)).collect();
                let glow = pulse.map(|t| 1. - t.elapsed().as_secs_f32() / PULSE.as_secs_f32()).filter(|g| *g > 0.);
                animating |= glow.is_some();
                let color = if glow.is_some() { c("ring") } else if *strong { c("foreground").opacity(0.7) } else { c("muted-foreground").opacity(if *born { 0.5 } else { 0.7 }) };
                let width = if glow.is_some() { 2.5 } else if *born { 1. } else { 1.5 };
                if *dashed {
                    // A tripped wire is dashed: every other of 24 short pieces.
                    for i in (0..24).step_by(2) {
                        let mut piece = PathBuilder::stroke(px(width));
                        piece.move_to(p(canvas::along(&screen, i as f32 / 24.)));
                        piece.line_to(p(canvas::along(&screen, (i + 1) as f32 / 24.)));
                        if let Ok(piece) = piece.build() { window.paint_path(piece, color); }
                    }
                } else {
                    // Straight runs with rounded bends.
                    let mut line = PathBuilder::stroke(px(width));
                    line.move_to(p(screen[0]));
                    for w in screen.windows(3) {
                        let (o, v, q) = (w[0], w[1], w[2]);
                        let (din, dout) = (((v[0] - o[0]).abs() + (v[1] - o[1]).abs()), ((q[0] - v[0]).abs() + (q[1] - v[1]).abs()));
                        let r = (8. * camera.zoom).min(din / 2.).min(dout / 2.);
                        let towards = |from: [f32; 2], d: f32| if d > 0. { [v[0] + (from[0] - v[0]) * r / d, v[1] + (from[1] - v[1]) * r / d] } else { v };
                        line.line_to(p(towards(o, din)));
                        line.curve_to(p(towards(q, dout)), p(v));
                    }
                    if let Some(last) = screen.last() { line.line_to(p(*last)); }
                    if let Ok(line) = line.build() { window.paint_path(line, color); }
                }
                let dot = |window: &mut Window, v: [f32; 2], r: f32| window.paint_quad(fill(Bounds::new(p([v[0] - r, v[1] - r]), size(px(2. * r), px(2. * r))), c("muted-foreground")).corner_radii(px(r)));
                if let Some(end) = screen.last() { dot(window, *end, 3.); }
                if !*born && let Some(start) = screen.first() { dot(window, *start, 3.); }
            }
            if let Some((start, world)) = live {
                let (sa, sz) = (camera.to_screen(start), camera.to_screen(world));
                let mut path = PathBuilder::stroke(px(1.5));
                path.move_to(p(sa));
                let [c1, c2] = bezier(sa, sz);
                path.cubic_bezier_to(p(sz), p(c1), p(c2));
                if let Ok(path) = path.build() { window.paint_path(path, c("ring")); }
            }
            if let Some(r) = target {
                let [x, y] = camera.to_screen([r[0], r[1]]);
                window.paint_quad(outline(Bounds::new(p([x, y]), size(px(r[2] * camera.zoom), px(r[3] * camera.zoom))), c("ring"), BorderStyle::Solid).corner_radii(px(8. * camera.zoom)));
            }
            if animating && !cx.reduce_motion() { window.request_animation_frame(); }
        }).absolute().size_full().into_any_element();
        (layer, marks)
    }
}
