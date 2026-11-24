// Modified for Canopy.
use super::*;
use crate::canvas::{self, Document, PILL_WIDTH};
use gpui::AnimationExt as _;

mod wire_view;
mod keyboard;
mod helper_bar;
mod pane_commands;
pub(super) mod browser;
mod browser_mask;
mod annotate;
mod image_pane;
mod pdf;
mod sim;
#[cfg(target_os = "macos")]
mod sim_screen;

/// What an inline name edit renames: a space (by id) or the tab of a cell (by tab id).
#[derive(Clone, PartialEq)]
pub(super) enum Rename {
    Space(String),
    Tab(String),
}

pub(super) struct State {
    pub doc: Document,
    pub(super) mode: bool,
    connecting: Option<String>,
    /// The camera before a focus fit and the camera that fit produced.
    previous_camera: Option<(canvas::Camera, canvas::Camera)>,
    rest: Option<canvas::Rest>,
    pending_layout: Option<(Option<String>, Option<String>)>,
    undo: Option<canvas::Rest>,
    shown: Option<Document>,
    layout_motion: Option<(canvas::Rest, std::time::Instant)>,
    layout_frame: bool,
    /// Latest move event to apply on the next frame or before ending the drag.
    queued_move: Option<Point<Pixels>>,
    /// Last pointer and frame time while a drag is panning at the canvas edge.
    edge_drag: Option<(Point<Pixels>, std::time::Instant)>,
    #[cfg(test)]
    layout_solves: usize,
    pub(super) selected: Vec<String>,
    pub subs: HashMap<String, Vec<crate::subs::Sub>>,
    sub_watches: HashMap<String, Task<()>>,
    previews: HashMap<String, (Entity<gpui_base::TextViewState>, Task<()>)>,
    sims: HashMap<String, sim::SimView>,
    bar_hover: Option<String>,
    // The starting size is present for an edge or corner resize.
    preview_drag: Option<(String, Option<canvas::Point>, Point<Pixels>)>,
    pub file: Option<String>,
    pub(super) editing: Option<String>,
    /// The Files editor reports transitions between saved and unsaved text.
    dirty: Option<String>,
    /// The file panel fills the window (like Claude's expanded side panel); not saved.
    panel_expanded: bool,
    editors: Vec<crate::programs::Editor>,
    pub tabs: Vec<String>,
    /// Space ids, most recently entered first, for ctrl-tab (memory only).
    recent: Vec<String>,
    /// The folder button opened the panel for the file tree, with no tab needed.
    pub files: bool,
    pub popped: HashMap<String, WindowHandle<PopOut>>,
    save: canvas::SaveSchedule,
    pub space_down: bool,
    pub hand: bool,
    resizing: bool,
    writable: bool,
    save_task: Option<Task<()>>,
    write_result: Option<std::sync::mpsc::Receiver<std::io::Result<()>>>,
    pill_sizes: RefCell<HashMap<String, (String, String, u8, canvas::Point)>>,
    pub error: Option<String>,
    pending_points: HashMap<String, canvas::Point>,
    space_drag: Option<(String, bool, Point<Pixels>)>,
    drag: Option<(Option<String>, Point<Pixels>)>,
    /// The current left-button pan moved: its mouse-up is not a click.
    pan_moved: bool,
    motion: Motion,
    /// The name being edited inline: a space label's or a cell header's.
    rename: Option<(Rename, Entity<gpui_base::input::InputState>, Subscription)>,
    /// The preview text a quote popover was opened from, kept highlighted, and that popover's id.
    quote_mark: Option<(Entity<gpui_base::TextViewState>, String)>,
    /// A finished selection waiting behind a small quote icon: the quote (with its
    /// window `point`) and the TextView it came from, None for the Files code viewer.
    quote_chip: Option<(Value, Option<Entity<gpui_base::TextViewState>>)>,
    /// The panel's file and tabs when it was closed, for the folder button to bring back.
    closed: Option<(Option<String>, Vec<String>)>,
    /// Per expanded cell: the layout before it grew, and what growing moved. Collapse puts those back.
    moves: HashMap<String, (canvas::Rest, canvas::Rest)>,
    /// The expanded cell's corner being dragged, and the pointer where it was last seen.
    cell_resize: Option<(String, Point<Pixels>)>,
    /// Sync input: the synced panes. Pruned while drawing to what is still selected (a Cell
    /// because drawing takes &self); fewer than two ends it.
    pub(super) synced: RefCell<Vec<String>>,
    /// The room and origin of the last ⌘T pane, for the next one's cascade step.
    cascade: Option<(Option<String>, canvas::Point)>,
    /// Which edge or corner the pane resize drags (see `canvas::resize_from`).
    resize_zone: (i8, i8),
    /// When the camera zoom last moved by gesture: expanded cells draw flat until it has rested.
    zoomed_at: Option<std::time::Instant>,
    zoom_task: Option<Task<()>>,
    /// When a plain wheel last panned the canvas: that gesture keeps panning across cells (`claims_wheel`).
    wheel_pan_at: Option<std::time::Instant>,
    wheel_zoom: Option<WheelZoom>,
    wire: wire_view::WireUi,
    /// A box being dragged on empty canvas: where it started, where the pointer is, and the
    /// selection it adds to (Shift), in window coordinates.
    marquee: Option<(Point<Pixels>, Point<Pixels>, Vec<String>)>,
    /// The selection a cell drag carries along: the solver keeps them where the drag put them.
    group: Vec<String>,
    /// Held Cmd (Ctrl elsewhere): surface hints use the current bindings.
    pub(super) actions: Option<Vec<(String, String, String)>>,
    /// Pending hold delay, cancelled on release or any key down.
    pub(super) hint_timer: Option<Task<anyhow::Result<()>>>,
    /// Tile view is on (`canvas.tile`): what it fills, and the geometry to put back.
    pub(super) tile: Option<Tile>,
    /// The pressed cell has moved `canvas::DRAG_THRESHOLD` px, so the press is a drag (see `drag_live`).
    drag_live: bool,
    /// Browser panes' web views and snapshots (not saved).
    pub(super) browsers: browser::Browsers,
    /// Most recently used browser pane, for routing terminal links.
    last_browser: Option<String>,
    annotations: HashMap<String, annotate::Annotate>,
    annotation_starts: HashMap<String, Rc<()>>,
    #[cfg(target_os = "macos")]
    annotation_targets: HashSet<String>,
    /// The browser pane expanded into a tab of the top strip, filling the canvas area (not saved).
    pub(super) browser_tab: Option<String>,
    /// An image region drag: pane id, start and current window points.
    region: Option<(String, Point<Pixels>, Point<Pixels>)>,
    /// Pixel sizes of open images, by path.
    image_sizes: HashMap<String, (u32, u32)>,
    /// One viewer per PDF pane, by pane id.
    pdfs: HashMap<String, Entity<gpui_pdf::PdfView>>,
    /// Viewers whose pane closed, released on the next render (release needs a Window).
    pdf_drops: Vec<Entity<gpui_pdf::PdfView>>,
    /// Image paths that failed to load, so each toasts once.
    image_errors: HashSet<String>,
}

/// Tile view of the current place (leaving it calls `State::untile`): the camera from before
/// (its top-left is where the tiles start, at zoom 1) and each member's rect and expanded size
/// from before, by pane id or space id.
pub(super) struct Tile {
    camera: canvas::Camera,
    before: Vec<(String, canvas::Rect, Option<canvas::Point>)>,
    /// Keys of the members in layout order, fixed when tile view starts. A selection of two or
    /// more tiles only those (the others stay put, under them); otherwise `all` is set and cells
    /// opened later join at the end. Closed ones drop out.
    order: Vec<String>,
    all: bool,
}
#[derive(Default)]
struct Motion {
    target: Option<(canvas::Camera, canvas::Camera, std::time::Instant, canvas::Point)>,
    velocity: [f32; 2],
    sample: Option<std::time::Instant>,
    frame: bool,
}

/// Zoom anchor and, on inputs without an end event, its pending quiet-period timer.
struct WheelZoom {
    anchor: canvas::Point,
    timeout: Option<Task<()>>,
}
impl State {
    pub fn load() -> Self {
        let (doc, error, writable) = match Document::load(&file("canvas.json")) {
            Ok((doc, bad)) => (
                doc,
                bad.then(|| "Corrupt canvas.json preserved as canvas.json.bad".into()),
                true,
            ),
            Err(e) => (
                Document::default(),
                Some(format!("Canvas could not load: {e}")),
                false,
            ),
        };
        Self {
            doc,
            // The canvas has the keyboard until a cell is entered (double-click or Enter).
            mode: true,
            connecting: None,
            previous_camera: None,
            rest: None,
            pending_layout: None,
            undo: None,
            shown: None,
            layout_motion: None,
            layout_frame: false,
            queued_move: None,
            edge_drag: None,
            #[cfg(test)]
            layout_solves: 0,
            selected: vec![],
            subs: HashMap::new(),
            sub_watches: HashMap::new(),
            previews: HashMap::new(),
            sims: HashMap::new(),
            bar_hover: None,
            preview_drag: None,
            file: None,
            editing: None,
            dirty: None,
            panel_expanded: false,
            editors: Vec::new(),
            tabs: Vec::new(),
            recent: Vec::new(),
            files: false,
            popped: HashMap::new(),
            save: canvas::SaveSchedule::default(),
            space_down: false,
            hand: false,
            resizing: false,
            writable,
            save_task: None,
            write_result: None,
            pill_sizes: RefCell::new(HashMap::new()),
            error,
            pending_points: HashMap::new(),
            space_drag: None,
            drag: None,
            pan_moved: false,
            motion: Motion::default(),
            rename: None,
            quote_mark: None,
            quote_chip: None,
            closed: None,
            moves: HashMap::new(),
            cell_resize: None,
            resize_zone: (1, 1),
            cascade: None,
            synced: RefCell::new(Vec::new()),
            zoomed_at: None,
            zoom_task: None,
            wheel_pan_at: None,
            wheel_zoom: None,
            wire: Default::default(),
            marquee: None,
            group: vec![],
            actions: None,
            hint_timer: None,
            tile: None,
            drag_live: false,
            browsers: Default::default(),
            last_browser: None,
            annotations: HashMap::new(),
            annotation_starts: HashMap::new(),
            #[cfg(target_os = "macos")]
            annotation_targets: HashSet::new(),
            browser_tab: None,
            region: None,
            image_sizes: HashMap::new(),
            pdfs: HashMap::new(),
            pdf_drops: Vec::new(),
            image_errors: HashSet::new(),
        }
    }
    /// The requested camera before animation finishes, for keyboard integration tests.
    #[cfg(test)]
    pub(super) fn motion_target_for_test(&self) -> canvas::Camera {
        self.motion.target.unwrap().1
    }
    /// Finish a layout ease at once: test windows draw no frames, so it would never end.
    #[cfg(test)]
    pub(super) fn settle_for_test(&mut self) {
        self.layout_motion = None;
        self.shown = None;
    }

    /// Turns tile view off, putting back the camera and geometry from before it. A cell expanded or
    /// collapsed while tiled keeps that and only goes back to its spot; their keys are
    /// returned so the caller can make room for their new size. Call before leaving a place.
    pub(super) fn untile(&mut self) -> Vec<String> {
        let Some(tile) = self.tile.take() else { return vec![] };
        self.doc.camera = tile.camera;
        self.motion = Motion::default();
        let back = |key: &str| tile.before.iter().find(|(k, ..)| k == key);
        let mut flipped = vec![];
        for c in &mut self.doc.panes {
            let Some((_, r, size)) = back(&c.id) else { continue };
            if c.size().is_some() == size.is_some() { c.rect = *r; if let Some(to) = c.size_mut() { *to = *size; } }
            else { c.rect[0] = r[0]; c.rect[1] = r[1]; flipped.push(c.id.clone()); }
        }
        for s in &mut self.doc.spaces { if let Some((_, r, _)) = back(&s.space) { s.rect = *r; } }
        flipped
    }

    /// Whether a cell or preview press, moved by `d` screen px since the press, is a drag. Smaller
    /// movement is a click and moves nothing. The first drag in tile view ends tile view without
    /// putting anything back: cells keep their tiled rects as the free layout and the camera stays.
    pub(super) fn drag_live(&mut self, d: [f32; 2]) -> bool {
        if !self.drag_live {
            if d[0].hypot(d[1]) < canvas::DRAG_THRESHOLD { return false; }
            self.drag_live = true;
            self.tile = None;
        }
        true
    }

    /// The last applied pointer of a pane or space move. Resizes and pans have their own handlers.
    fn move_pointer(&mut self) -> Option<&mut Point<Pixels>> {
        if let Some((_, None, previous)) = &mut self.preview_drag {
            Some(previous)
        } else if let Some((_, _, previous)) = &mut self.space_drag {
            Some(previous)
        } else if let Some((Some(_), previous)) = &mut self.drag {
            Some(previous)
        } else {
            None
        }
    }

    fn resizing_pane(&self) -> bool {
        self.cell_resize.is_some() || matches!(self.preview_drag, Some((_, Some(_), _)))
    }

    fn entered(&self) -> Option<&str> {
        self.selected.last().filter(|_| !self.mode).map(String::as_str)
    }

    /// End any drag, returning whether geometry was committed.
    pub(super) fn end_drag(&mut self) -> bool {
        self.drag_live = false;
        self.queued_move = None;
        self.edge_drag = None;
        let mut changed = false;
        let resized = self.cell_resize.take();
        if let Some(rest) = self.rest.take() {
            if !rest.matches(&self.doc) {
                // Undo would put neighbours back over a cell that stays big, so a corner drag is not undoable.
                match resized {
                    Some((key, _)) => if let Some((base, moved)) = self.moves.get_mut(&key) { *moved = base.changed(&canvas::Rest::capture(&self.doc)); },
                    None => self.undo = Some(rest),
                }
                self.save.dirty = true;
                changed = true;
            }
            if self.layout_motion.is_none() { self.shown = None; }
        }
        self.preview_drag = None;
        self.drag = None;
        self.group.clear();
        self.space_drag = None;
        self.resizing = false;
        changed
    }
    pub fn flush(&mut self) {
        self.save_task = None;
        self.save.pending = false;
        if let Some(result) = self.write_result.take() {
            let _ = result.recv();
            self.save.dirty = true;
        }
        if self.save.dirty && self.writable {
            match self.doc.save(&file("canvas.json")) {
                Ok(()) => {
                    self.save.dirty = false;
                }
                Err(e) => {
                    self.error = Some(format!("Canvas could not save: {e}"));
                }
            }
        }
    }
}
pub(super) struct PopOut {
    pub(super) terminal: Entity<TerminalView>,
    _sub: Option<Subscription>,
}
impl Render for PopOut {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(c("surface"))
            .child(shown_terminal(&self.terminal, cx))
    }
}
impl AppWindow {
    /// A cell's own menu (`canvas-card` in the menu mod): from its pill, header or terminal.
    pub(super) fn open_card_menu(&mut self, addr: &str, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.canvas_state.selected.iter().any(|s| s == addr) { self.canvas_state.selected = vec![addr.to_owned()]; }
        let tab = self.model.pane_of(addr).and_then(|p| self.model.tab_of(&p)).map(|t| t.id.clone());
        let target = json!({"terminal":addr,"id":tab,"popped":self.canvas_state.popped.contains_key(addr),
            "expanded":self.canvas_state.doc.panes.iter().any(|c| c.id == addr && c.size().is_some()),
            "finished":self.canvas_state.subs.get(addr).is_some_and(|subs| subs.iter().any(|s| s.done))});
        self.open_menu("canvas-card", target, Some(at), window, cx);
    }

    fn begin_layout(&mut self) {
        self.canvas_state.drag_live = false;
        self.canvas_state.queued_move = None;
        self.canvas_state.edge_drag = None;
        self.canvas_state.motion.target = None;
        self.canvas_state.motion.velocity = [0.; 2];
        if self.canvas_state.rest.is_none() {
            self.canvas_state.rest = Some(canvas::Rest::capture(&self.canvas_state.doc));
        }
    }

    fn canvas_members(&self) -> Vec<Option<String>> {
        self.canvas_state.doc.panes.iter().map(|c| {
            let addr = match &c.kind {
                canvas::Kind::File { .. } | canvas::Kind::Mod { .. } => return c.space.clone(),
                canvas::Kind::Image { .. } | canvas::Kind::Pdf { .. } | canvas::Kind::Sim { .. } | canvas::Kind::Browser { .. } => return c.space.clone(),
                canvas::Kind::Sub { parent, .. } => parent,
                canvas::Kind::Terminal { .. } => &c.id,
            };
            cell_space(&self.model, &self.live, addr).filter(|s| !root_space(s)).map(|s| s.id.clone())
        }).collect()
    }

    /// Keep pointer handling cheap even when several move events arrive between frames.
    fn queue_canvas_move(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let state = &mut self.canvas_state;
        let Some(&mut previous) = state.move_pointer() else { return false };
        let delta = [f32::from(position.x - previous.x), f32::from(position.y - previous.y)];
        if state.space_drag.is_none() && !state.drag_live(delta) { return true; }
        if state.queued_move.unwrap_or(previous) == position { return true; }
        state.pan_moved |= delta != [0., 0.];
        state.queued_move = Some(position);
        self.layout_frame(window, cx);
        true
    }

    /// Move the carried panes directly. Only neighbours need to ease to their new positions.
    fn apply_canvas_move(&mut self, pan: canvas::Point, cx: &mut Context<Self>) -> bool {
        let state = &mut self.canvas_state;
        let position = state.queued_move.take();
        let Some(previous) = state.move_pointer() else { return false };
        let position = position.unwrap_or(*previous);
        let delta = [f32::from(position.x - previous.x), f32::from(position.y - previous.y)];
        *previous = position;
        if delta == [0., 0.] && pan == [0., 0.] { return false; }

        let pane = state.preview_drag.as_ref().map(|(id, _, _)| id)
            .or_else(|| state.drag.as_ref().and_then(|(id, _)| id.as_ref())).cloned();
        let space = state.space_drag.as_ref().map(|(id, _, _)| id.clone());
        let zoom = state.doc.camera.zoom;
        let world = [(delta[0] - pan[0]) / zoom, (delta[1] - pan[1]) / zoom];
        state.shown.get_or_insert_with(|| state.doc.clone());
        if let Some(id) = &pane {
            state.group = if state.preview_drag.is_none() && state.selected.contains(id) {
                state.selected.clone()
            } else {
                vec![id.clone()]
            };
            for card in state.doc.panes.iter_mut().filter(|card| state.group.contains(&card.id)) {
                let (rect, d) = if let Some(pinned) = &mut card.pinned {
                    (pinned, delta)
                } else {
                    (&mut card.rect, world)
                };
                rect[0] += d[0];
                rect[1] += d[1];
            }
        } else if let Some(card) = state.doc.spaces.iter_mut().find(|s| Some(&s.space) == space.as_ref()) {
            state.group.clear();
            card.rect[0] += world[0];
            card.rect[1] += world[1];
        }
        self.solve_layout(pane.as_deref(), space.as_deref(), cx);
        let state = &mut self.canvas_state;
        let shown = state.shown.as_mut().unwrap();
        for card in state.doc.panes.iter().filter(|card| state.group.contains(&card.id)) {
            if let Some(drawn) = shown.panes.iter_mut().find(|p| p.id == card.id) {
                drawn.rect = card.rect;
                drawn.pinned = card.pinned;
            }
        }
        if let Some(card) = state.doc.spaces.iter().find(|s| Some(&s.space) == space.as_ref())
            && let Some(drawn) = shown.spaces.iter_mut().find(|s| s.space == card.space)
        {
            drawn.rect = card.rect;
        }
        self.retarget_layout();
        self.canvas_changed(cx);
        true
    }

    fn end_canvas_drag(&mut self, cx: &mut Context<Self>) {
        self.apply_canvas_move([0., 0.], cx);
        if let Some((pin, space)) = self.canvas_state.pending_layout.take() {
            self.solve_layout_now(pin.as_deref(), space.as_deref(), cx);
        }
        let resize = self.canvas_state.preview_drag.as_ref().and_then(|(id, before, _)| {
            let before = (*before)?;
            let pane = self.canvas_state.doc.panes.iter().find(|p| &p.id == id)?;
            let [_, _, w, h] = pane.pinned.unwrap_or(pane.rect);
            (before != [w, h]).then(|| id.clone())
        });
        self.canvas_state.end_drag();
        if let Some(id) = resize { self.resize_resting_browser(&id, cx); }
    }

    fn canvas_overlay_at(&self, position: Point<Pixels>) -> bool {
        !self.overlays.is_empty() && self.bounds.borrow().iter().any(|(id, bounds)| {
            id.starts_with("cover:overlay:") && bounds.contains(&position)
        })
    }

    /// Resize the held edge in world pixels, or in screen pixels for a pinned pane.
    fn apply_resize(&mut self, position: Point<Pixels>, pan: canvas::Point, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        let (id, previous) = if let Some((id, previous)) = &mut state.cell_resize {
            (id, previous)
        } else if let Some((id, Some(_), previous)) = &mut state.preview_drag {
            (id, previous)
        } else {
            return;
        };
        let screen = [f32::from(position.x - previous.x), f32::from(position.y - previous.y)];
        *previous = position;
        if screen == [0., 0.] && pan == [0., 0.] { return; }

        let id = id.clone();
        let z = state.doc.camera.zoom;
        let world = [(screen[0] - pan[0]) / z, (screen[1] - pan[1]) / z];
        state.shown.get_or_insert_with(|| state.doc.clone());
        let Some(card) = state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
        let zone = state.resize_zone;
        if let Some(size) = card.size() {
            let (origin, size) = canvas::resize_from([card.rect[0], card.rect[1]], size, zone, world, [canvas::PILL_WIDTH * 1.5, canvas::HEADER * 3.]);
            card.rect[..2].copy_from_slice(&origin);
            *card.size_mut().unwrap() = Some(size);
        } else {
            let (rect, delta) = match &mut card.pinned {
                Some(rect) => (rect, screen),
                None => (&mut card.rect, world),
            };
            let [x, y, w, h] = *rect;
            let ([x, y], [w, h]) = canvas::resize_from([x, y], [w, h], zone, delta, [240., 120.]);
            *rect = [x, y, w, h];
        }
        self.solve_layout(Some(&id), None, cx);
        self.animate_layout(window, cx);
        self.canvas_changed(cx);
    }

    /// Resolve cell and space spacing, deferring neighbours until release in drop mode.
    pub(super) fn solve_layout(&mut self, pin: Option<&str>, space_pin: Option<&str>, cx: &App) {
        // A pinned pane floats over the canvas: it neither pushes nor gets pushed.
        if pin.is_some_and(|id| self.canvas_state.doc.panes.iter().any(|p| p.id == id && p.pinned.is_some())) { return; }
        if self.person.canvas_push == settings::CanvasPush::Drop
            && (self.canvas_state.drag.is_some() || self.canvas_state.cell_resize.is_some()
                || self.canvas_state.preview_drag.is_some() || self.canvas_state.space_drag.is_some()) {
            self.canvas_state.pending_layout = Some((pin.map(str::to_owned), space_pin.map(str::to_owned)));
            return;
        }
        self.canvas_state.pending_layout = None;
        self.solve_layout_now(pin, space_pin, cx);
    }

    fn solve_layout_now(&mut self, pin: Option<&str>, space_pin: Option<&str>, cx: &App) {
        #[cfg(test)]
        { self.canvas_state.layout_solves += 1; }
        let members = self.canvas_members();
        let sizes: Vec<_> = self
            .canvas_state
            .doc
            .panes
            .iter()
            .map(|c| self.pill_rect(c, cx))
            .collect();
        let state = &mut self.canvas_state;
        let pinned = pin.and_then(|id| state.doc.panes.iter().position(|p| p.id == id));
        let current: Vec<_> = state
            .doc
            .panes
            .iter()
            .map(|c| c.rect)
            .collect();
        if let Some(rest) = &state.rest {
            rest.restore(&mut state.doc);
        }
        let doc = &mut state.doc;
        let mut cells: Vec<_> = doc
            .panes
            .iter()
            .zip(&sizes)
            .map(|(c, r)| [c.rect[0], c.rect[1], r[2], r[3]])
            .collect();
        // The pinned cell, and the rest of a dragged selection, stay where they were put.
        let carried = |i: usize| state.group.contains(&doc.panes[i].id);
        for i in (0..cells.len()).filter(|i| Some(*i) == pinned || carried(*i)) {
            cells[i] = current[i];
            cells[i][2] = sizes[i][2];
            cells[i][3] = sizes[i][3];
        }
        // Push off lets cells overlap; spaces and root cells on Home still separate below.
        if let (Some(i), false) = (pinned, self.person.canvas_push == settings::CanvasPush::Off) {
            let indices: Vec<_> = (0..cells.len())
                .filter(|j| members[*j] == members[i] && doc.panes[*j].pinned.is_none())
                .collect();
            let rects: Vec<_> = indices.iter().map(|j| cells[*j]).collect();
            let solved = canvas::solve(
                &rects,
                indices.iter().position(|j| *j == i).unwrap(),
                cells[i],
                self.person.canvas_gap,
                |j| carried(indices[j]),
            );
            for (j, r) in indices.into_iter().zip(solved) {
                cells[j] = r;
            }
        }
        let active = space_pin.or_else(|| pinned.and_then(|i| members[i].as_deref()));
        // On Home, root cells and space cards share one level, so neither covers the other.
        let roots: Vec<_> = (0..cells.len()).filter(|i|members[*i].is_none() && doc.panes[*i].pinned.is_none()).collect();
        let rects: Vec<_> = doc.spaces.iter().map(|s|s.rect).chain(roots.iter().map(|i|cells[*i])).collect();
        let top_pin = doc.spaces.iter().position(|s|Some(s.space.as_str())==active)
            .or_else(||pinned.and_then(|i|roots.iter().position(|j|*j==i)).map(|i|doc.spaces.len()+i));
        if let Some(i) = top_pin {
            let space_count = doc.spaces.len();
            let solved = canvas::solve(&rects, i, rects[i], self.person.canvas_gap, |j| {
                j.checked_sub(space_count).is_some_and(|j| carried(roots[j]))
            });
            for (space,r) in doc.spaces.iter_mut().zip(&solved) { space.rect = *r; }
            for (i,r) in roots.iter().zip(&solved[doc.spaces.len()..]) { cells[*i] = *r; }
        }
        for (r, to) in doc
            .panes
            .iter_mut()
            .map(|c| &mut c.rect)
            .zip(cells)
        {
            *r = to;
        }
    }

    fn animate_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.retarget_layout() {
            self.layout_frame(window, cx);
        } else {
            self.canvas_changed(cx);
        }
        cx.notify();
    }

    fn retarget_layout(&mut self) -> bool {
        let state = &mut self.canvas_state;
        let from = state.shown.as_ref().map(canvas::Rest::capture);
        state.layout_motion = from.filter(|from| !from.matches(&state.doc))
            .map(|from| (from, std::time::Instant::now()));
        if state.layout_motion.is_none() { state.shown = None; }
        state.layout_motion.is_some()
    }

    fn layout_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.layout_frame {
            return;
        }
        self.canvas_state.layout_frame = true;
        let entity = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.canvas_state.layout_frame = false;
                let mut pan = [0., 0.];
                if let Some((position, last)) = this.canvas_state.edge_drag.take() {
                    let bounds = this.bounds.borrow().get("canopy-canvas").copied();
                    let speed = bounds.map_or([0., 0.], |bounds| edge_pan(position, bounds));
                    if speed != [0., 0.] {
                        let now = std::time::Instant::now();
                        pan = speed.map(|v| v * now.duration_since(last).as_secs_f32().min(0.05));
                        this.canvas_state.doc.camera.pan(pan);
                        this.canvas_state.edge_drag = Some((position, now));
                        this.apply_resize(position, pan, window, cx);
                        this.layout_frame(window, cx);
                    }
                }
                let moved = this.apply_canvas_move(pan, cx);
                let state = &mut this.canvas_state;
                let Some((from, start)) = &state.layout_motion else {
                    if moved || pan != [0., 0.] { cx.notify(); }
                    return;
                };
                let t = start.elapsed().as_secs_f32() / 0.12;
                let a = canvas::ease_out(t);
                let mut shown = state.doc.clone();
                from.restore(&mut shown);
                for (r, to) in shown
                    .panes
                    .iter_mut()
                    .map(|c| &mut c.rect)
                    .zip(state.doc.panes.iter().map(|c| c.rect))
                    .chain(
                        shown
                            .spaces
                            .iter_mut()
                            .map(|s| &mut s.rect)
                            .zip(state.doc.spaces.iter().map(|s| s.rect)),
                    )
                {
                    for k in 0..4 {
                        r[k] += (to[k] - r[k]) * a;
                    }
                }
                if let Some((Some(id), _)) = &state.drag
                    && let (Some(c), Some(target)) = (
                        shown.panes.iter_mut().find(|c| &c.id == id),
                        state.doc.panes.iter().find(|c| &c.id == id),
                    ) {
                    c.rect = target.rect;
                }
                // The corner follows the pointer; the cells it pushes ease away.
                if let Some((id, _)) = &state.cell_resize
                    && let (Some(c), Some(target)) = (
                        shown.panes.iter_mut().find(|c| &c.id == id),
                        state.doc.panes.iter().find(|c| &c.id == id),
                    ) {
                    c.rect = target.rect;
                }
                if let Some((id, _, _)) = &state.preview_drag
                    && let (Some(p), Some(target)) = (
                        shown.panes.iter_mut().find(|p| &p.id == id),
                        state.doc.panes.iter().find(|p| &p.id == id),
                    ) {
                    p.rect = target.rect;
                }
                if let Some((id, _, _)) = &state.space_drag {
                    if let (Some(s), Some(target)) = (
                        shown.spaces.iter_mut().find(|s| &s.space == id),
                        state.doc.spaces.iter().find(|s| &s.space == id),
                    ) {
                        s.rect = target.rect;
                    }
                }
                state.shown = Some(shown);
                if t < 1. {
                    this.layout_frame(window, cx);
                } else {
                    state.layout_motion = None;
                    state.shown = None;
                    if state.rest.is_none() {
                        this.canvas_changed(cx);
                    }
                }
                cx.notify();
            });
        });
    }

    /// Arrange selected cells or the current space, remembering the layout choice.
    pub(super) fn arrange_canvas(
        &mut self,
        args: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if args["picker"] == true {
            self.open_menu(
                "canvas-arrange",
                json!({"space":args["space"]}),
                None,
                window,
                cx,
            );
            return;
        }
        let layout = args["layout"]
            .as_str()
            .unwrap_or(if self.canvas_state.doc.arrange_layout.is_empty() {
                "tidy"
            } else {
                &self.canvas_state.doc.arrange_layout
            })
            .to_owned();
        if !["tidy", "row", "column", "wires"].contains(&layout.as_str()) {
            self.command_error("Arrange", "Use tidy, row, column or wires", window, cx);
            return;
        }
        self.end_canvas_drag(cx);
        let members = self.canvas_members();
        let doc = &self.canvas_state.doc;
        let ids: Vec<_> = doc
            .panes
            .iter()
            .map(|c| c.id.clone())
            .collect();
        let all: Vec<_> = doc
            .panes
            .iter()
            .map(|c| self.pill_rect(c, cx))
            .collect();
        // The named space, else the room being shown (None arranges Home's own cells).
        let space = args["space"].as_str().map(str::to_owned).or_else(|| doc.room.clone());
        let picked = self.canvas_state.selected.last().and_then(|id| ids.iter().position(|k| k == id));
        let indices: Vec<_> = (0..ids.len())
            .filter(|i| {
                if args["space"].as_str().is_none() && let Some(last) = picked {
                    // A selection never spans spaces: each cell stays in its own.
                    self.canvas_state.selected.contains(&ids[*i]) && members[*i] == members[last]
                } else {
                    members[*i] == space
                }
            })
            .collect();
        if indices.is_empty() {
            return;
        }
        let rects: Vec<_> = indices.iter().map(|i| all[*i]).collect();
        let parents: Vec<_> = indices
            .iter()
            .map(|i| {
                doc.panes.get(*i)
                    .and_then(|c| match &c.kind { canvas::Kind::Sub { parent, .. } => Some(parent.as_str()), canvas::Kind::Terminal { .. } => self.live.get(&c.id)?["parent"].as_str(), canvas::Kind::File { .. } | canvas::Kind::Mod { .. } | canvas::Kind::Image { .. } | canvas::Kind::Pdf { .. } | canvas::Kind::Sim { .. } | canvas::Kind::Browser { .. } => None })
                    .and_then(|p| indices.iter().position(|j| ids[*j] == p))
            })
            .collect();
        // Drawn wires and born parents both keep cells together.
        let at = |key: &str| indices.iter().position(|j| ids[*j] == key);
        let links: Vec<_> = doc.wires.iter().filter_map(|w| Some((at(&w.a)?, at(&w.b)?)))
            .chain(parents.iter().enumerate().filter_map(|(i, p)| Some((i, (*p)?))))
            .collect();
        let arranged = canvas::arrange(&rects, &parents, &links, &layout);
        let before = self.canvas_state.doc.clone();
        self.canvas_state.undo = Some(canvas::Rest::capture(&before));
        for (i, r) in indices.iter().zip(arranged) {
            self.canvas_state.doc.panes[*i].rect = r;
        }
        self.solve_layout(Some(&ids[indices[0]]), None, cx);
        self.canvas_state.doc.arrange_layout = layout;
        self.canvas_state.shown = Some(before);
        self.animate_layout(window, cx);
    }

    /// Restore the one geometry snapshot without changing terminal state.
    pub(super) fn undo_canvas(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.end_canvas_drag(cx);
        if let Some(rest) = self.canvas_state.undo.take() {
            let before = self.canvas_state.doc.clone();
            rest.restore(&mut self.canvas_state.doc);
            self.canvas_state.shown = Some(before);
            self.animate_layout(window, cx);
        }
    }
    /// `canvas.tile` (`toggle`): turns tile view on, fitting the selected cells (two or more) or else
    /// the current place's expanded cells and previews into the visible canvas at zoom 1, or off,
    /// putting everything back. With
    /// `toggle` false it only re-tiles when tile view is on and the layout would change: the
    /// active (last selected) tile, the members or the visible size moved, or a tile was nudged.
    pub(super) fn tile_canvas(&mut self, toggle: bool, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        if toggle && state.tile.is_some() {
            self.end_canvas_drag(cx);
            let state = &mut self.canvas_state;
            let (before, from) = (state.doc.clone(), state.doc.camera);
            let flipped = state.untile();
            state.shown = Some(before);
            // Ease the camera back too.
            let to = std::mem::replace(&mut state.doc.camera, from);
            let visible = self.bounds.borrow().get("canopy-canvas").map_or([0., 0.], |b| [f32::from(b.size.width), f32::from(b.size.height)]);
            self.canvas_state.motion.target = Some((from, to, std::time::Instant::now(), [visible[0] / 2., visible[1] / 2.]));
            self.animate_camera(window, cx);
            for key in flipped { self.solve_layout(Some(&key), None, cx); }
            self.animate_layout(window, cx);
            return cx.notify();
        }
        if !toggle && state.tile.is_none() { return; }
        if state.drag.is_some() || state.cell_resize.is_some() || state.preview_drag.is_some() || state.space_drag.is_some() { return; }
        let visible = self.bounds.borrow().get("canopy-canvas").map(|b| [f32::from(b.size.width), f32::from(b.size.height)])
            .unwrap_or([f32::from(window.viewport_size().width), f32::from(window.viewport_size().height)]);
        let doc = &self.canvas_state.doc;
        let camera = self.canvas_state.motion.target.map_or(doc.camera, |t| t.1);
        let area = self.canvas_state.tile.as_ref().map_or(camera, |t| t.camera);
        let area = [area.x, area.y];
        // What the place shows: expanded cells and previews tile; pills (and space cards on Home) line up.
        let shown = |c: &canvas::Pane| doc.shows(c.space.as_ref()) && c.pinned.is_none()
            && (c.boxed() || matches!(c.kind, canvas::Kind::Sub { .. }) || canvas::view_allows(&doc.view, self.live.get(&c.id).is_some_and(canvas_agent)));
        let cards: Vec<usize> = (0..doc.panes.len()).filter(|i| shown(&doc.panes[*i])).collect();
        let (tiled, pilled): (Vec<usize>, Vec<usize>) = cards.into_iter().partition(|i| doc.panes[*i].boxed() || doc.panes[*i].size().is_some());
        let spaces: Vec<usize> = (0..doc.spaces.len()).filter(|i| doc.room.is_none()
            && !self.model.spaces.iter().any(|s| s.id == doc.spaces[*i].space && root_space(s))).collect();
        let count = doc.panes.len();
        // (id, rect, index): an index below `count` is a pane, above it a space.
        let mut tiles: Vec<(String, canvas::Rect, usize)> = tiled.iter().map(|i| (doc.panes[*i].id.clone(), doc.panes[*i].rect, *i)).collect();
        let mut lined_up: Vec<(String, canvas::Rect, usize)> = pilled.iter().map(|i| (doc.panes[*i].id.clone(), doc.panes[*i].rect, *i))
            .chain(spaces.iter().map(|i| (doc.spaces[*i].space.clone(), doc.spaces[*i].rect, count + i))).collect();
        // Who is tiled and in what order is fixed when tile view starts (a selection of two or more,
        // else everything, in reading order); later layouts keep it, so a click or a jitter never swaps tiles.
        let started = self.canvas_state.tile.is_none();
        let (mut order, all) = match &self.canvas_state.tile {
            Some(t) => (t.order.clone(), t.all),
            None => {
                let every: Vec<_> = tiles.iter().chain(&lined_up).collect();
                let chosen: Vec<&String> = self.canvas_state.selected.iter().filter(|k| every.iter().any(|x| &x.0 == *k)).collect();
                let all = chosen.len() < 2;
                let members: Vec<_> = every.iter().filter(|x| all || chosen.contains(&&x.0)).collect();
                let rects: Vec<_> = members.iter().map(|x| x.1).collect();
                (canvas::reading(&rects).into_iter().map(|i| members[i].0.clone()).collect(), all)
            }
        };
        order.retain(|k| tiles.iter().chain(&lined_up).any(|x| &x.0 == k));
        // Opened cells join a tile view that started with everything.
        if all { for x in tiles.iter().chain(&lined_up) { if !order.contains(&x.0) { order.push(x.0.clone()); } } }
        let rank = |k: &String| order.iter().position(|o| o == k);
        tiles.retain(|x| rank(&x.0).is_some());
        tiles.sort_by_key(|x| rank(&x.0));
        lined_up.retain(|x| rank(&x.0).is_some());
        lined_up.sort_by_key(|x| rank(&x.0));
        let rects: Vec<_> = tiles.iter().map(|x| x.1).collect();
        let pills: Vec<_> = lined_up.iter().map(|x| if x.2 < count { self.pill_rect(&doc.panes[x.2], cx) } else { x.1 }).collect();
        // Every tile the same size unless `tile_focus_grows` lets the selected one take more.
        let active = self.person.tile_focus_grows.then(|| self.canvas_state.selected.iter().rev().find_map(|k| tiles.iter().position(|t| &t.0 == k))).flatten();
        // The canvas starts under the bar, so tiles fill its whole area.
        let (placed, lined) = canvas::tile([area[0], area[1], visible[0], visible[1]], tiles.len(), active, &pills, self.person.tile_gap);
        let state = &mut self.canvas_state;
        let unchanged = rects == placed && lined_up.iter().zip(&lined).all(|(a, b)| a.1[0] == b[0] && a.1[1] == b[1]);
        if !toggle && unchanged { return; }
        if started {
            let doc = &mut state.doc;
            let before = doc.panes.iter().map(|c| (c.id.clone(), c.rect, c.size()))
                .chain(doc.spaces.iter().map(|s| (s.space.clone(), s.rect, None)))
                .filter(|(k, ..)| order.contains(k)).collect();
            // Members draw above the cells that stay put.
            if !all {
                let top = doc.panes.iter().map(|c| c.z).max().unwrap_or(0).saturating_add(1);
                for c in &mut doc.panes { if let Some(r) = rank(&c.id) { c.z = top + r as u64; } }
            }
            state.tile = Some(Tile { camera, before, order, all });
        } else if let Some(t) = &mut state.tile { t.order = order; }
        if state.shown.is_none() { state.shown = Some(state.doc.clone()); }
        let doc = &mut state.doc;
        for ((_, _, i), r) in tiles.iter().zip(placed) {
            doc.panes[*i].rect = r;
            if let Some(size) = doc.panes[*i].size_mut() { *size = Some([r[2], r[3]]); }
        }
        for ((_, _, i), r) in lined_up.iter().zip(lined) {
            let rect = if *i < count { &mut doc.panes[*i].rect } else { &mut doc.spaces[*i - count].rect };
            rect[0] = r[0];
            rect[1] = r[1];
        }
        let to = canvas::Camera { x: area[0], y: area[1], zoom: 1. };
        if camera != to {
            state.motion.target = Some((doc.camera, to, std::time::Instant::now(), [visible[0] / 2., visible[1] / 2.]));
            self.animate_camera(window, cx);
        }
        self.animate_layout(window, cx);
        cx.notify();
    }

    pub(super) fn refresh_sub_hooks(&mut self) {
        self.canvas_state.subs = self.live.iter().filter(|(_, record)| record["state"] == "live")
            .map(|(addr, record)| (addr.clone(), crate::subs::hooks(record))).collect();
    }

    /// Watches transcript folders after label events, without an idle timer.
    pub(super) fn watch_subs(&mut self, cx: &mut Context<Self>) {
        let folders: std::collections::HashSet<String> = self.canvas_state.subs.values().flatten()
            .filter_map(|s| s.transcript.as_ref()?.parent()?.ancestors().find(|p| p.is_dir()))
            .map(|p| p.to_string_lossy().into_owned()).collect();
        self.canvas_state.sub_watches.retain(|folder, _| folders.contains(folder));
        for folder in folders {
            if self.canvas_state.sub_watches.contains_key(&folder) { continue; }
            let watch = folder.clone();
            let task = cx.spawn(async move |this, cx| {
                loop {
                    let folder = watch.clone();
                    let result = cx.background_executor().spawn(async move { crate::files::next_change(folder).await }).await;
                    let failed = result.is_err();
                    if this.update(cx, |this, cx| {
                        if let Err(error) = result { this.canvas_state.error = Some(error); }
                        else {
                            this.refresh_sub_hooks();
                            if this.canvas_state.doc.sync_subs(&this.canvas_state.subs) { this.canvas_changed(cx); }
                        }
                        cx.notify();
                    }).is_err() || failed { break; }
                }
            });
            self.canvas_state.sub_watches.insert(folder, task);
        }
    }

    /// Removes finished observations through the parent terminal's lifecycle labels.
    pub(super) fn clear_subs(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let Some(parent) = a["terminal"].as_str() else { return };
        let ids = crate::subs::finished_ids(self.canvas_state.subs.get(parent).map(Vec::as_slice).unwrap_or_default(), a["id"].as_str());
        if ids.is_empty() { return; }
        let parent = parent.to_owned();
        let labels: serde_json::Map<String, Value> = ids.iter().map(|id| (format!("canopy.sub.{id}"), json!(""))).collect();
        let core = self.core.clone();
        cx.spawn_in(window, async move |this, cx| {
            let params = json!({"terminal":parent,"labels":labels});
            let result = cx.background_executor().spawn(async move { core.call("terminal.label", params) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Err(error) = result { this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":format!("Could not clear finished: {error}")}}), window, cx); }
                cx.notify();
            });
        }).detach();
    }

    /// The panes an action applies to.
    fn targets(&self, window: &Window) -> Vec<String> {
        let doc = &self.canvas_state.doc;
        let here = |key: &str| doc.panes.iter().any(|c| !c.boxed() && c.id == key && doc.shows(c.space.as_ref()));
        let selected: Vec<_> = self.canvas_state.selected.iter().filter(|k| here(k)).cloned().collect();
        if !selected.is_empty() { return selected; }
        self.terms.values().find(|t| t.focus.is_focused(window) && here(&t.address))
            .map(|t| t.address.clone())
            .or_else(|| self.canvas_state.selected.last().filter(|k| here(k)).cloned())
            .into_iter().collect()
    }

    /// Removes file and mod panes, their selection and previews no longer in use.
    pub(super) fn close_boxed_panes(&mut self, ids: &[String], cx: &mut Context<Self>) {
        self.forget_annotations(ids, cx);
        let state = &mut self.canvas_state;
        let paths: Vec<_> = state.doc.panes.iter().filter(|p| ids.contains(&p.id)).filter_map(|p| p.file().map(str::to_owned)).collect();
        state.doc.panes.retain(|p| !ids.contains(&p.id));
        state.selected.retain(|id| !ids.contains(id));
        self.forget_sims(ids, cx);
        let state = &mut self.canvas_state;
        for path in paths {
            if state.file.as_ref() != Some(&path) && !state.doc.panes.iter().any(|p| p.file() == Some(path.as_str())) {
                state.previews.remove(&path);
            }
        }
        self.sync_browsers(cx);
        self.prune_images(ids);
        self.prune_pdf_views();
        self.canvas_changed(cx);
        cx.notify();
    }

    /// Closes the selected cell with confirmation appropriate to its current kind.
    pub(super) fn close_canvas_cell(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = a["pane"].as_str() {
            let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return self.fail(format!("no pane {id}")) };
            if !pane.boxed() { return self.fail(format!("pane {id} is a {}: close it with canvas.close {{terminal}}", pane.kind_name())); }
            // Its text lives only in canvas.json: an agent must not delete what the person wrote.
            if pane.text().is_some() { return self.fail(format!("pane {id} holds text the person wrote: only they can close it")); }
            return self.close_boxed_panes(&[id.to_owned()], cx);
        }
        // The confirmed dialog hands back the panes whose text closing deletes.
        if let Some(list) = a["panes"].as_array() {
            let ids: Vec<String> = list.iter().filter_map(Value::as_str).map(str::to_owned).collect();
            return self.close_boxed_panes(&ids, cx);
        }
        if a["terminal"].is_null() && self.wire_delete_selected(window, cx) { return; }
        // The confirmed dialog hands back the whole list; terminate it as is.
        if let Some(list) = a["terminals"].as_array() {
            for key in list.iter().filter_map(Value::as_str) { self.dispatch("terminal.kill", &json!({"terminal":key}), true, window, cx); }
            return;
        }
        let key = match a["terminal"].as_str() {
            Some(key) => key.to_owned(),
            None => {
                if !self.focus.is_focused(window) { return; }
                let state = &self.canvas_state;
                let (written, files): (Vec<_>, Vec<_>) = state.doc.panes.iter().filter(|p| state.selected.contains(&p.id) && state.doc.shows(p.space.as_ref()) && p.boxed()).partition(|p| p.text().is_some());
                let written: Vec<_> = written.into_iter().map(|p| p.id.clone()).collect();
                let files: Vec<_> = files.into_iter().map(|p| p.id.clone()).collect();
                if !files.is_empty() {
                    self.close_boxed_panes(&files, cx);
                }
                if !written.is_empty() { return self.confirm_close_written(written, window, cx); }
                if !files.is_empty() && self.targets(window).is_empty() { return; }
                // Every target pane closes behind one dialog; only spaces and sub-agents take the single path.
                let panes: Vec<_> = self.targets(window).into_iter()
                    .filter(|k| self.canvas_state.doc.panes.iter().any(|c| &c.id == k && c.terminal().is_some())).collect();
                if !panes.is_empty() { return self.close_panes(panes, window, cx); }
                let Some(key) = self.canvas_state.selected.last().cloned() else { return };
                key
            }
        };
        // A space card on Home: delete the space, with its usual confirmation.
        if self.canvas_state.doc.spaces.iter().any(|s| s.space == key) {
            return self.run_key("space.close", json!({"id":key}), window, cx);
        }
        let Some(card) = self.canvas_state.doc.panes.iter().find(|c| c.id == key) else { return };
        let sub = if let canvas::Kind::Sub { parent, sub } = &card.kind { self.canvas_state.subs.get(parent).into_iter().flatten().find(|s| &s.id == sub) } else { None };
        let agent = self.live.get(&key).is_some_and(canvas_agent);
        let choice = canvas::close_choice(sub.map(|s| s.done), agent, self.person.confirm_close_terminal);
        match choice {
            canvas::CloseChoice::RefuseSub => {
                self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"This sub-agent is still working. Stop it through its parent."}}), window, cx);
            }
            canvas::CloseChoice::ClearSub => {
                let canvas::Kind::Sub { parent, sub } = &card.kind else { return };
                let args = json!({"terminal":parent,"id":sub});
                self.clear_subs(&args, window, cx);
            }
            canvas::CloseChoice::ConfirmAgent | canvas::CloseChoice::ConfirmTerminal if a["confirmed"] != if agent { "agent" } else { "terminal" } => {
                let (title, detail) = if agent {
                    ("Terminate agent?", format!("{} stops and its cell closes.", self.pill_name(&key)))
                } else {
                    ("Close this terminal?", "This ends the terminal and its running programs.".into())
                };
                self.open_overlay(mods::new_overlay_id(), json!({"mod":"confirm","kind":"dialog","width":400,
                    "props":{"title":title,"detail":detail,"destructive":true,
                    "accept":if agent {"Terminate"} else {"Close"},"dontAsk":!agent,
                    "command":{"name":"canvas.close","args":{"terminal":key,"confirmed":if agent {"agent"} else {"terminal"}}}}}), window, cx);
            }
            _ => {
                if !agent && a["dont_ask"] == true {
                    if let Err(error) = settings::skip_terminal_confirmation(&settings::path()) { return self.fail(error); }
                    self.person.confirm_close_terminal = false;
                }
                self.dispatch("terminal.kill", &json!({"terminal":key}), true, window, cx);
            }
        }
    }

    /// `canvas.sync`: turns sync input on for the target panes (two or more), or off.
    pub(super) fn toggle_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.canvas_state.synced.borrow().is_empty() {
            self.canvas_state.synced.borrow_mut().clear();
        } else {
            let panes: Vec<_> = self.targets(window).into_iter().filter(|k| self.live.contains_key(k)).collect();
            if panes.len() < 2 { return self.fail("select two or more terminal panes to sync their input"); }
            *self.canvas_state.synced.borrow_mut() = panes;
        }
        cx.notify();
    }

    /// `canvas.equalize`: every target pane opens at the standard size; tile view re-tiles.
    pub(super) fn equalize_panes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let size = self.default_pane_size(window);
        let targets = self.targets(window);
        let mut changed = false;
        for card in self.canvas_state.doc.panes.iter_mut().filter(|c| c.terminal().is_some() && targets.contains(&c.id)) {
            changed |= card.size() != Some(size);
            *card.size_mut().unwrap() = Some(size);
        }
        if self.canvas_state.tile.is_some() { return self.tile_canvas(false, window, cx); }
        if changed {
            if let Some(first) = targets.first() { self.solve_layout(Some(first), None, cx); }
            self.canvas_changed(cx);
            cx.notify();
        }
    }

    /// `canvas.front` / `canvas.back`: moves the target panes above or below every other pane.
    /// A `terminal` outside the selection (a right-clicked pane) moves alone.
    pub(super) fn stack_panes(&mut self, front: bool, terminal: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let targets = match terminal {
            Some(t) if !self.canvas_state.selected.iter().any(|s| s == t) => vec![t.to_owned()],
            _ => self.targets(window),
        };
        if self.canvas_state.doc.stack(&targets, front) { self.canvas_changed(cx); cx.notify(); }
    }

    /// A click on a pane raises it; saves only when the order changed.
    fn raise_pane(&mut self, key: &str, cx: &mut Context<Self>) {
        if self.canvas_state.doc.stack(&[key.to_owned()], true) { self.canvas_changed(cx); }
    }

    /// Terminates every pane behind one dialog listing them; without agents and with the
    /// terminal prompt switched off it just closes them.
    /// Asks before closing mod panes whose text exists only in canvas.json (notes).
    fn confirm_close_written(&mut self, panes: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let rows: Vec<_> = panes.iter().filter_map(|id| self.canvas_state.doc.panes.iter().find(|p| &p.id == id)).map(|p| {
            let hint = p.mod_name().and_then(|n| self.folders.get(n)).and_then(|f| f.hint.pane.as_ref());
            json!({"name":hint.map_or(p.kind_name(), |h| h.label.as_str()),"icon":hint.map_or("icons/x.svg", |h| h.icon.as_str()),"tint":true})
        }).collect();
        let title = if panes.len() == 1 { "Close this pane?".to_owned() } else { format!("Close {} panes?", panes.len()) };
        self.open_overlay(mods::new_overlay_id(), json!({"mod":"confirm","kind":"dialog","width":400,
            "props":{"title":title,"detail":"Their text is deleted. This cannot be undone.","panes":rows,"destructive":true,
            "accept":"Close and delete text",
            "command":{"name":"canvas.close","args":{"panes":panes}}}}), window, cx);
    }

    fn close_panes(&mut self, panes: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let agents = panes.iter().any(|k| self.live.get(k).is_some_and(canvas_agent));
        if !agents && !self.person.confirm_close_terminal {
            for key in &panes { self.dispatch("terminal.kill", &json!({"terminal":key}), true, window, cx); }
            return;
        }
        let rows: Vec<_> = panes.iter().map(|k| {
            let (icon, tint) = program_icon(self.live.get(k).unwrap_or(&Value::Null));
            json!({"name":self.pill_name(k),"icon":icon,"tint":tint})
        }).collect();
        let title = if panes.len() == 1 { "Terminate this pane?".to_owned() } else { format!("Terminate {} panes?", panes.len()) };
        self.open_overlay(mods::new_overlay_id(), json!({"mod":"confirm","kind":"dialog","width":400,
            "props":{"title":title,"detail":"Their programs stop and the panes close.","panes":rows,"destructive":true,
            "accept":"Terminate and close all",
            "command":{"name":"canvas.close","args":{"terminals":panes}}}}), window, cx);
    }

    pub(super) fn check_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let editors = crate::programs::editors();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_executor().spawn(async move { crate::programs::installed_editors(editors) }).await;
            let _ = this.update_in(cx, |this, _, cx| {
                match result {
                    Ok(editors) => this.canvas_state.editors = editors,
                    Err(e) => this.canvas_state.error = Some(e),
                }
                cx.notify();
            });
        }).detach();
    }

    fn panel_editors(&self) -> Vec<crate::programs::Editor> {
        if self.canvas_state.doc.panel.space.as_deref().and_then(|id| self.model.space(id)).is_some_and(|s| s.machine.is_some()) {
            vec![]
        } else { self.canvas_state.editors.clone() }
    }

    pub(super) fn panel_editor(&mut self, args: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.canvas_state.file.clone() else { return };
        if args["path"].as_str().is_some_and(|p| p != path) { return; }
        if let Some(dirty) = args["dirty"].as_bool() {
            if self.canvas_state.editing.as_ref() == Some(&path) {
                self.canvas_state.dirty = dirty.then_some(path);
                cx.notify();
            }
            return;
        }
        if args["done"].as_bool() == Some(true) {
            self.canvas_state.dirty = None;
            self.canvas_state.editing = None;
        } else {
            let choice = args["choice"].as_str().unwrap_or("here");
            if choice == "here" {
                self.canvas_state.editing = Some(path.clone());
                self.canvas_state.dirty = None;
            } else {
                let Some(editor) = self.panel_editors().into_iter().find(|e| e.key == choice) else { return };
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx.background_executor().spawn(async move {
                        crate::spawn::command(&editor.argv[0]).args(&editor.argv[1..]).arg(&path)
                            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
                    }).await;
                    match result {
                        Ok(mut child) => { std::thread::spawn(move || { let _ = child.wait(); }); }
                        Err(e) => { let _ = this.update_in(cx, |this, window, cx| this.layout_notice("Editor", &format!("Could not open editor: {e}"), window, cx)); }
                    }
                }).detach();
            }
            self.canvas_state.doc.panel.editor = Some(choice.to_owned());
            self.canvas_changed(cx);
        }
        self.sync(window, cx);
        cx.notify();
    }

    /// The folder of the space last clicked on the canvas.
    fn panel_root(&self) -> Option<String> {
        let space = self.canvas_state.doc.panel.space.as_deref()?;
        self.model.space(space).map(|s| s.folder.clone())
    }
    /// The space a window position belongs to: the room being shown, else the Home card under it.
    fn space_at(&self, position: Point<Pixels>) -> Option<String> {
        if let Some(room) = &self.canvas_state.doc.room { return Some(room.clone()); }
        let origin = self.bounds.borrow().get("canopy-canvas").map(|b| b.origin).unwrap_or_default();
        let p = self.canvas_state.doc.camera.to_world([
            f32::from(position.x - origin.x),
            f32::from(position.y - origin.y),
        ]);
        canvas::space_at(&self.canvas_state.doc.spaces, p).map(|s| s.space.clone())
    }
    /// The window root forwards modifier changes, including while a terminal has focus.
    pub(in crate::window) fn canvas_modifiers(&mut self, m: gpui::Modifiers, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.has_canvas() && window.is_window_active() && !m.alt && !m.shift && !m.function
            && if cfg!(target_os = "macos") { m.platform && !m.control } else { m.control && !m.platform };
        if !held {
            self.hide_key_hints(window, cx);
        } else if self.canvas_state.hint_timer.is_none() && self.canvas_state.actions.is_none() {
            self.canvas_state.hint_timer = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(400)).await;
                this.update_in(cx, |this, window, cx| this.show_key_hints(window, cx))
            }));
        }
    }

    fn show_key_hints(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.actions = Some(crate::keys::listing());
        if mods::publish_window(self.window_json(window)) { self.refresh_snapshot_mods(cx); }
        self.sync(window, cx);
        cx.notify();
    }

    pub(super) fn hide_key_hints(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.hint_timer.is_none() && self.canvas_state.actions.is_none() { return; }
        self.canvas_state.hint_timer = None;
        self.canvas_state.actions = None;
        if mods::publish_window(self.window_json(window)) { self.refresh_snapshot_mods(cx); }
        self.sync(window, cx);
        cx.notify();
    }
    /// Shows a space's room (None: Home) with the camera it had there; a first visit fits what is inside.
    pub(super) fn go_room(&mut self, room: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        // Any space tab or Home puts an expanded browser back, even the place already shown.
        self.collapse_browser(window, cx);
        if room == self.canvas_state.doc.room { return; }
        let first = !self.canvas_state.doc.cameras.contains_key(&room.clone().unwrap_or_default());
        self.end_canvas_drag(cx);
        // Leaving a tiled place puts its geometry back at once (no ease into the next place).
        self.canvas_state.untile();
        self.canvas_state.motion = Motion::default();
        self.canvas_state.wheel_zoom = None;
        self.canvas_state.wheel_pan_at = None;
        self.canvas_state.doc.go(room.clone());
        self.canvas_state.selected.clear();
        self.canvas_state.wire.selected = None;
        (self.canvas_state.wire.drag, self.canvas_state.wire.sticky) = (None, false);
        self.canvas_state.connecting = None;
        self.canvas_state.previous_camera = None;
        if let Some(id) = &room {
            self.canvas_state.recent.retain(|r| r != id);
            self.canvas_state.recent.insert(0, id.clone());
            self.canvas_state.doc.panel.space = Some(id.clone());
            self.model.active = Some(id.clone());
        }
        if first { self.canvas_command("canvas.fit", window, cx); }
        self.canvas_changed(cx);
        self.focus.focus(window, cx);
        self.sync(window, cx);
        cx.notify();
    }
    /// Points the panel's file tree at a space's folder.
    fn show_space_files(&mut self, space: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.doc.panel.space.as_deref() != Some(space.as_str()) {
            self.canvas_state.doc.panel.space = Some(space);
            self.canvas_changed(cx);
            self.sync(window, cx);
            cx.notify();
        }
    }
    /// Whether the canvas-keys strip shows one connect line instead of its key list: a wire
    /// follows the pointer, or a keyboard connection is pending.
    fn connect_hint(&self) -> bool {
        self.canvas_state.wire.sticky || self.canvas_state.connecting.is_some()
    }
    pub(super) fn panel_mods(&self) -> Vec<(String, Instance)> {
        if !self.has_canvas() { return vec![]; }
        let mut slots = vec![];
        if self.canvas_state.connecting.is_some() {
            slots.push(("canvas-keys".into(), Instance { name: "canvas-keys".into(), props: json!({"connecting":self.canvas_state.connecting.is_some()}) }));
        }
        // A wire following the pointer (helper bar Connect, a clicked "+") shows only its one-line
        // hint, never the key list.
        if self.canvas_state.wire.sticky {
            slots.retain(|(slot, _)| slot != "canvas-keys");
            slots.push(("canvas-keys".into(), Instance { name: "canvas-keys".into(), props: json!({"pointer":true}) }));
        }
        for pane in &self.canvas_state.doc.panes {
            if !self.canvas_state.doc.shows(pane.space.as_ref()) { continue; }
            if let Some(name) = pane.mod_name().filter(|name| self.folders.get(*name).is_some_and(|f| f.hint.pane.is_some())) {
                slots.push((format!("pane:{}", pane.id), Instance { name: name.into(), props: json!({"pane":pane.id}) }));
            }
        }
        if self.canvas_state.file.is_none() && !self.canvas_state.files { return slots; }
        let root = self.panel_root();
        if !self.canvas_state.doc.panel.tree_hidden {
            slots.push((
                "canvas-tree".into(),
                Instance {
                    name: "files".into(),
                    props: json!({"canvas":"tree","root":root}),
                },
            ));
        }
        if let Some(path) = &self.canvas_state.file {
            if self.canvas_state.editing.as_ref() == Some(path) || !rendered(path)
            {
                slots.push(("canvas-file".into(), Instance { name: "files".into(), props: json!({"canvas":"viewer","editing":self.canvas_state.editing.as_ref() == Some(path),"root":std::path::Path::new(path).parent(),"file":std::path::Path::new(path).file_name().map(|p|p.to_string_lossy())}) }));
            }
        }
        slots
    }
    pub(super) fn open_preview(
        &mut self,
        args: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let point = match args.get("point") {
            Some(p) => match serde_json::from_value::<canvas::Point>(p.clone()) {
                Ok(p) if p.iter().all(|v| v.is_finite()) => p,
                _ => return self.fail("invalid canvas point"),
            },
            None => self.view_center(window),
        };
        let Some(path) = args["path"].as_str() else {
            let paths = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Select file".into()),
            });
            cx.spawn_in(window, async move |this, cx| {
                if let Ok(Ok(Some(paths))) = paths.await {
                    if let Some(path) = paths.first() {
                        let _ = this.update_in(cx, |this, window, cx| {
                            this.open_preview(json!({"path":path,"point":point}), window, cx)
                        });
                    }
                }
            })
            .detach();
            return;
        };
        if args["place"] != "canvas" && args.get("point").is_none() {
            let path = std::path::Path::new(path);
            if !path.is_absolute() {
                return self.fail("preview path must be absolute");
            }
            let path = path
                .canonicalize()
                .unwrap_or_else(|_| path.to_owned())
                .to_string_lossy()
                .into_owned();
            canvas::open_panel_tab(&mut self.canvas_state.tabs, &path);
            if self.canvas_state.file.as_ref() != Some(&path) { self.canvas_state.dirty = None; }
            self.canvas_state.file = Some(path);
            self.focus.focus(window, cx);
            self.sync(window, cx);
            cx.notify();
            return;
        }
        let room = self.canvas_state.doc.room.clone();
        if let Err(e) = self.add_file_pane(std::path::Path::new(path), point, room, window, cx) {
            self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":e}}), window, cx);
        }
    }

    /// The rects of the panes (and, on Home, space cards) in the current place that `pick` keeps.
    /// Pinned panes float over the camera, so zoom limits and fits leave them out.
    fn place_rects(&self, pick: impl Fn(&String) -> bool, cx: &App) -> Vec<canvas::Rect> {
        let doc = &self.canvas_state.doc;
        doc.panes.iter().filter(|c| doc.shows(c.space.as_ref()) && c.pinned.is_none() && pick(&c.id)).map(|c| self.pill_rect(c, cx))
            .chain(doc.spaces.iter().filter(|s| doc.room.is_none() && pick(&s.space)).map(|s| s.rect))
            .collect()
    }

    /// How far the camera may zoom in this place (see `canvas::zoom_range`).
    fn zoom_range(&self, window: &Window, cx: &App) -> [f32; 2] {
        let size = self.bounds.borrow().get("canopy-canvas").map(|b| b.size).unwrap_or(window.viewport_size());
        canvas::zoom_range(&self.place_rects(|_| true, cx), [f32::from(size.width), f32::from(size.height)])
    }

    /// `zoom_range` widened to hold the current zoom, for gestures and zoom steps: a zoom left
    /// past a limit (a pane grew, or a sim went to actual size) moves back freely instead of
    /// being yanked to the limit on the next event; `settle_zoom` then eases it inside.
    fn gesture_range(&self, window: &Window, cx: &App) -> [f32; 2] {
        canvas::widen(self.zoom_range(window, cx), self.canvas_state.doc.camera.zoom)
    }

    /// Puts pinned panes back where they sit on screen (`canvas::Document::pin`). Runs before
    /// each render, with the canvas size from the last frame.
    pub(in crate::window) fn pin_panes(&mut self) {
        let Some(size) = self.bounds.borrow().get("canopy-canvas").map(|b| [f32::from(b.size.width), f32::from(b.size.height)]) else { return };
        let state = &mut self.canvas_state;
        let camera = state.doc.camera;
        state.doc.pin(camera, size);
        if let Some(shown) = &mut state.shown { shown.pin(camera, size); }
    }

    /// The viewport centre in world pixels.
    pub(super) fn view_center(&self, window: &Window) -> canvas::Point {
        let size = self.bounds.borrow().get("canopy-canvas").map(|b| b.size).unwrap_or(window.viewport_size());
        self.canvas_state.doc.camera.to_world([f32::from(size.width) / 2., f32::from(size.height) / 2.])
    }

    /// Opens or moves a file-backed pane, returning its id, canonical path and kind.
    pub(super) fn add_file_pane(&mut self, path: &std::path::Path, point: canvas::Point, space: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Result<(String, String, String), String> {
        let kind = canvas::file_kind(path);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match kind {
            "file" if canvas::read_preview(path).is_err() => return Err(format!("Can't preview {name}")),
            "pdf" if !pdf::has_pdf_header(path) => return Err(format!("Can't open {name}: not a PDF")),
            _ => {}
        }
        if kind == "image" && !path.is_file() {
            return Err(format!("Can't open {name}: not a readable image"));
        }
        let canonical = path.canonicalize().map_err(|e| match kind {
            "image" => format!("Can't open {name}: not a readable image"),
            _ => format!("Can't open {}: {e}", path.display()),
        })?.to_string_lossy().into_owned();
        let (kind, size) = match kind {
            "image" => {
                let size = self.open_image(path, &canonical).map_err(|_| format!("Can't open {name}: not a readable image"))?;
                (canvas::Kind::Image { path: canonical.clone() }, size)
            }
            "pdf" => (canvas::Kind::Pdf { path: canonical.clone(), highlights: vec![] }, [560., 720.]),
            _ => (canvas::Kind::File { path: canonical.clone() }, [560., 420.]),
        };
        let path = canonical;
        let doc = &mut self.canvas_state.doc;
        let (id, kind_name) = if let Some(pane) = doc.panes.iter_mut().find(|p| p.path() == Some(path.as_str())) {
            pane.rect[0] = point[0];
            pane.rect[1] = point[1];
            let id = pane.id.clone();
            // A pane that changes space leaves its wires behind: wires join one space only.
            if pane.space != space {
                pane.space = space;
                doc.wires.retain(|w| w.a != id && w.b != id);
            }
            (id, pane.kind_name().to_owned())
        } else {
            let id = doc.next_file_id();
            let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
            let pane = canvas::Pane { id: id.clone(), kind, rect: [point[0], point[1], size[0], size[1]], z, space, color: None, pinned: None };
            let kind_name = pane.kind_name().to_owned();
            doc.panes.push(pane);
            (id, kind_name)
        };
        self.load_previews(window, cx);
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        cx.notify();
        Ok((id, path, kind_name))
    }
    /// Opens a mod pane at its own saved rect; its mod copy receives the pane id.
    fn add_mod_pane(&mut self, kind: String, point: canvas::Point, space: Option<String>, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Mod { kind, props: serde_json::Map::new() }, rect: [point[0], point[1], 320., 240.], z, space, color: None, pinned: None });
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        cx.notify();
        id
    }

    fn load_previews(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_browsers(cx);
        self.sync_pdf_views(window, cx);
        self.sync_image_sizes(window, cx);
        let mut paths: Vec<_> = self.canvas_state.doc.panes.iter().filter_map(|p| p.file().map(str::to_owned)).collect();
        if let Some(path) = &self.canvas_state.file {
            if rendered(path) { paths.push(path.clone()); }
        }
        self.canvas_state.previews.retain(|path, _| paths.contains(path));
        for path in paths {
            if self.canvas_state.previews.contains_key(&path) { continue; }
            let text = preview_source(&path);
            let view = cx.new(|cx| gpui_base::TextViewState::markdown(&text, cx));
            let cache_path = path.clone();
            let folder = std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.ancestors().find(|p| p.is_dir()))
                .unwrap_or(std::path::Path::new("/"))
                .to_string_lossy()
                .into_owned();
            let task = cx.spawn(async move |this, cx| {
                loop {
                    let watch = folder.clone();
                    let result = cx
                        .background_executor()
                        .spawn(async move { crate::files::next_change(watch).await })
                        .await;
                    let Ok(changed) = result else { break };
                    if !changed
                        .iter()
                        .any(|p| p == &path || p == &path.replace(".jsonl", ".meta.json") || std::path::Path::new(&path).starts_with(p))
                    {
                        continue;
                    }
                    let read = path.clone();
                    let text = cx
                        .background_executor()
                        .spawn(async move { preview_source(&read) })
                        .await;
                    if this
                        .update(cx, |this, cx| {
                            if let Some((view, _)) = this.canvas_state.previews.get(&path) {
                                view.update(cx, |view, cx| view.set_text(&text, cx));
                            }
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
            self.canvas_state.previews.insert(cache_path, (view, task));
        }
    }

    pub(super) fn close_pop_outs(&mut self, cx: &mut Context<Self>) {
        for (_, handle) in self.canvas_state.popped.drain() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub(super) fn pop_canvas(
        &mut self,
        addr: String,
        bring_back: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(handle) = self.canvas_state.popped.get(&addr).copied() {
            if bring_back {
                self.canvas_state.popped.remove(&addr);
                let _ = handle.update(cx, |_, window, _| window.remove_window());
                self.set_expanded(&addr, true, window, cx);
            } else {
                let _ = handle.update(cx, |_, window, _| window.activate_window());
            }
            return;
        }
        if bring_back || !self.live.contains_key(&addr) {
            return;
        }
        let core = self.core.clone();
        let title = self.live[&addr]["title"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&addr)
            .to_owned();
        let owner = window.window_handle().downcast::<AppWindow>().unwrap();
        let closed_addr = addr.clone();
        let terminal_addr = addr.clone();
        let result = cx.open_window(WindowOptions::default(), move |window, cx| {
            window.set_window_title(&title);
            window.on_window_should_close(cx, move |_, cx| {
                let _ = owner.update(cx, |this, window, cx| {
                    this.canvas_state.popped.remove(&closed_addr);
                    this.set_expanded(&closed_addr, true, window, cx);
                    this.sync(window, cx);
                    cx.notify();
                });
                true
            });
            cx.new(|cx| {
                let terminal = cx.new(|cx| {
                    let mut terminal = TerminalView::attach(core, terminal_addr, (120, 40), cx);
                    terminal.standalone = true;
                    terminal
                });
                terminal.read(cx).focus_handle().clone().focus(window, cx);
                cx.on_release(|this: &mut PopOut, cx| this.terminal.read(cx).detach())
                    .detach();
                PopOut {
                    terminal,
                    _sub: None,
                }
            })
        });
        match result {
            Ok(handle) => {
                let terminal = handle.read(cx).unwrap().terminal.clone();
                let block = self
                    .model
                    .pane_of(&addr)
                    .unwrap_or_else(|| "canvas-panel".into());
                let sub = self.setup_term(&terminal, &block, &addr, window, cx);
                let _ = handle.update(cx, |pop, _, _| pop._sub = Some(sub));
                self.canvas_state.popped.insert(addr.clone(), handle);
                self.set_expanded(&addr, false, window, cx);
                self.sync(window, cx);
            }
            Err(error) => self.fail(error.to_string()),
        }
    }

    /// The standard pane size (world px, header and frame included): the `terminal_size` grid.
    /// New panes open at it and `canvas.equalize` returns panes to it.
    pub(super) fn default_pane_size(&self, window: &Window) -> canvas::Point {
        let body = crate::terminal::size_of_grid(self.person.terminal_size[0], self.person.terminal_size[1], window);
        // The 2 px is the frame around the body.
        [f32::from(body.width) + 2., f32::from(body.height) + canvas::HEADER + 2.]
    }

    /// Grows a terminal cell into a live terminal in place (neighbours give way), or shrinks it back to a pill.
    pub(super) fn set_expanded(&mut self, addr: &str, expand: bool, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        let Some(i) = state.doc.panes.iter().position(|c| c.terminal().is_some() && c.id == addr) else { return };
        if state.doc.panes[i].size().is_some() == expand || (expand && (!self.live.contains_key(addr) || state.popped.contains_key(addr))) { return; }
        self.end_canvas_drag(cx);
        let state = &mut self.canvas_state;
        if state.shown.is_none() { state.shown = Some(state.doc.clone()); }
        if expand {
            let before = canvas::Rest::capture(&state.doc);
            let size = self.default_pane_size(window);
            let state = &mut self.canvas_state;
            let z = state.doc.panes.iter().map(|c| c.z).max().unwrap_or(0).saturating_add(1);
            *state.doc.panes[i].size_mut().unwrap() = Some(size);
            state.doc.panes[i].z = z;
            state.selected = vec![addr.to_owned()];
            // Tile view re-tiles below; neighbours do not give way.
            if self.canvas_state.tile.is_none() {
                self.solve_layout(Some(addr), None, cx);
                let state = &mut self.canvas_state;
                let moved = before.changed(&canvas::Rest::capture(&state.doc));
                state.moves.insert(addr.to_owned(), (before, moved));
            }
        } else {
            let tiled = state.tile.is_some();
            if let Some((base, moved)) = state.moves.remove(addr) { if !tiled { base.put_back(&moved, &mut state.doc); } }
            *state.doc.panes[i].size_mut().unwrap() = None;
            if !tiled { self.solve_layout(Some(addr), None, cx); }
        }
        self.animate_layout(window, cx);
        self.canvas_changed(cx);
        // In tile view an expanded cell joins the tiles and a collapsed one the pills.
        self.tile_canvas(false, window, cx);
        if expand && let Some(p) = self.model.pane_of(addr) { self.model.focus_pane(&p); }
        self.sync(window, cx);
        match self.terms.get(&cell_slot(addr)).filter(|_| expand && !self.canvas_state.mode) {
            Some(t) => t.focus.focus(window, cx),
            None => self.focus.focus(window, cx),
        }
        cx.notify();
    }

    /// `canvas.expand {terminal?}`: toggles the named cell, else the one selected last.
    pub(super) fn expand_canvas_cell(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let Some(addr) = a["terminal"].as_str().map(str::to_owned).or_else(|| self.canvas_state.selected.last().cloned()) else {
            return self.fail("select a terminal cell first");
        };
        let Some(card) = self.canvas_state.doc.panes.iter().find(|c| c.terminal().is_some() && c.id == addr) else {
            return self.fail("no such terminal cell");
        };
        if self.canvas_state.popped.contains_key(&addr) {
            return self.pop_canvas(addr, true, window, cx);
        }
        let expand = card.size().is_none();
        self.set_expanded(&addr, expand, window, cx);
    }

    /// The camera zoom moved by a gesture: expanded cells draw flat until it has rested for a moment.
    fn zoom_moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        if !state.doc.panes.iter().any(|c| c.size().is_some()) { return; }
        state.zoomed_at = Some(std::time::Instant::now());
        state.zoom_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(ZOOM_REST).await;
            let _ = this.update(cx, |this, cx| {
                this.canvas_state.zoomed_at = None;
                cx.notify();
            });
        }));
    }

    /// Whether the camera has rested, so expanded cells may draw live text at its zoom.
    fn camera_rested(&self) -> bool {
        let state = &self.canvas_state;
        state.motion.target.is_none() && state.zoomed_at.is_none()
    }

    /// One wheel or trackpad event on the canvas: Cmd (Ctrl) zooms at the pointer, else it pans.
    fn canvas_wheel(&mut self, e: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let zoom = wheel_zooms(e.modifiers);
        let delta = e.delta.pixel_delta(px(20.));
        if zoom && delta.y != px(0.) {
            let anchor = self.wire_local(e.position);
            self.zoom_gesture(f32::from(delta.y) * 0.005, anchor, window, cx);
            self.canvas_state.wheel_zoom = Some(WheelZoom { anchor, timeout: None });
        } else if !zoom && delta != point(px(0.), px(0.)) {
            self.canvas_state.motion.target = None;
            self.canvas_state.motion.velocity = [0.; 2];
            self.canvas_state.doc.camera.pan([delta.x.into(), delta.y.into()]);
            self.canvas_state.wheel_pan_at = Some(std::time::Instant::now());
            self.canvas_changed(cx);
            cx.notify();
        }
        if gesture_ended(e.touch_phase) {
            self.canvas_state.wheel_pan_at = None;
            self.settle_wheel_zoom(window, cx);
        } else if let Some(gesture) = &mut self.canvas_state.wheel_zoom
            && (!cfg!(target_os = "macos") || !e.delta.precise())
        {
            // X11 and discrete wheels have no gesture-end event.
            gesture.timeout = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(WHEEL_GESTURE).await;
                let _ = this.update_in(cx, |this, window, cx| this.settle_wheel_zoom(window, cx));
            }));
        }
        cx.stop_propagation();
    }

    fn settle_wheel_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(gesture) = self.canvas_state.wheel_zoom.take() {
            self.settle_zoom(gesture.anchor, window, cx);
        }
    }

    /// Pinch uses the same pointer-anchored zoom as a modified wheel on every platform.
    fn canvas_pinch(&mut self, e: &PinchEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.wheel_zoom = None;
        let anchor = self.wire_local(e.position);
        if e.delta != 0. {
            self.zoom_gesture(e.delta, anchor, window, cx);
        }
        if gesture_ended(e.phase) {
            self.settle_zoom(anchor, window, cx);
        }
        cx.stop_propagation();
    }

    fn zoom_gesture(&mut self, delta: f32, anchor: canvas::Point, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.motion.target = None;
        self.canvas_state.motion.velocity = [0.; 2];
        let z = self.canvas_state.doc.camera.zoom * canvas::zoom_factor(delta);
        let z = canvas::rubber_band(z, self.gesture_range(window, cx));
        self.canvas_state.doc.camera.zoom_at(z, anchor);
        self.zoom_moved(window, cx);
        self.canvas_changed(cx);
        cx.notify();
    }

    /// The view of the expanded cell last selected, for the keyboard to return to.
    pub(super) fn focused_cell(&self) -> Option<&Term> {
        let addr = self.canvas_state.selected.last()?;
        self.terms.get(&cell_slot(addr))
    }

    /// Swaps a name for an input; Enter or blur dispatches `space.rename` / `tab.rename`.
    pub(super) fn rename_canvas(&mut self, target: Rename, window: &mut Window, cx: &mut Context<Self>) {
        let value = match &target {
            Rename::Space(id) => self.model.space(id).map(|s| s.name.clone()),
            Rename::Tab(id) => self.model.tab(id).map(|t| t.name.clone()),
        };
        let Some(value) = value else { return };
        let input = cx.new(|cx| {
            let mut input = gpui_base::input::InputState::new(window, cx);
            input.set_value(value, window, cx);
            input.focus(window, cx);
            input.select_all(window, cx);
            input
        });
        let sub = cx.subscribe_in(&input, window, |this, input, event, window, cx| {
            if matches!(event, gpui_base::input::InputEvent::PressEnter { .. } | gpui_base::input::InputEvent::Blur) {
                if let Some((target, _, _)) = this.canvas_state.rename.take() {
                    let name = input.read(cx).value().to_string();
                    let (command, id) = match target { Rename::Space(id) => ("space.rename", id), Rename::Tab(id) => ("tab.rename", id) };
                    this.dispatch(command, &json!({"id":id,"name":name}), true, window, cx);
                    cx.notify();
                }
            }
        });
        self.canvas_state.rename = Some((target, input, sub));
        cx.notify();
    }
    /// Glides the camera to center a pane at the same zoom when it is not wholly on screen.
    pub(super) fn reveal_pane(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.bounds.borrow().get("canopy-canvas").map(|b| [f32::from(b.size.width), f32::from(b.size.height)]);
        let rect = self.canvas_state.doc.panes.iter().find(|c| c.id == id).map(|c| c.rect);
        let from = self.canvas_state.doc.camera;
        if let (Some(visible), Some(rect)) = (visible, rect) && let Some(to) = canvas::reveal(from, rect, visible) {
            self.canvas_state.motion.target = Some((from, to, std::time::Instant::now(), [visible[0] / 2., visible[1] / 2.]));
            self.animate_camera(window, cx);
        }
    }

    fn animate_camera(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.motion.target.is_some() {
            self.canvas_state.wheel_zoom = None;
        }
        if self.canvas_state.motion.frame { return; }
        self.canvas_state.motion.frame = true;
        if self.canvas_state.motion.sample.is_some_and(|t| t.elapsed() > Duration::from_millis(80)) && self.canvas_state.motion.target.is_none() {
            self.canvas_state.motion.velocity = [0.; 2];
        }
        let entity = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = entity.update(cx, |this, cx| {
                let state = &mut this.canvas_state;
                state.motion.frame = false;
                let now = std::time::Instant::now();
                let mut active = false;
                if let Some((from, to, start, anchor)) = state.motion.target {
                    let t = start.elapsed().as_secs_f32() / 0.18;
                    let a = canvas::ease_out(t);
                    let zoom = from.zoom + (to.zoom-from.zoom)*a;
                    let f = from.to_world(anchor);
                    let world = to.to_world(anchor);
                    state.doc.camera = canvas::Camera { x: f[0]+(world[0]-f[0])*a-anchor[0]/zoom, y: f[1]+(world[1]-f[1])*a-anchor[1]/zoom, zoom };
                    active = t < 1.;
                    if !active { state.doc.camera = to; state.motion.target = None; }
                } else if state.drag.is_none() {
                    let dt = state.motion.sample.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.016).min(0.05);
                    state.motion.sample = Some(now);
                    let v = &mut state.motion.velocity;
                    state.doc.camera.pan([v[0]*dt, v[1]*dt]);
                    let decay = canvas::friction(dt);
                    v[0] *= decay; v[1] *= decay;
                    active = v[0].abs().max(v[1].abs()) > 5.;
                    if !active { *v = [0.; 2]; }
                }
                if active { this.animate_camera(window, cx); }
                else { this.canvas_changed(cx); }
                cx.notify();
            });
        });
        // Not window.request_animation_frame(): that is paint-only and this runs from events.
        cx.notify();
    }
    fn settle_zoom(&mut self, anchor: canvas::Point, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.zoom_range(window, cx);
        let from = self.canvas_state.doc.camera;
        let zoom = from.zoom.clamp(range[0], range[1]);
        if zoom == from.zoom { return; }
        let mut to = from;
        to.zoom_at(zoom, anchor);
        self.canvas_state.motion.target = Some((from, to, std::time::Instant::now(), anchor));
        self.animate_camera(window, cx);
    }
    pub(super) fn create_canvas(
        &mut self,
        args: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // No point (the menu bar): the world point at the middle of the view.
        let point = match args.get("point") {
            None => {
                let size = self.bounds.borrow().get("canopy-canvas").map(|b| b.size);
                let [w, h] = size
                    .map(|s| [f32::from(s.width), f32::from(s.height)])
                    .unwrap_or([1200., 800.]);
                Some(self.canvas_state.doc.camera.to_world([w / 2., h / 2.]))
            }
            Some(v) => serde_json::from_value::<canvas::Point>(v.clone())
                .ok()
                .filter(|p| p.iter().all(|v| v.is_finite())),
        };
        let Some(point) = point else {
            return self.fail("invalid canvas point");
        };
        if args["kind"] == "space" {
            if args.get("folder").is_none() {
                let mut folders: Vec<String> = self.model.spaces.iter().rev()
                    .filter(|s| s.machine.is_none() && !s.folder.is_empty()).map(|s| s.folder.clone()).collect();
                folders.extend(self.canvas_state.doc.recent_folders.clone());
                let mut unique = std::collections::HashSet::new();
                folders.retain(|f| unique.insert(f.clone()));
                let mut args = args.clone(); args["point"] = json!(point);
                self.open_overlay(mods::new_overlay_id(), json!({"mod":"space-new", "kind":"dialog", "width":440, "props":{"args":args,"folders":folders}}), window, cx);
                return;
            }
            let folder = args["folder"].as_str().unwrap_or_default().trim().to_owned();
            let name = args["name"].as_str().unwrap_or_default().trim();
            let name = if name.is_empty() { std::path::Path::new(&folder).file_name().and_then(|s| s.to_str()).unwrap_or_default() } else { name }.to_owned();
            if name.is_empty() { return self.fail("space needs a name or folder"); }
            if let Err(error) = self.require_document(None) { return self.fail(error); }
            let id = self.model.fresh_id_on("sp", None);
            self.model.spaces.push(model::Space { id: id.clone(), name, folder: folder.clone(), machine: None, tabs: vec![], active: None, rest: Default::default() });
            canvas::remember_folder(&mut self.canvas_state.doc.recent_folders, &folder);
            let color = canvas::unused_color(&self.canvas_state.doc.spaces, self.canvas_state.doc.spaces.len());
            // A new space is a card on Home; you go straight inside it.
            self.canvas_state.doc.spaces.push(canvas::Space { space: id.clone(), rect: [point[0], point[1], canvas::SPACE_CARD[0], canvas::SPACE_CARD[1]], color, frame: true });
            self.canvas_state.untile();
            self.canvas_state.doc.go(Some(id));
            self.canvas_changed(cx);
            self.flush_documents(false, window, cx);
            cx.notify();
            return;
        }
        if args.get("folder").is_some() { return self.fail("folder is only for kind space"); }
        // No space: the one under the point, so scripted creates land where they are placed.
        let space = Self::owned(&args, "space").or_else(|| self.canvas_state.doc.room.clone())
            .filter(|id| self.model.space(id).is_some());
        let folder = space
            .as_deref()
            .and_then(|id| self.model.space(id))
            .map(|s| s.folder.clone())
            .unwrap_or_else(home);
        let machine = space
            .as_deref()
            .and_then(|id| self.model.space(id))
            .and_then(|s| s.machine.as_deref());
        if let Err(error) = self.require_document(machine) {
            return self.fail(error);
        }
        let argv = Self::argv(&args);
        let space = space.or_else(|| {
            self.model
                .spaces
                .iter()
                .find(|s| root_space(s))
                .map(|s| s.id.clone())
        });
        let result = if let Some(space) = space {
            self.new_tab(Some(space), "", Some(folder), argv, None, window)
        } else {
            self.new_space("Canvas", Some(home()), None, argv, None, None, window)
        };
        match result {
            Ok(value) => {
                if let Some(addr) = value["terminal"].as_str() {
                    self.canvas_state
                        .pending_points
                        .insert(addr.to_owned(), point);
                    if let Some(quote) = args.get("quote") {
                        let mut quote = quote.clone();
                        quote.as_object_mut().unwrap().remove("argv");
                        quote["terminal"] = json!(addr);
                        self.quote_command("quote.send", &quote, window, cx);
                    }
                    // Terminals first: a new cell opens as a full terminal pane (collapse it by hand).
                    let size = self.default_pane_size(window);
                    let cascade = args["cascade"] == true;
                    if let Err(error) = self.place_canvas_card(addr, point, !cascade, cx) {
                        self.fail(error);
                    } else if let Some(card) = self.canvas_state.doc.panes.iter_mut().find(|c| c.id == addr) {
                        *card.size_mut().unwrap() = Some(size);
                        (card.rect[2], card.rect[3]) = (size[0], size[1]);
                        if !cascade { self.solve_layout(Some(addr), None, cx); }
                        self.canvas_changed(cx);
                    }
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub(super) fn canvas_changed(&mut self, cx: &mut Context<Self>) {
        // Closing a cell removes its wires.
        self.canvas_state.doc.prune_wires();
        self.canvas_state.save.dirty = true;
        self.canvas_state.save_task = None;
        self.canvas_state.save.pending = false;
        // Debounced: each change restarts the 500 ms timer. Camera animation saves once from its last frame.
        if self.canvas_state.rest.is_some() || self.canvas_state.layout_motion.is_some() || self.canvas_state.motion.target.is_some() || self.canvas_state.motion.frame { return; }
        // Mods see panes once a move settles, not every drag frame.
        if mods::publish_canvas_panes(serde_json::to_value(&self.canvas_state.doc.panes).unwrap_or_default()) { self.refresh_snapshot_mods(cx); }
        if !self.canvas_state.writable || !self.canvas_state.save.changed() {
            return;
        }
        self.canvas_state.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let Ok((doc, path, tx)) = this.update(cx, |this, _| {
                let (tx, rx) = std::sync::mpsc::channel();
                this.canvas_state.write_result = Some(rx);
                this.canvas_state.save.dirty = false;
                (this.canvas_state.doc.clone(), file("canvas.json"), tx)
            }) else {
                return;
            };
            cx.background_executor()
                .spawn(async move {
                    let _ = tx.send(doc.save(&path));
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.canvas_state.save_task = None;
                this.canvas_state.save.pending = false;
                if let Some(result) = this.canvas_state.write_result.take() {
                    if let Ok(Err(e)) = result.recv() {
                        this.canvas_state.error = Some(format!("Canvas could not save: {e}"));
                        this.canvas_state.save.dirty = true;
                    }
                }
                if this.canvas_state.save.dirty {
                    this.canvas_changed(cx);
                }
            });
        }));
    }
    /// A terminal's shown name (`pill_label`), with the name its tab was given.
    fn pill_name(&self, addr: &str) -> String {
        let given = self.model.pane_of(addr).and_then(|p| self.model.tab_of(&p)).map(|t| t.name.as_str());
        pill_label(given, self.live.get(addr).unwrap_or(&Value::Null))
    }
    fn pill_rect(&self, card: &canvas::Pane, cx: &App) -> canvas::Rect {
        if card.terminal().is_none() { return card.rect; }
        if let Some(size) = card.size() { return [card.rect[0], card.rect[1], size[0], size[1]]; }
        let rec = self.live.get(&card.id).unwrap_or(&Value::Null);
        // Room taken by the trailing status and pop-out glyphs (12 px each plus the gap).
        let trailing = 20 * (canvas_agent(rec) as u8 + self.canvas_state.popped.contains_key(&card.id) as u8);
        let text = self.pill_name(&card.id).replace(['\n', '\r'], " ");
        let family = settings::get().ui_font_family.to_string();
        let mut sizes = self.canvas_state.pill_sizes.borrow_mut();
        if let Some((label, cached_font, cached_trailing, size)) = sizes.get(&card.id)
            && label == &text
            && cached_font == &family
            && *cached_trailing == trailing
        {
            return [card.rect[0], card.rect[1], size[0], size[1]];
        }
        let run = TextRun {
            len: text.len(),
            font: font(settings::get().ui_font_family.clone()),
            ..Default::default()
        };
        let ts = WindowTextSystem::new(cx.text_system().clone());
        let width = f32::from(
            ts.shape_line(text.clone().into(), px(13.), &[run], None)
                .width(),
        );
        // Every pill is PILL_WIDTH wide; a name that does not fit wraps to a second line.
        let room = PILL_WIDTH - 2. - 24. - 16. - 8. - trailing as f32;
        let size = [PILL_WIDTH, if width > room { 54. } else { 36. }];
        sizes.insert(card.id.clone(), (text, family, trailing, size));
        [card.rect[0], card.rect[1], size[0], size[1]]
    }
    /// Menus can place a newly created terminal without duplicating membership or geometry rules.
    pub(super) fn place_canvas_card(
        &mut self,
        terminal: &str,
        point: canvas::Point,
        push: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if !point.iter().all(|v| v.is_finite()) {
            return Err("invalid canvas point".into());
        }
        if !self.live.contains_key(terminal) && self.model.pane_of(terminal).is_none()
            && !self.canvas_state.doc.panes.iter().any(|c| matches!(c.kind, canvas::Kind::Sub { .. }) && c.id == terminal)
        {
            return Err("no running terminal".into());
        }
        let card = self
            .canvas_state
            .doc
            .panes
            .iter()
            .find(|c| c.id == terminal)
            .cloned()
            .unwrap_or(canvas::Pane { id: terminal.into(), kind: canvas::Kind::Terminal { size: None },
                rect: [point[0], point[1], 240., 36.],
                z: 0, space: None, color: None, pinned: None,
            });
        let mut r = self.pill_rect(&card, cx);
        r[0] = point[0];
        r[1] = point[1];
        self.canvas_state
            .doc
            .place(terminal, [r[0], r[1]], [card.rect[2], card.rect[3]]);
        if push && self.canvas_state.rest.is_none() { self.solve_layout(Some(terminal),None,cx); }
        self.canvas_changed(cx);
        Ok(())
    }
    pub(super) fn has_canvas(&self) -> bool {
        self.chrome
            .walk()
            .iter()
            .any(|n| matches!(n.kind, Kind::Canvas))
    }
    pub(super) fn sync_canvas(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_canvas() || !self.listed_once || !core_layout_answered(self.documents.get("")) {
            return;
        }
        self.load_previews(window, cx);
        self.load_sims(window, cx);
        let missing: Vec<_> = self
            .live
            .keys()
            .filter(|a| {
                // A killed or ended terminal stays in `live` until the next listing; it is not adopted.
                self.model.pane_of(a).is_none()
                    && !self.changing_terminals.contains(*a)
                    && !self.closing_terminals.contains_key(*a)
                    && !a.contains('/')
            })
            .cloned()
            .collect();
        for addr in missing {
            let parent_space = self.live[&addr]["parent"]
                .as_str()
                .and_then(|p| self.model.pane_of(p))
                .and_then(|p| self.model.space_of(&p))
                .map(|s| s.id.clone());
            let space = parent_space
                .or_else(|| {
                    self.model
                        .spaces
                        .iter()
                        .find(|s| root_space(s))
                        .map(|s| s.id.clone())
                })
                .unwrap_or_else(|| {
                    let id = self.model.fresh_id_on("sp", None);
                    self.model.spaces.push(model::Space {
                        id: id.clone(),
                        name: "Canvas".into(),
                        folder: home(),
                        machine: None,
                        tabs: vec![],
                        active: None,
                        rest: Default::default(),
                    });
                    id
                });
            let active = self.model.active.replace(space);
            let _ = self.keep_terminal(&addr, "tab", window, cx);
            self.model.active = active;
        }
        let ready: Vec<_> = self
            .canvas_state
            .pending_points
            .iter()
            .filter(|(addr, _)| self.live.contains_key(*addr))
            .map(|(a, p)| (a.clone(), *p))
            .collect();
        for (addr, point) in ready {
            self.canvas_state.pending_points.remove(&addr);
            if let Err(error) = self.place_canvas_card(&addr, point, true, cx) {
                self.canvas_state.error = Some(error);
            }
        }
        let sizes: HashMap<_, _> = self
            .live
            .keys()
            .map(|a| {
                let r = self.pill_rect(
                    &canvas::Pane { id: a.clone(), kind: canvas::Kind::Terminal { size: None },
                        rect: [0., 0., 240., 36.],
                        z: 0, space: None, color: None, pinned: None,
                    },
                    cx,
                );
                (a.clone(), [r[2], r[3]])
            })
            .collect();
        let effective = |c: &canvas::Pane| {
            let size = c.size().or_else(|| sizes.get(&c.id).copied()).unwrap_or([240., 36.]);
            [c.rect[0], c.rect[1], size[0], size[1]]
        };
        self.canvas_state
            .pill_sizes
            .borrow_mut()
            .retain(|a, _| self.live.contains_key(a));
        let gone: Vec<_> = self
            .canvas_state
            .popped
            .keys()
            .filter(|a| !self.live.contains_key(*a))
            .cloned()
            .collect();
        for addr in gone {
            if let Some(handle) = self.canvas_state.popped.remove(&addr) {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        }
        let state = &mut self.canvas_state;
        let mut changed = false;
        let before = state.doc.spaces.len();
        state
            .doc
            .spaces
            .retain(|s| self.model.space(&s.space).is_some());
        changed |= before != state.doc.spaces.len();
        for space in &self.model.spaces {
            if !root_space(space) && !state.doc.spaces.iter().any(|s| s.space == space.id) {
                let rect = canvas::new_space_rect(&state.doc.spaces);
                state.doc.spaces.push(canvas::Space {
                    space: space.id.clone(),
                    rect,
                    color: canvas::Hue::Blue,
                    frame: true,
                });
                changed = true;
            }
        }
        if state.doc.room.as_ref().is_some_and(|id| !state.doc.spaces.iter().any(|s| &s.space == id)) {
            state.untile();
            state.doc.go(None);
            changed = true;
        }
        let before = state.doc.panes.len();
        state.doc.panes.retain(|c| {
            c.terminal().is_none() || self.live.contains_key(&c.id) || self.born.borrow().contains_key(&c.id)
        });
        changed |= before != state.doc.panes.len();
        state.selected.retain(|id|state.doc.panes.iter().any(|c|&c.id==id));
        state.moves.retain(|key, _| state.doc.panes.iter().any(|c| !c.boxed() && &c.id == key));
        let mut addresses: Vec<_> = self.live.keys().cloned().collect();
        addresses.sort_by_key(|a| (self.live[a]["created_ms"].as_u64().unwrap_or(0), a.clone()));
        // Parents are placed first even when a snapshot arrives out of order.
        for _ in 0..addresses.len().max(1) {
            let mut progress = false;
            for addr in &addresses {
                if state.doc.panes.iter().any(|c| c.terminal() == Some(addr.as_str())) {
                    continue;
                }
                let parent = self.live[addr]["parent"].as_str();
                if parent.is_some_and(|p| {
                    p != addr
                        && self.live.contains_key(p)
                        && !state.doc.panes.iter().any(|c| c.terminal() == Some(p))
                }) {
                    continue;
                }
                let space = self.model.spaces.iter().find(|s| {
                    s.tabs.iter().any(|t| {
                        t.root
                            .terminals()
                            .iter()
                            .any(|n| n.terminal() == Some(addr))
                    })
                });
                // Cells land in their own room (None for Home), among that room's cells only.
                let room = space.filter(|s| !root_space(s)).map(|s| s.id.clone());
                let occupied: Vec<_> = state.doc.panes.iter().filter(|c| !c.boxed() && c.space == room).map(effective).collect();
                let area = canvas::union(&occupied).map_or([0., 0., 960., 640.], |u| [u[0] - 24., u[1] - 24., u[2].max(912.) + 48., u[3] + 48.]);
                let r = if let Some(p) =
                    parent.and_then(|p| state.doc.panes.iter().find(|c| c.terminal() == Some(p)))
                {
                    let siblings: Vec<_> = state
                        .doc
                        .panes
                        .iter()
                        .filter(|c| {
                            !c.boxed() && self.live
                                .get(&c.id)
                                .and_then(|r| r["parent"].as_str())
                                == parent
                        })
                        .map(effective)
                        .collect();
                    canvas::child_position(effective(p), &siblings, sizes[addr])
                } else {
                    canvas::free_spot(area, &occupied, sizes[addr])
                };
                state.doc.place(addr, [r[0], r[1]], [r[2], r[3]]);
                if let Some(card) = state.doc.panes.iter_mut().find(|c| c.terminal() == Some(addr.as_str())) { card.space = room; }
                progress = true;
                changed = true;
            }
            if !progress {
                break;
            }
        }
        // The core says where a cell lives (None: Home); one it does not know yet keeps its last space.
        let members: Vec<_> = self.canvas_state.doc.panes.iter().map(|c| {
            let addr = match &c.kind {
                canvas::Kind::File { .. } | canvas::Kind::Mod { .. } => return None,
                canvas::Kind::Image { .. } | canvas::Kind::Pdf { .. } | canvas::Kind::Sim { .. } | canvas::Kind::Browser { .. } => return None,
                canvas::Kind::Sub { parent, .. } => parent,
                canvas::Kind::Terminal { .. } => &c.id,
            };
            cell_space(&self.model, &self.live, addr).map(|s| (!root_space(s)).then(|| s.id.clone()))
        }).collect();
        let state = &mut self.canvas_state;
        for (card, member) in state.doc.panes.iter_mut().zip(members) {
            if let Some(member) = member && card.space != member { card.space = member; changed = true; }
        }
        let spaces = &state.doc.spaces;
        state.doc.cameras.retain(|id, _| id.is_empty() || spaces.iter().any(|s| &s.space == id));
        if changed {
            state.doc.sync_subs(&state.subs);
            // Tile view re-tiles below instead.
            if self.canvas_state.rest.is_none() && self.canvas_state.tile.is_none() {
                let members = self.canvas_members();
                let mut groups = std::collections::HashSet::new();
                let pins: Vec<_> = self.canvas_state.doc.panes.iter().zip(members)
                    .filter(|(c,space)|!c.boxed() && groups.insert(space.clone())).map(|(c,_)|c.id.clone()).collect();
                for pin in pins { self.solve_layout(Some(&pin),None,cx); }
            }
            self.canvas_changed(cx);
            // A cell that opened or closed in a tiled place joins or leaves the tiles.
            self.tile_canvas(false, window, cx);
        }
    }
    pub(super) fn quote_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(path), Some(text)) = (a["path"].as_str(), a["text"].as_str()) else { return self.fail("quote needs path and text"); };
        // A browser pick points at a URL and may have no text.
        if a["kind"] != "browser" && (!std::path::Path::new(path).is_absolute() || text.is_empty()) { return self.fail("quote needs an absolute path and non-empty text"); }
        if name == "quote.menu" {
            // The selection stays live (copyable) until the person opens the menu from its icon.
            self.clear_quote_mark(cx);
            self.canvas_state.quote_chip = Some((a.clone(), None));
            cx.notify();
            return;
        }
        if a.get("argv").is_some() && a.get("terminal").is_none() {
            let mut args = json!({"argv":a["argv"],"space":self.canvas_state.doc.panel.space,"quote":a});
            if args["space"].is_null() { args.as_object_mut().unwrap().remove("space"); }
            self.create_canvas(args, window, cx);
            return;
        }
        let Some(addr) = a["terminal"].as_str().map(str::to_owned) else { return self.fail("quote.send needs terminal or argv"); };
        if a["kind"] == "annotation" {
            #[cfg(target_os = "macos")]
            {
                let result = self.route_annotation(addr, a, window, cx);
                return self.answer(result.map(|()| json!({})));
            }
            #[cfg(not(target_os = "macos"))]
            return self.fail("Browser and simulator annotations need macOS");
        }
        self.paste_agent(addr, a.clone(), window, cx);
    }

    fn paste_agent(&mut self, addr: String, quote: Value, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let started = Instant::now();
            let mut opened = false;
            loop {
                let done = this.update_in(cx, |this, window, cx| {
                    if started.elapsed() >= Duration::from_secs(30) {
                        this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"Text not pasted: the agent did not get ready for a paste"}}), window, cx);
                        return true;
                    }
                    let Some(rec) = this.live.get(&addr).filter(|rec| canvas_agent(rec) || crate::programs::agent_program(&rec["program"])) else { return false };
                    let text = quote.as_str().map(str::to_owned).unwrap_or_else(|| format_quote(&quote, rec.pointer("/foreground/cwd").and_then(Value::as_str).or_else(|| rec["cwd"].as_str())));
                    // Open once: the person may collapse the cell or pick another pill while the agent starts.
                    if !opened {
                        opened = true;
                        this.open_agent_for_quote(&addr, window, cx);
                    }
                    // Paste only into the target's own view.
                    let Some(term) = this.terms.get(&cell_slot(&addr)) else { return false };
                    if !term.view.read(cx).paste_quote(&text) { return false; }
                    term.focus.focus(window, cx);
                    true
                });
                if !matches!(done, Ok(false)) { break; }
                cx.background_executor().timer(Duration::from_millis(100)).await;
            }
        }).detach();
    }

    fn open_agent_for_quote(&mut self, addr: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_state.popped.contains_key(addr) { self.pop_canvas(addr.to_owned(), true, window, cx); }
        self.dispatch("panel.open", &json!({"terminal":addr}), true, window, cx);
        self.sync(window, cx);
    }

    /// Pastes each image separately, then submits after the target's visible prompt settles.
    #[cfg(target_os = "macos")]
    fn paste_annotation(&mut self, addr: String, pane: String, session: Rc<()>, outgoing: annotate::Outgoing, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let result = async {
                let mut paths = outgoing.images;
                if addr.contains('/') {
                    let addr = addr.clone();
                    paths = cx.background_executor().spawn(async move {
                        paths.into_iter().map(|path| {
                            let out = crate::spawn::command(crate::core_bin::path()).args(["put", "--json", &addr, &path])
                                .output().map_err(|e| format!("Cannot copy the annotation to {addr}: {e}"))?;
                            if !out.status.success() {
                                return Err(format!("Cannot copy the annotation to {addr}: {}", String::from_utf8_lossy(&out.stderr).trim()));
                            }
                            serde_json::from_slice::<Value>(&out.stdout).ok()
                                .and_then(|v| v["path"].as_str().map(str::to_owned))
                                .filter(|p| std::path::Path::new(p).is_absolute() && !p.chars().any(char::is_control))
                                .ok_or_else(|| format!("The annotation copy to {addr} returned no absolute path"))
                        }).collect::<Result<Vec<_>, _>>()
                    }).await?;
                }
                let paths: Vec<_> = paths.into_iter().map(|p| {
                    if p.contains(' ') { crate::terminal::quote_paths([p]).trim_end().to_owned() } else { p }
                }).collect();
                let started = Instant::now();
                let mut opened = false;
                let agent = |app: &Self| app.live.get(&addr).is_some_and(|rec| canvas_agent(rec) || crate::programs::agent_program(&rec["program"]));
                // An agent asking for approval would take the Enter as its answer.
                let asking = |app: &Self| app.live.get(&addr).is_some_and(|rec| rec["status"] == "blocked");
                const ASKING: &str = "The agent is waiting for an approval: answer it, then send again";
                let (view, paste_at, mut screen) = loop {
                    if started.elapsed() >= Duration::from_secs(30) { return Err("The agent did not get ready for a paste".into()); }
                    let ready = this.update_in(cx, |this, window, cx| {
                        if !this.annotation_sending(&pane, &session) { return Err("The annotation closed".to_owned()); }
                        if !agent(this) { return Ok(None); }
                        if asking(this) { return Err(ASKING.to_owned()); }
                        if !opened {
                            opened = true;
                            this.open_agent_for_quote(&addr, window, cx);
                        }
                        let Some(term) = this.terms.get(&cell_slot(&addr)) else { return Ok(None) };
                        let view = term.view.read(cx);
                        // Once the first paste succeeds, all pastes run in this same window update.
                        if !view.paste_own(&paths[0]) { return Ok(None); }
                        if !paths[1..].iter().all(|p| view.paste_own(p)) || !view.paste_own(&outgoing.text) {
                            return Err("The agent stopped accepting the annotation".into());
                        }
                        let stamped = view.annotation_input().ok_or("The agent stopped accepting the annotation")?;
                        let screen = view.screen_text();
                        term.focus.focus(window, cx);
                        Ok(Some((term.view.clone(), stamped, screen)))
                    }).map_err(|_| "The window closed".to_owned())??;
                    if let Some(ready) = ready { break ready; }
                    cx.background_executor().timer(Duration::from_millis(100)).await;
                };
                // From here the text sits in the agent's prompt: a failure must not invite a second paste.
                let landed = |e: String| format!("{e}. {}", annotate::NOT_SUBMITTED);
                let start = Instant::now();
                // The settle counts from the paste's first echo, so a slow echo cannot be mistaken for a settled screen.
                let mut changed: Option<Instant> = None;
                loop {
                    cx.background_executor().timer(Duration::from_millis(50)).await;
                    let submitted = this.update_in(cx, |this, _, cx| {
                        let current = this.terms.get(&cell_slot(&addr)).is_some_and(|term| term.view == view);
                        let target = view.read(cx);
                        if !this.annotation_sending(&pane, &session) || !current || !agent(this)
                            || target.annotation_input() != Some(paste_at)
                        {
                            return Err(landed("The agent's prompt changed before the annotation could be submitted".into()));
                        }
                        if asking(this) { return Err(landed(ASKING.into())); }
                        let next = target.screen_text();
                        if next != screen {
                            screen = next;
                            changed = Some(Instant::now());
                        }
                        if changed.is_some_and(|c| c.elapsed() >= Duration::from_millis(300)) || start.elapsed() >= Duration::from_secs(3) {
                            return target.press_enter().then_some(true).ok_or_else(|| landed("The agent disconnected before submit".into()));
                        }
                        Ok(false)
                    }).map_err(|_| "The window closed".to_owned())??;
                    if submitted { return Ok(()); }
                }
            }.await;
            let _ = this.update_in(cx, |this, window, cx| this.annotation_delivered(&addr, &pane, &session, result, window, cx));
        }).detach();
    }

    /// Opens the quote popover: agents wired to the quoted pane first, then other
    /// running agents (the quoted file's space first).
    fn quote_menu(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> String {
        let path = std::path::Path::new(a["path"].as_str().unwrap_or_default());
        let home = self.model.spaces.iter().filter(|s| path.starts_with(&s.folder)).max_by_key(|s| s.folder.len()).map(|s| s.id.as_str());
        // Agents wired to the quoted pane come first.
        let wired: Vec<String> = a["pane"].as_str().map(|pane| crate::wire::neighbors(&crate::wire::all(&self.canvas_state.doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned)), pane)).unwrap_or_default();
        let mut rows: Vec<_> = self.live.iter().filter(|(_, rec)| canvas_agent(rec)).map(|(addr, rec)| {
            let name = self.pill_name(addr);
            let (icon, tint) = program_icon(rec);
            let elsewhere = self.model.pane_of(addr).and_then(|p| self.model.space_of(&p)).map(|s| s.id.as_str()) != home;
            let is_wired = wired.contains(addr);
            ((!is_wired, elsewhere, name.clone(), addr.clone()), json!({"terminal":addr,"name":name,"icon":icon,"tint":tint,"wired":is_wired}))
        }).collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let mut quote = a.clone();
        quote.as_object_mut().unwrap().remove("point");
        let mut spec = json!({"mod":"quote","kind":"popup","width":280,
            "props":{"quote":quote,"agents":rows.into_iter().map(|r| r.1).collect::<Vec<_>>()}});
        if let Some(p) = a.get("point").filter(|p| p.get("x").is_some()) { spec["at"] = p.clone(); }
        let id = mods::new_overlay_id();
        self.open_overlay(id.clone(), spec, window, cx);
        id
    }

    /// A finished selection in a preview TextView shows the quote icon at the mouse-up
    /// point; the selection stays live, so it can still be copied.
    fn quote_selection(&mut self, path: &str, pane: Option<&str>, view: &Entity<gpui_base::TextViewState>, position: Point<Pixels>, _window: &mut Window, cx: &mut Context<Self>) {
        let text = view.read(cx).selected_text();
        if text.trim().is_empty() { return; }
        let mut quote = if rendered(path) {
            markdown_quote(path, &preview_source(path), &text)
        } else { code_quote(path, &text) };
        if let Some(pane) = pane { quote["pane"] = json!(pane); }
        quote["point"] = json!({"x": f32::from(position.x), "y": f32::from(position.y)});
        self.clear_quote_mark(cx);
        self.canvas_state.quote_chip = Some((quote, Some(view.clone())));
        cx.notify();
    }

    /// Where the quote icon sits: just below and right of the mouse-up point.
    fn quote_chip_bounds(quote: &Value) -> Option<Bounds<Pixels>> {
        let p = &quote["point"];
        Some(Bounds::new(point(px(p["x"].as_f64()? as f32 + 6.), px(p["y"].as_f64()? as f32 + 6.)), size(px(26.), px(26.))))
    }

    /// The quote icon, absolutely placed in the window.
    pub(super) fn quote_chip_element(&self) -> Option<AnyElement> {
        let bounds = Self::quote_chip_bounds(&self.canvas_state.quote_chip.as_ref()?.0)?;
        Some(div().id("quote-chip").absolute().left(bounds.origin.x).top(bounds.origin.y).size(bounds.size.width)
            .flex().items_center().justify_center().rounded(px(ui::RADIUS_SM)).border_1().border_color(c("border")).bg(c("surface"))
            .shadow_sm().cursor_pointer().hover(|d| d.bg(c("hover")))
            // The press itself opens the menu (quote_press); nothing under the icon takes it.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(svg().path("icons/quote.svg").size(px(14.)).text_color(c("foreground")))
            .into_any_element())
    }

    fn clear_quote_mark(&mut self, cx: &mut Context<Self>) {
        if let Some((view, _)) = self.canvas_state.quote_mark.take() {
            view.update(cx, |v, cx| v.clear_range_highlights(cx));
        }
    }

    /// A left press on the quote icon opens the quote menu there, marking the selection
    /// first (the press would clear it). Any other press drops the icon, and the
    /// highlight unless it lands in the open popover.
    pub(super) fn quote_press(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((quote, view)) = self.canvas_state.quote_chip.take() {
            cx.notify();
            if Self::quote_chip_bounds(&quote).is_some_and(|b| b.contains(&position)) {
                let id = self.quote_menu(&quote, window, cx);
                let marked = view.as_ref().is_some_and(|view| {
                    let range = view.read(cx).selected_source_range().and_then(|r| view.read(cx).rendered_text().range_for_source(r));
                    range.is_some_and(|range| view.update(cx, |v, cx| {
                        let ok = v.set_range_highlights([gpui_base::RangeHighlight::new(range, c("selection"))], cx).is_ok();
                        if ok { v.clear_selection(cx); }
                        ok
                    }))
                });
                if marked { self.canvas_state.quote_mark = Some((view.unwrap(), id)); }
                return;
            }
        }
        if self.canvas_state.quote_mark.as_ref().is_some_and(|(_, id)| !self.overlays.iter().any(|o| &o.id == id)) {
            self.clear_quote_mark(cx);
        }
    }

    pub(super) fn canvas_command(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // An entered cell takes the zoom keys (and the View menu) for its own text.
        if matches!(name, "canvas.zoom_in" | "canvas.zoom_out" | "canvas.zoom_reset") {
            let entered = self.canvas_state.entered().filter(|k| self.canvas_state.doc.panes.iter().any(|c| c.id == *k && c.terminal().is_some()));
            if let Some(t) = entered.and_then(|k| self.terms.get(&cell_slot(k))) {
                let factor = match name { "canvas.zoom_in" => Some(1.1), "canvas.zoom_out" => Some(1. / 1.1), _ => None };
                t.view.update(cx, |v, cx| v.zoom_text(factor, cx));
                return;
            }
        }
        let bounds = self
            .bounds
            .borrow()
            .get("canopy-canvas")
            .copied()
            .unwrap_or(Bounds::new(point(px(0.), px(0.)), window.viewport_size()));
        let anchor = [
            f32::from(bounds.size.width) / 2.,
            f32::from(bounds.size.height) / 2.,
        ];
        // Fit the selection, else everything in the current place.
        let fit_rects = if name.starts_with("canvas.fit") {
            let all = name == "canvas.fit_all" || self.canvas_state.selected.is_empty();
            self.place_rects(|key| all || self.canvas_state.selected.contains(key), cx)
        } else { vec![] };
        let range = self.zoom_range(window, cx);
        let from = self.canvas_state.doc.camera;
        let state = &mut self.canvas_state;
        if let Some((_, target, ..)) = state.motion.target { state.doc.camera = target; }
        match name {
            "view.agents" | "view.all" | "view.terminals" => {
                state.doc.view = match name {
                    "view.agents" => canvas::View::Agents,
                    "view.terminals" => canvas::View::Terminals,
                    _ => canvas::View::All,
                };
                state.selected.retain(|key| state.doc.panes.iter().any(|card|
                    !card.boxed() && card.id == *key && canvas::view_allows(&state.doc.view, matches!(card.kind, canvas::Kind::Sub { .. }) || self.live.get(key).is_some_and(canvas_agent))));
                self.focus.focus(window, cx);
            }
            "panel.close" => {
                state.dirty = None;
                if !state.tabs.is_empty() {
                    state.closed = Some((state.file.take(), std::mem::take(&mut state.tabs)));
                }
                state.file = None;
                state.files = false;
                state.panel_expanded = false;
                self.focus.focus(window, cx);
            }
            "canvas.zoom_reset" => state.doc.camera.zoom_at(1f32.clamp(range[0], range[1]), anchor),
            "canvas.fit" | "canvas.fit_all" => {
                canvas::focus_fit(&mut state.doc.camera, &mut state.previous_camera,
                    &fit_rects, [anchor[0] * 2., anchor[1] * 2.]);
            }
            _ => state
                .doc
                .camera
                .step(if name == "canvas.zoom_in" { 1 } else { -1 }, anchor, canvas::widen(range, from.zoom)),
        }
        if name.starts_with("canvas.") {
            let to = state.doc.camera;
            state.doc.camera = from;
            state.motion.target = Some((from, to, std::time::Instant::now(), anchor));
            self.animate_camera(window, cx);
        } else { self.canvas_changed(cx); }
        self.sync(window, cx);
        cx.notify();
    }
    fn preview_card(
        &self,
        card: &canvas::Pane,
        file: &str,
        view: &Entity<gpui_base::TextViewState>,
        camera: canvas::Camera,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = std::path::Path::new(file);
        let quote_pane = card.id.clone();
        let quote_path = file.to_owned();
        let quote_view = view.clone();
        let body = div().flex_1().min_h_0().overflow_hidden().px(px(16.)).py(px(12.)).child(gpui_base::TextView::new(view).scrollable(true)
                    .image_source(|_| {
                        static BLANK: std::sync::LazyLock<Arc<gpui::Image>> = std::sync::LazyLock::new(|| Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Svg, b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'/>".to_vec())));
                        gpui::ImageSource::Image(BLANK.clone())
                    })
                    .on_link_click(|_, _, _, _| {}))
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, e: &MouseUpEvent, window, cx| this.quote_selection(&quote_path, Some(&quote_pane), &quote_view, e.position, window, cx))).into_any_element();
        self.pane_card(card, "mods/files/file.svg".into(), path.file_name().unwrap_or_default().to_string_lossy().into_owned(), path.parent().unwrap_or(path).to_string_lossy().into_owned(), None, body, camera, cx)
    }

    /// A pane header: a fixed screen-size row (icon 16, gap 8, text 13/18) at any zoom. Callers
    /// add their children and listeners; overflowing text is cut by the pane's own clipping.
    fn pane_header() -> Div {
        div().flex().items_start().px(px(12.)).py(px(9.)).gap(px(8.)).text_size(px(13.)).line_height(px(18.))
            .text_color(c("foreground")).overflow_hidden()
    }

    /// Unselected panes show their effective tint in the frame.
    fn pane_frame_color(&self, id: &str) -> Hsla {
        let state = &self.canvas_state;
        if state.selected.iter().any(|key| key == id) {
            c("ring")
        } else {
            state.doc.effective_color(id).map_or_else(|| c("border"), |hue| c(&hue.tint_token()))
        }
    }

    /// The shared saved-rect frame, header drag and resize corner for boxed panes. It occludes
    /// the panes beneath it and raises itself on a click. `header` (a browser pane's controls)
    /// replaces the icon, title and detail inside the same header.
    #[allow(clippy::too_many_arguments)]
    fn pane_card(&self, card: &canvas::Pane, icon: SharedString, title: String, detail: String, header: Option<AnyElement>, body: AnyElement, camera: canvas::Camera, cx: &mut Context<Self>) -> AnyElement {
        let p = camera.to_screen([card.rect[0], card.rect[1]]);
        let drag = card.id.clone();
        let color_menu = drag.clone();
        let raise = drag.clone();
        let browser = card.url().is_some();
        #[cfg(target_os = "macos")]
        let rest = card.id.clone();
        let card_element = div().id(SharedString::from(format!("preview-{}", card.id))).absolute().left(px(p[0])).top(px(p[1]))
                .w(px(card.rect[2] * camera.zoom)).h(px(card.rect[3] * camera.zoom))
                .flex().flex_col().bg(c("surface")).border_1().border_color(self.pane_frame_color(&card.id)).rounded(px(ui::RADIUS)).overflow_hidden()
                .occlude()
                // Capture phase lets clicks in the header and body raise the pane.
                .capture_any_mouse_down(cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    if e.button != MouseButton::Left { return; }
                    this.raise_pane(&raise, cx);
                    if browser {
                        this.select_browser_pane(&raise);
                        cx.notify();
                    }
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        #[cfg(target_os = "macos")]
        let card_element = card_element.when(card.url().is_some(), |d| d.on_mouse_down_out(
            cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                let state = &this.canvas_state;
                if matches!(e.button, MouseButton::Left | MouseButton::Right)
                    && state.drag.is_none() && !state.space_down && !state.hand
                {
                    this.rest_live_browser(&rest, cx);
                }
            }),
        ));
        card_element
                .child(Self::pane_header().h(px(canvas::header_height(&card.kind))).flex_none().border_b_1().border_color(c("border"))
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                        this.open_menu("canvas-colors", json!({"pane":color_menu}), Some(e.position), window, cx);
                        cx.stop_propagation();
                    }))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                        this.begin_layout();
                        this.canvas_state.selected = vec![drag.clone()];
                        this.canvas_state.shown = Some(this.canvas_state.doc.clone());
                        this.canvas_state.preview_drag = Some((drag.clone(), None, e.position)); cx.stop_propagation();
                    }))
                    .map(|head| match header {
                        Some(header) => head.p(px(0.)).child(header),
                        None => head
                            .child(div().flex_none().mt(px(1.)).child(svg().path(icon).size(px(16.)).text_color(c("foreground"))))
                            .child(div().min_w_0().truncate().child(title))
                            .child(div().flex_1().min_w_0().truncate().text_color(c("muted-foreground")).child(detail)),
                    }))
                .child(div().flex_1().min_h_0().overflow_hidden().child(body))
                .children(Self::resize_zones(card, camera.zoom, cx))
                .into_any_element()
    }

    /// Eight resize hitboxes shared by expanded terminals and boxed panes.
    fn resize_zones(card: &canvas::Pane, zoom: f32, cx: &mut Context<Self>) -> [gpui::Stateful<Div>; 8] {
        let (edge, corner) = resize_zone_sizes(zoom);
        let boxed = card.boxed();
        let browser = card.url().is_some();
        let [_, _, width, height] = card.pinned.unwrap_or(card.rect);
        [(-1i8, -1i8), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)].map(|zone| {
            let id = card.id.clone();
            let thick = |other: i8| px(if other == 0 { edge } else { corner });
            let cursor = match zone {
                (0, _) => CursorStyle::ResizeUpDown,
                (_, 0) => CursorStyle::ResizeLeftRight,
                (a, b) if a == b => CursorStyle::ResizeUpLeftDownRight,
                _ => CursorStyle::ResizeUpRightDownLeft,
            };
            let d = div().id(SharedString::from(format!("canvas-resize-{id}-{}-{}", zone.0, zone.1)))
                .absolute().cursor(cursor).block_mouse_except_scroll();
            let d = match zone.0 { -1 => d.left_0().w(thick(zone.1)), 1 => d.right_0().w(thick(zone.1)), _ => d.left(px(corner)).right(px(corner)) };
            let d = match zone.1 { -1 => d.top_0().h(thick(zone.0)), 1 => d.bottom_0().h(thick(zone.0)), _ => d.top(px(corner)).bottom(px(corner)) };
            d.when(boxed && zone == (1, 1), |d| d.rounded_br(px(ui::RADIUS))
                .border_r_2().border_b_2().border_color(c("muted-foreground"))
                .hover(|d| d.border_color(c("foreground"))))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.begin_layout();
                    this.raise_pane(&id, cx);
                    if browser { this.select_browser_pane(&id); }
                    let state = &mut this.canvas_state;
                    state.shown = Some(state.doc.clone());
                    state.resize_zone = zone;
                    if boxed {
                        state.preview_drag = Some((id.clone(), Some([width, height]), e.position));
                    } else {
                        state.selected = vec![id.clone()];
                        state.cell_resize = Some((id.clone(), e.position));
                    }
                    cx.stop_propagation();
                    cx.notify();
                }))
        })
    }

    /// Shortcut hints for an empty canvas, like VS Code's empty editor: only actions that have a key.
    fn empty_hints(&self) -> Option<AnyElement> {
        let keys = crate::keys::listing();
        let rows: Vec<_> = [
            ("New terminal", "canvas.new"),
            ("New agent", "canvas.agents"),
            ("Go to pane", "mod.open:picker"),
            ("Command palette", "app.palette"),
            ("Fit all panes", "canvas.fit"),
        ].into_iter().filter_map(|(label, command)| {
            let binding = action_binding(&keys, command)?;
            let caps = crate::keys::caps(&binding).into_iter().map(|cap| {
                div().min_w(px(18.)).h(px(18.)).px(px(4.)).flex().items_center()
                    .justify_center().rounded(px(4.)).bg(c("popover")).border_1()
                    .border_color(c("border")).text_size(px(11.)).text_color(c("muted-foreground")).child(cap)
            });
            Some(div().flex().items_center().h(px(28.)).gap(px(12.))
                .child(div().w(px(160.)).text_right().text_size(px(12.)).text_color(c("muted-foreground")).child(label))
                .child(div().w(px(160.)).flex().gap(px(4.)).children(caps)))
        }).collect();
        (!rows.is_empty()).then(|| {
            div().absolute().inset_0().flex().items_center().justify_center()
                .child(div().flex().flex_col().children(rows)).into_any_element()
        })
    }

    pub(super) fn render_canvas(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = &self.canvas_state;
        // A pane that leaves the selection leaves sync input; under two, sync ends.
        let synced = {
            let mut synced = state.synced.borrow_mut();
            synced.retain(|a| state.selected.contains(a));
            if synced.len() < 2 { synced.clear(); }
            crate::terminal::set_synced(self.terms.values().filter(|t| synced.contains(&t.address))
                .map(|t| (t.address.clone(), t.view.read(cx).sync_handle())).collect());
            synced.clone()
        };
        let shown_doc = state.shown.as_ref().unwrap_or(&state.doc);
        let camera = state.doc.camera;
        let bounds = self.bounds.borrow().get("canopy-canvas").copied();
        let viewport = bounds
            .map(|b| [f32::from(b.size.width), f32::from(b.size.height)])
            .unwrap_or([1200., 800.]);
        let maps = self.bounds.clone();
        let bg = self.person.canvas_color.as_deref().unwrap_or("canvas");
        let panning = matches!(state.drag, Some((None, _)));
        let view = cx.entity();
        let edges = self.wire_edges();
        let mut area = div()
            .id("canopy-canvas")
            .relative()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .bg(c(bg))
            .map(|d| match (&state.drag, state.space_down || state.hand) {
                (None | Some((Some(_), _)), true) => d.cursor(CursorStyle::OpenHand),
                _ => d,
            })
            // Middle drag, or left drag with Space held, starts a camera pan.
            .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                let state = &mut this.canvas_state;
                if e.button == MouseButton::Left && state.wire.sticky { return; }
                state.motion.target = None;
                state.motion.velocity = [0.; 2];
                state.motion.sample = Some(std::time::Instant::now());
                if matches!(e.button, MouseButton::Middle | MouseButton::Navigate(_))
                    || (e.button == MouseButton::Left && (state.space_down || state.hand))
                {
                    state.selected.clear();
                    state.drag = Some((None, e.position));
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    move |b, _, _| {
                        let mut maps = maps.borrow_mut();
                        maps.insert("canopy-canvas".into(), b);
                        #[cfg(target_os = "macos")]
                        maps.insert("browser-canvas".into(), b);
                    },
                    move |b, _, window, _| {
                        // Pane hitboxes hide the area behind them, but window listeners still
                        // see gestures and drags that cross from one pane or surface to another.
                        let down = view.clone();
                        window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture || e.button != MouseButton::Left
                                || !b.contains(&e.position) || down.read(cx).canvas_overlay_at(e.position)
                            { return; }
                            down.update(cx, |this, cx| {
                                if this.wire_pick(e.position, window, cx) {
                                    cx.stop_propagation();
                                }
                            });
                        });
                        let up = view.clone();
                        window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture || e.button != MouseButton::Left { return; }
                            up.update(cx, |this, cx| {
                                if this.wire_release(e.position, window, cx) {
                                    cx.stop_propagation();
                                } else if this.canvas_state.resizing_pane() || this.canvas_state.move_pointer().is_some() {
                                    this.end_canvas_drag(cx);
                                    this.canvas_changed(cx);
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            });
                        });
                        let move_view = view.clone();
                        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture { return; }
                            move_view.update(cx, |this, cx| {
                                if b.contains(&e.position) && this.wire_pointer(e, cx) {
                                    cx.stop_propagation();
                                    return;
                                }
                                let resize = this.canvas_state.resizing_pane();
                                if e.pressed_button == Some(MouseButton::Left)
                                    && (resize || this.queue_canvas_move(e.position, window, cx))
                                {
                                    if resize { this.apply_resize(e.position, [0., 0.], window, cx); }
                                    let state = &mut this.canvas_state;
                                    if (resize || state.drag_live) && edge_pan(e.position, b) != [0., 0.] {
                                        let last = state.edge_drag.map_or_else(std::time::Instant::now, |(_, last)| last);
                                        state.edge_drag = Some((e.position, last));
                                        this.layout_frame(window, cx);
                                    } else {
                                        state.edge_drag = None;
                                    }
                                    cx.stop_propagation();
                                } else if e.pressed_button.is_none() && (resize || this.canvas_state.move_pointer().is_some()) {
                                    this.end_canvas_drag(cx);
                                    this.canvas_changed(cx);
                                    cx.notify();
                                }
                            });
                        });
                        let exit = view.clone();
                        window.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && exit.read(cx).canvas_state.edge_drag.is_some() {
                                exit.update(cx, |this, _| this.canvas_state.edge_drag = None);
                            }
                        });
                        let pinch = view.clone();
                        window.on_mouse_event(move |e: &PinchEvent, phase, window, cx| {
                            if phase == DispatchPhase::Capture && b.contains(&e.position)
                                && !pinch.read(cx).canvas_overlay_at(e.position)
                            {
                                pinch.update(cx, |this, cx| this.canvas_pinch(e, window, cx));
                            }
                        });
                        let wheel = view.clone();
                        window.on_mouse_event(move |e: &ScrollWheelEvent, phase, window, cx| {
                            if !b.contains(&e.position) || wheel.read(cx).canvas_overlay_at(e.position) { return; }
                            // Capture before inactive panes can scroll. Bubble after eligible children,
                            // so a terminal that owns keyboard focus can scroll its own output.
                            let claim = match phase {
                                DispatchPhase::Capture => wheel.read(cx).claims_wheel(e),
                                DispatchPhase::Bubble => true,
                            };
                            if claim {
                                wheel.update(cx, |this, cx| this.canvas_wheel(e, window, cx));
                            }
                        });
                        if panning {
                            window.set_window_cursor_style(CursorStyle::ClosedHand);
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(self.wire_layer(viewport, false, &edges))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                let origin = this
                    .bounds
                    .borrow()
                    .get("canopy-canvas")
                    .map(|b| b.origin)
                    .unwrap_or_default();
                let p = window.mouse_position() - origin;
                let point = this
                    .canvas_state
                    .doc
                    .camera
                    .to_world([f32::from(p.x), f32::from(p.y)]);
                for path in paths.paths() {
                    this.open_preview(json!({"path":path,"point":point}), window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    if this.wire_menu(e.position, window, cx) { cx.stop_propagation(); return; }
                    let origin = this
                        .bounds
                        .borrow()
                        .get("canopy-canvas")
                        .map(|b| b.origin)
                        .unwrap_or_default();
                    let point = this.canvas_state.doc.camera.to_world([
                        f32::from(e.position.x - origin.x),
                        f32::from(e.position.y - origin.y),
                    ]);
                    let space = this.space_at(e.position);
                    let machine = space
                        .as_deref()
                        .and_then(|s| this.model.space(s))
                        .and_then(|s| s.machine.clone());
                    this.open_menu(
                        "canvas-empty",
                        json!({"point":point,"space":space,"machine":machine}),
                        Some(e.position),
                        window,
                        cx,
                    );
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                // Reached only on empty canvas: pills, space cards and controls stop propagation.
                // A click takes the keyboard back to the canvas; dragging draws a selection box
                // (Shift adds to the selection). Space+drag, middle drag and scrolling pan.
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    if this.wire_click(e.position, window, cx) { return; }
                    this.canvas_state.mode = true;
                    this.focus.focus(window, cx);
                    let base = if e.modifiers.shift { this.canvas_state.selected.clone() } else { vec![] };
                    this.canvas_state.selected = base.clone();
                    this.canvas_state.marquee = Some((e.position, e.position, base));
                    this.canvas_state.pan_moved = false;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                // The button came up outside the window: end any drag instead of panning on hover.
                if e.pressed_button.is_none()
                    && matches!(this.canvas_state.drag, Some((None, _)))
                {
                    this.end_canvas_drag(cx);
                    this.canvas_changed(cx);
                    cx.notify();
                    return;
                }
                if let Some((start, _, base)) = this.canvas_state.marquee.clone() {
                    if e.pressed_button.is_none() { this.canvas_state.marquee = None; cx.notify(); return; }
                    this.canvas_state.pan_moved = true;
                    let (a, b) = (this.wire_local(start), this.wire_local(e.position));
                    let camera = this.canvas_state.doc.camera;
                    let (p, q) = (camera.to_world([a[0].min(b[0]), a[1].min(b[1])]), camera.to_world([a[0].max(b[0]), a[1].max(b[1])]));
                    let area = [p[0], p[1], q[0] - p[0], q[1] - p[1]];
                    let mut picked = base;
                    for (key, rect) in this.keyboard_cells(cx) {
                        if canvas::intersects(area, rect) && !picked.contains(&key) { picked.push(key); }
                    }
                    this.canvas_state.selected = picked;
                    this.canvas_state.marquee = Some((start, e.position, this.canvas_state.marquee.take().map(|m| m.2).unwrap_or_default()));
                    cx.notify();
                    return;
                }
                let Some((None, previous)) = this.canvas_state.drag else { return };
                let d = [
                    f32::from(e.position.x - previous.x),
                    f32::from(e.position.y - previous.y),
                ];
                let now = std::time::Instant::now();
                let dt = this.canvas_state.motion.sample.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.016).max(0.001);
                this.canvas_state.motion.velocity = [d[0]/dt, d[1]/dt];
                this.canvas_state.motion.sample = Some(now);
                this.canvas_state.doc.camera.pan(d);
                this.canvas_state.pan_moved = true;
                this.canvas_state.drag.as_mut().unwrap().1 = e.position;
                this.canvas_changed(cx);
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, window, cx| {
                    // A click (no box drawn) inside a space: its files go to the panel tree.
                    let click = this.canvas_state.marquee.take().is_some() && !this.canvas_state.pan_moved;
                    this.end_canvas_drag(cx);
                    this.animate_camera(window, cx);
                    if let Some(space) = click.then(|| this.space_at(e.position)).flatten() {
                        this.show_space_files(space, window, cx);
                    }
                    cx.notify();
                }),
            )
            // Tile view follows any click, wherever it lands (terminals keep their mouse-up):
            // once the event is done, a new selection becomes the large tile and drags snap back.
            .capture_any_mouse_up(cx.listener(|this, _: &MouseUpEvent, window, cx| {
                if this.canvas_state.tile.is_some() {
                    cx.defer_in(window, |this, window, cx| this.tile_canvas(false, window, cx));
                }
            }));
        // Middle and the side buttons pan; any of them ending the pan.
        for button in [MouseButton::Middle, MouseButton::Navigate(NavigationDirection::Back), MouseButton::Navigate(NavigationDirection::Forward)] {
            area = area.on_mouse_up(button, cx.listener(|this, _, window, cx| {
                this.end_canvas_drag(cx);
                this.animate_camera(window, cx);
                cx.notify();
            }));
        }
        // Home shows each space as a card; double-click it (or Enter) to go inside.
        for space in shown_doc.spaces.iter().filter(|_| shown_doc.room.is_none()) {
            if self.model.spaces.iter().any(|s| s.id == space.space && root_space(s)) {
                continue;
            }
            let id = space.space.clone();
            let menu_id = id.clone();
            let label = self.model.space(&id).map(|s| self.model.space_name(s, &home())).unwrap_or_default();
            let inside: Vec<_> = shown_doc.panes.iter().filter(|c| c.terminal().is_some() && c.space.as_ref() == Some(&id))
                .map(|c| self.live.get(&c.id).unwrap_or(&Value::Null)).collect();
            let working = inside.iter().filter(|r| canvas_agent(r) && r["status"] == "working").count();
            let blocked = inside.iter().any(|r| canvas_agent(r) && r["status"] == "blocked");
            let summary = match (inside.len(), working) {
                (0, _) => "Empty".to_owned(),
                (n, 0) => format!("{n} terminal{}", if n == 1 { "" } else { "s" }),
                (n, w) => format!("{n} terminal{} · {w} working", if n == 1 { "" } else { "s" }),
            };
            let z = camera.zoom;
            let [x, y] = camera.to_screen([space.rect[0], space.rect[1]]);
            let selected = state.selected.contains(&id);
            let rename_id = id.clone();
            let color = c(space.color.name());
            // Like a Figma frame: the name sits above a rounded box tinted with the space's color.
            let name = div()
                .id(SharedString::from(format!("space-label-{id}")))
                .absolute().top(px(-24.)).left_0().w_full().h(px(20.))
                .flex().items_center().gap(px(6.))
                .text_size(px(12.)).text_color(c(if selected { "foreground" } else { "muted-foreground" }))
                .child(div().flex_none().size(px(8.)).rounded_full().bg(color))
                .child(div().flex_1().min_w_0().truncate().map(|d| {
                    if let Some((editing, input, _)) = &state.rename {
                        if *editing == Rename::Space(id.clone()) { return d.child(gpui_base::Input::new(input)); }
                    }
                    d.child(label)
                }))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    // Double-click the name to rename, as in Figma; the box's double-click goes inside.
                    if e.click_count == 2 {
                        this.rename_canvas(Rename::Space(rename_id.clone()), window, cx);
                        cx.stop_propagation();
                    }
                }));
            area = area.child(
                div()
                    .id(SharedString::from(format!("space-card-{id}")))
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(space.rect[2] * z))
                    .h(px(space.rect[3] * z))
                    .rounded(px(ui::RADIUS_LG * z))
                    .border_1()
                    .border_color(if selected { c("ring") } else { color.opacity(0.35) })
                    .bg(color.opacity(self.person.space_fill))
                    .p(px(12. * z))
                    .flex()
                    .flex_col()
                    .justify_end()
                    .child(name)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6. * z))
                            .text_size(px(12. * z))
                            .text_color(c("muted-foreground"))
                            .when(blocked, |d| d.child(svg().path("frame/settings/bell.svg").size(px(12. * z)).text_color(c("muted-foreground"))))
                            .child(summary),
                    )
                    .capture_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                        if e.keystroke.key == "escape" && this.canvas_state.rename.is_some() {
                            this.canvas_state.rename = None;
                            this.focus.focus(window, cx);
                            cx.stop_propagation(); cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                            if this.canvas_state.rename.as_ref().is_some_and(|(editing, _, _)| *editing == Rename::Space(id.clone())) {
                                cx.stop_propagation(); return;
                            }
                            if e.click_count == 2 {
                                this.go_room(Some(id.clone()), window, cx);
                                cx.stop_propagation(); return;
                            }
                            if e.modifiers.shift {
                                if this.canvas_state.selected.contains(&id) { this.canvas_state.selected.retain(|s| s != &id); }
                                else { this.canvas_state.selected.push(id.clone()); }
                            } else { this.canvas_state.selected = vec![id.clone()]; }
                            this.begin_layout();
                            this.canvas_state.shown = Some(this.canvas_state.doc.clone());
                            this.canvas_state.space_drag = Some((id.clone(), false, e.position));
                            this.show_space_files(id.clone(), window, cx);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                            this.open_menu("canvas-space", json!({"id":menu_id}), Some(e.position), window, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        }
        // Every pane kind joins one z-sorted list, so overlapping panes stack the same whatever they are.
        let mut pills: Vec<((bool, u64), AnyElement)> = Vec::new();
        let mut drawn_sims = vec![];
        let mut drawn_pages = vec![];
        // An expanded browser fills the canvas area: no other pane is drawn.
        for card in shown_doc.panes.iter().filter(|p| state.browser_tab.is_none() && shown_doc.shows(p.space.as_ref()) && canvas::visible(camera, p.rect, viewport)) {
            let element = match &card.kind {
                // Drawn by the terminal path below.
                canvas::Kind::Terminal { .. } | canvas::Kind::Sub { .. } => continue,
                canvas::Kind::File { path } => {
                    let Some((view, _)) = state.previews.get(path) else { continue };
                    self.preview_card(card, path, view, camera, cx)
                }
                canvas::Kind::Image { path } => self.image_card(card, path, camera, cx),
                canvas::Kind::Pdf { path, .. } => {
                    let Some(element) = self.pdf_card(card, path, camera, cx) else { continue };
                    element
                }
                canvas::Kind::Browser { .. } => {
                    drawn_pages.extend(self.page_key(&card.id));
                    let header = self.browser_chrome(&card.id, browser::Host::Pane, cx);
                    let body = self.browser_body(card, camera, cx);
                    self.pane_card(card, SharedString::default(), String::new(), String::new(), Some(header), body, camera, cx)
                }
                canvas::Kind::Mod { .. } => {
                    let name = card.mod_name().unwrap();
                    let (icon, title, body) = if let Some(hint) = self.folders.get(name).and_then(|f| f.hint.pane.as_ref()) {
                        (SharedString::from(hint.icon.clone()), hint.label.clone(), self.render_slot(&format!("pane:{}", card.id), cx))
                    } else {
                        ("icons/x.svg".into(), card.kind_name().to_owned(), div().p(px(12.)).text_color(c("muted-foreground")).child(format!("Mod {name} is not installed")).into_any_element())
                    };
                    self.pane_card(card, icon, title, String::new(), None, body, camera, cx)
                }
                canvas::Kind::Sim { .. } => {
                    drawn_sims.push(card.id.clone());
                    let (title, detail) = self.sim_title(card, cx);
                    let body = self.sim_body(card, cx);
                    let body = self.annotate_body(card, body, annotate::card_body(card, camera.zoom), cx);
                    #[cfg(target_os = "macos")]
                    let body = div().relative().size_full().child(body)
                        .when_some(self.sim_bottom_bar(card, camera.zoom, cx), |d, bar| d.child(
                            div().absolute().bottom(px(12.)).left_0().right_0().flex().justify_center().child(bar)))
                        .into_any_element();
                    self.pane_card(card, "icons/smartphone.svg".into(), title, detail, None, body, camera, cx)
                }
            };
            pills.push((paint_order(card), element));
        }
        // A live simulator streams only while its pane is drawn.
        #[cfg(target_os = "macos")]
        self.show_sims(&drawn_sims, cx);
        // A resting page that leaves sight is checked once for media still playing.
        self.watch_pages(&drawn_pages, cx);
        let cards: HashMap<_, _> = shown_doc
            .panes
            .iter()
            .filter(|card| {
                card.terminal().is_some() && shown_doc.shows(card.space.as_ref()) && canvas::view_allows(
                    &state.doc.view,
                    self.live
                        .get(&card.id)
                        .is_some_and(|r| canvas_agent(r)),
                )
            })
            .map(|card| {
                // An expanded cell has its own size, eased by the layout motion.
                let size = card.size().map(|_| [card.rect[2].max(PILL_WIDTH), card.rect[3].max(canvas::HEADER)]).unwrap_or_else(|| state
                    .pill_sizes
                    .borrow()
                    .get(&card.id)
                    .map(|v| v.3)
                    .unwrap_or([240., 36.]));
                (
                    card.id.as_str(),
                    (card, [card.rect[0], card.rect[1], size[0], size[1]]),
                )
            })
            .collect();
        let shown: Vec<_> = cards
            .values()
            .filter(|(_, rect)| canvas::visible(camera, *rect, viewport))
            .map(|(card, rect)| {
                let mut card = (*card).clone();
                card.rect = *rect;
                card
            })
            .collect();
        let mut sub_pills = Vec::new();
        if state.doc.view != canvas::View::Terminals {
            for card in state.doc.panes.iter().filter(|c| matches!(c.kind, canvas::Kind::Sub { .. }) && state.doc.shows(c.space.as_ref())) {
                let canvas::Kind::Sub { parent, sub: sub_id } = &card.kind else { continue };
                let Some(sub) = state.subs.get(parent).into_iter().flatten().find(|s| &s.id == sub_id) else { continue };
                if canvas::visible(camera, card.rect, viewport) { sub_pills.push((parent.clone(), sub.clone(), card.clone())); }
            }
        }
        for (parent, sub, card) in sub_pills {
            let rect = card.rect;
            let key = card.id.clone();
            let color_menu = key.clone();
            let [x, y] = camera.to_screen([rect[0], rect[1]]);
            let z = camera.zoom;
            let (icon, tint) = program_icon(&json!({"program":sub.program}));
            let mut pill = Self::pane_header().id(SharedString::from(format!("canvas-sub-{parent}-{}", sub.id)))
                .absolute().left(px(x)).top(px(y)).w(px(rect[2] * z)).h(px(rect[3] * z))
                .rounded(px(ui::RADIUS * z)).border_1().border_color(self.pane_frame_color(&key)).bg(c("surface"))
                .when(sub.done, |d| d.opacity(0.5)).occlude()
                .child(div().flex_none().mt(px(1.)).child(icon_element(&icon, tint, px(16.), c("foreground"))));
            let text = div().flex_1().min_w_0().child(div().text_ellipsis().when(sub.done, |d| d.text_color(c("muted-foreground")))
                .child(format!("{} · {}", sub.name, if sub.done { "done" } else { "working" })));
            pill = pill.child(text);
            let click_sub = sub.clone();
            pill = pill.on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                this.begin_layout();
                this.canvas_state.selected = vec![key.clone()];
                this.canvas_state.drag = Some((Some(key.clone()), e.position));
                this.focus.focus(window, cx);
                if let Some(path) = click_sub.transcript.as_ref() {
                    this.open_preview(json!({"path":path}), window, cx);
                }
                cx.notify();
                cx.stop_propagation();
            }));
            pill = pill.on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                this.open_menu("canvas-colors", json!({"pane":color_menu}), Some(e.position), window, cx);
                cx.stop_propagation();
            }));
            pills.push((paint_order(&card), pill.into_any_element()));
        }
        let mut shown = shown;
        shown.sort_by_key(|c| c.z);
        for card in shown {
            let addr = card.id.clone();
            let drag_addr = addr.clone();
            let rec = self.live.get(&addr).unwrap_or(&Value::Null);
            let agent = canvas_agent(rec);
            let name = self.pill_name(&addr);
            let [x, y] = camera.to_screen([card.rect[0], card.rect[1]]);
            let z = camera.zoom;
            // An expanded cell is a pill (its header) over a terminal; it keeps its world size at every zoom.
            let expanded = card.size().is_some();
            let [cw, ch] = [card.rect[2], card.rect[3]];
            let is_synced = synced.contains(&addr);
            let frame_color = if is_synced { c("accent") } else { self.pane_frame_color(&addr) };
            // A collapsed pill is a box at world size; its header contents keep their screen size, clipped.
            let mut pill = Self::pane_header()
                .id(SharedString::from(format!("canvas-{addr}")))
                .when(!expanded, |d| d.absolute().left(px(x)).top(px(y)).rounded(px(ui::RADIUS * z)).border_1().border_color(frame_color).bg(c("surface")).occlude())
                .w(px(cw * z))
                .h(px(if expanded { canvas::HEADER } else { ch * z }));
            let (icon, tint) = program_icon(rec);
            // Glyphs sit on the first line, so a two-line name keeps its icon top-left.
            pill = pill.child(div().flex_none().mt(px(1.)).child(icon_element(&icon, tint, px(16.), c("foreground"))));
            // On an expanded cell, double-clicking the name edits its tab's name in place (a pill enters instead).
            let tab_id = self.model.pane_of(&addr).and_then(|p| self.model.tab_of(&p)).map(|t| t.id.clone()).filter(|_| expanded);
            let editing = tab_id.as_ref().and_then(|id| state.rename.as_ref().filter(|(t, _, _)| *t == Rename::Tab(id.clone())));
            let name_box = div().flex_1().min_w_0().line_clamp(if expanded { 1 } else { 2 }).text_ellipsis()
                .when_some(editing, |d, (_, input, _)| d.child(gpui_base::Input::new(input)))
                .when(editing.is_none(), |d| d.child(name.clone()))
                .id(SharedString::from(format!("canvas-name-{addr}")));
            let pill_badge = is_synced.then(|| div().flex_none().px(px(5.)).rounded(px(3.)).border_1().border_color(c("accent"))
                .text_size(px(10.)).line_height(px(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c("accent")).child("SYNC"));
            pill = pill.child(match tab_id {
                Some(id) => name_box.on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    let editing = this.canvas_state.rename.as_ref().is_some_and(|(t, _, _)| *t == Rename::Tab(id.clone()));
                    if editing || e.click_count == 2 {
                        if !editing { this.rename_canvas(Rename::Tab(id.clone()), window, cx); }
                        cx.stop_propagation();
                    }
                })).into_any_element(),
                None => name_box.into_any_element(),
            }).children(pill_badge);
            if agent && rec["status"] == "working" {
                pill = pill.child(
                    svg()
                        .path("mods/workspace-chrome/spinner.svg")
                        .size(px(12.))
                        .mt(px(3.))
                        .text_color(c("muted-foreground"))
                        .with_animation(
                            "working",
                            Animation::new(Duration::from_millis(900)).repeat(),
                            |svg, delta| {
                                svg.with_transformation(Transformation::rotate(percentage(delta)))
                            },
                        ),
                );
            } else if agent && rec["status"] == "blocked" {
                pill = pill.child(
                    svg()
                        .path("frame/settings/bell.svg")
                        .size(px(12.))
                        .mt(px(3.))
                        .text_color(c("muted-foreground")),
                );
            }
            if state.popped.contains_key(&addr) {
                pill = pill.child(
                    svg()
                        .path("icons/external-link.svg")
                        .size(px(12.))
                        .mt(px(3.))
                        .text_color(c("muted-foreground")),
                );
            }
            let menu_addr = addr.clone();
            pill = pill.capture_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "escape" && this.canvas_state.rename.as_ref().is_some_and(|(t, _, _)| matches!(t, Rename::Tab(_))) {
                    this.canvas_state.rename = None;
                    this.focus.focus(window, cx);
                    cx.stop_propagation(); cx.notify();
                }
            }));
            pill = pill.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    this.open_card_menu(&menu_addr, e.position, window, cx);
                    cx.stop_propagation();
                }),
            );
            let pill = pill.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    if this.canvas_state.popped.contains_key(&addr) {
                        this.pop_canvas(addr.clone(), false, window, cx);
                        cx.stop_propagation();
                        return;
                    }
                    this.canvas_state.pan_moved = false;
                    this.raise_pane(&drag_addr, cx);
                    this.begin_layout();
                    if e.modifiers.shift {
                        if this.canvas_state.selected.contains(&drag_addr) { this.canvas_state.selected.retain(|id|id!=&drag_addr); }
                        else { this.canvas_state.selected.push(drag_addr.clone()); }
                    } else if !this.canvas_state.selected.contains(&drag_addr) { this.canvas_state.selected = vec![drag_addr.clone()]; }
                    this.canvas_state.drag = Some((Some(drag_addr.clone()), e.position));
                    cx.stop_propagation();
                    this.canvas_state.mode = true;
                    this.focus.focus(window, cx);
                    if e.click_count == 2 {
                        // Double-click enters the cell: it expands and its terminal takes the keyboard.
                        this.canvas_state.selected = vec![drag_addr.clone()];
                        this.canvas_state.drag = None;
                        this.canvas_keyboard("canvas.enter", &json!({}), window, cx);
                        return;
                    }
                    if let Some(p) = this.model.pane_of(&addr) {
                        this.model.focus_pane(&p);
                    }
                    this.focus.focus(window, cx);
                    cx.notify();
                }),
            );
            if !expanded {
                pills.push((paint_order(&card), pill.into_any_element()));
                continue;
            }
            // The body: the terminal, or a flat stand-in when too far out to read. It draws live at every zoom frame; its PTY grid waits for `settled`.
            let slot = cell_slot(&card.id);
            // The entered cell always draws live: its terminal holds the keyboard, Cmd+Esc included.
            let entered = state.entered() == Some(card.id.as_str());
            let live = self.terms.get(&slot).filter(|_| entered || z >= canvas::EXPANDED_MIN_ZOOM);
            let body_size = size(px(card.rect[2] - 2.), px(card.rect[3] - canvas::header_world(z) - 2.));
            // A tile squeezed below readable size shows who it is instead, and its program keeps its grid.
            let compact = state.tile.is_some() && !entered && (cw < canvas::TILE_COMPACT[0] || ch < canvas::TILE_COMPACT[1]);
            if let Some(t) = self.terms.get(&slot) {
                let resizing = state.cell_resize.as_ref().is_some_and(|(key, _)| key == &card.id);
                // The grid follows the cell only once the zoom rests, else a pinch resizes the program every frame.
                let settled = !resizing && !compact && self.camera_rested();
                t.view.update(cx, |v, cx| {
                    // Unentered cells let the wheel through, so the canvas pans over them.
                    v.wheel_needs_focus = true;
                    v.set_scale(z, settled.then_some(body_size), (!self.person.canvas_resize_terminals).then_some(body_size), cx);
                });
            }
            let (select_addr, mode_addr) = (card.id.clone(), card.id.clone());
            let body = div()
                .id(SharedString::from(format!("canvas-body-{}", card.id)))
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .capture_any_mouse_down(cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    // Only the entered cell's terminal takes clicks. Elsewhere a click selects the
                    // cell (and leaves the entered one), a drag moves it, a double-click enters it.
                    let entered = this.canvas_state.entered() == Some(mode_addr.as_str());
                    if !entered && e.button == MouseButton::Left {
                        this.raise_pane(&mode_addr, cx);
                        this.canvas_state.mode = true;
                        this.focus.focus(window, cx);
                        if e.click_count == 2 {
                            this.canvas_state.selected = vec![mode_addr.clone()];
                            this.canvas_keyboard("canvas.enter", &json!({}), window, cx);
                        } else {
                            if e.modifiers.shift {
                                if this.canvas_state.selected.contains(&mode_addr) { this.canvas_state.selected.retain(|k| k != &mode_addr); }
                                else { this.canvas_state.selected.push(mode_addr.clone()); }
                            } else if !this.canvas_state.selected.contains(&mode_addr) { this.canvas_state.selected = vec![mode_addr.clone()]; }
                            this.begin_layout();
                            this.canvas_state.pan_moved = false;
                            this.canvas_state.drag = Some((Some(mode_addr.clone()), e.position));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    // The terminal took the focus; the cell is the one selected, and the canvas does not pan.
                    // A synced pane keeps the selection, or typing into it would end sync input.
                    if !this.canvas_state.synced.borrow().contains(&select_addr) { this.canvas_state.selected = vec![select_addr.clone()]; }
                    this.raise_pane(&select_addr, cx);
                    if let Some(p) = this.model.pane_of(&select_addr) { this.model.focus_pane(&p); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(match live.filter(|_| !compact) {
                    Some(t) => shown_terminal(&t.view, cx),
                    None if compact => div().size_full().flex().flex_col().items_center().justify_center().gap(px(8. * z)).p(px(8. * z))
                        .bg(c("surface")).text_size(px(13. * z)).text_color(c("foreground"))
                        .child(icon_element(&icon, tint, px(28. * z), c("foreground")))
                        .child(div().max_w_full().truncate().child(name.clone()))
                        .into_any_element(),
                    None => div().size_full().bg(crate::terminal::hsla(crate::vt::theme().bg)).into_any_element(),
                });
            pills.push((paint_order(&card), div()
                .id(SharedString::from(format!("canvas-cell-{}", card.id)))
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(card.rect[2] * z))
                .h(px(card.rect[3] * z))
                .rounded(px(ui::RADIUS * z))
                .when(is_synced, |d| d.border_2()).when(!is_synced, |d| d.border_1())
                .border_color(frame_color)
                .bg(c("surface"))
                .overflow_hidden()
                .flex()
                .flex_col()
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pill)
                .child(body)
                .children(Self::resize_zones(&card, z, cx))
                .into_any_element()));
        }
        pills.sort_by_key(|(z, _)| *z);
        for (_, pill) in pills { area = area.child(pill); }
        if !shown_doc.panes.iter().any(|p| shown_doc.shows(p.space.as_ref()))
            && (state.doc.room.is_some() || self.model.spaces.iter().all(|s| root_space(s)))
        {
            area = area.children(self.empty_hints());
        }
        area = area.child(self.wire_layer(viewport, true, &edges)).children(self.wire_handles(cx));
        if let Some((a, b, _)) = &state.marquee {
            let (a, b) = (self.wire_local(*a), self.wire_local(*b));
            area = area.child(div().absolute().left(px(a[0].min(b[0]))).top(px(a[1].min(b[1])))
                .w(px((a[0] - b[0]).abs())).h(px((a[1] - b[1]).abs()))
                .border_1().border_color(c("ring")).bg(c("ring").opacity(0.08)));
        }
        area = area.children(self.helper_bars(shown_doc, cx));
        // The bar's controls opt out of window dragging.
        let control = || div().flex().items_center().gap(px(4.)).on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        // Expanded, the file panel fills the window: no space tabs, canvas or canvas controls.
        let expanded = state.panel_expanded && (state.file.is_some() || state.files);
        let strip = if expanded { bg } else { "chrome" };
        // The expanded browser's tab is the active one; the place it came from draws inactive.
        let home_active = state.doc.room.is_none() && state.browser_tab.is_none();
        let home_tab = strip_tab("canvas-home", home_active, c(bg), c(strip)).group("canvas-home").role(gpui::Role::Tab).aria_label("Home").aria_selected(home_active).flex_none().w(px(36.)).h(px(BAR)).flex().items_center().justify_center()
            .cursor_pointer()
            .text_color(c(if home_active { "foreground" } else { "muted-foreground" }))
            .hover(|d| d.text_color(c("foreground")))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .tooltip(strip_tooltip("Home".into(), Some("canvas.home")))
            .child(svg().path("icons/house.svg").size(px(14.)).text_color(c(if home_active { "foreground" } else { "muted-foreground" })).group_hover("canvas-home", |s| s.text_color(c("foreground"))))
            .on_click(cx.listener(|this, _, window, cx| this.go_room(None, window, cx)));
        // The expanded browser's tab sits right after its pane's space tab, else after Home.
        let browser_space = state.browser_tab.as_ref().and_then(|id| state.doc.panes.iter().find(|p| &p.id == id)).and_then(|p| p.space.clone())
            .filter(|id| self.model.spaces.iter().any(|s| s.id == *id && !root_space(s)));
        let mut browser_tab = self.browser_tab_strip(c(bg), c(strip), cx);
        let mut space_tabs = div().id("canvas-space-tabs").role(gpui::Role::TabList).flex().min_w_0().h_full().overflow_x_scroll();
        for space in self.model.spaces.iter().filter(|s| !root_space(s)) {
            let active = state.doc.room.as_ref() == Some(&space.id) && state.browser_tab.is_none();
            let id = space.id.clone();
            let close = id.clone();
            let group = SharedString::from(format!("space-tab-{id}"));
            let status = self.model.rollup(&space.tabs.iter().flat_map(|t| crate::model::addresses(&t.root)).collect::<Vec<_>>());
            let name = self.model.space_name(space, &home());
            space_tabs = space_tabs.child(strip_tab(group.clone(), active, c(bg), c(strip)).group(group.clone()).role(gpui::Role::Tab).aria_selected(active).aria_label(name.clone()).flex().items_center()
                .h_full().w(px(180.)).min_w(px(96.)).gap(px(6.)).px(px(12.))
                .text_size(px(12.)).text_color(c(if active { "foreground" } else { "muted-foreground" }))
                .hover(|d| d.text_color(c("foreground"))).cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| this.go_room(Some(id.clone()), window, cx)))
                .child(svg().path("mods/files/folder.svg").flex_none().size(px(14.)).text_color(c(if active { "foreground" } else { "muted-foreground" })).group_hover(group.clone(), |s| s.text_color(c("foreground"))))
                .child(div().flex_1().min_w_0().truncate().child(name))
                .when(matches!(status, Some("working" | "blocked")), |d| d.child(div().flex_none().size(px(6.)).rounded_full().bg(c("accent"))))
                .child(panel_button("space-tab-close", "mods/pane/close.svg", c("muted-foreground"), c("hover"))
                    .flex_none().aria_label("Close space").when(!active, |d| d.opacity(0.).group_hover(group, |d| d.opacity(1.)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.run_key("space.close", json!({"id":close}), window, cx);
                    }))));
            if browser_space.as_ref() == Some(&space.id) { space_tabs = space_tabs.children(browser_tab.take()); }
        }
        let crumbs = div().flex().flex_1().min_w_0().h_full().items_center()
            .child(home_tab).children(browser_tab).child(space_tabs)
            .child(panel_button("canvas-space-new", "mods/workspace-chrome/plus.svg", c("muted-foreground"), c("hover"))
                .flex_none().aria_label("New space").tooltip(strip_tooltip("New space".into(), Some("space.new")))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, window, cx| this.run_key("space.new", json!({}), window, cx))));
        let mut tabs = control();
        // Like Claude's titlebar: the folder opens or closes the files pane (tree and preview).
        let open = state.file.is_some() || state.files;
        tabs = tabs.child(
            panel_button("canvas-files", "mods/files/folder.svg", c("muted-foreground"), c("hover"))
                .aria_label("Files").tooltip(strip_tooltip("Files".into(), None))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if open {
                        this.canvas_command("panel.close", window, cx);
                    } else {
                        this.canvas_state.files = true;
                        this.canvas_state.doc.panel.tree_hidden = false;
                        // Bring back the tabs it closed with; files and terminals closed since are gone.
                        if let Some((file, tabs)) = this.canvas_state.closed.take() {
                            this.canvas_state.tabs = tabs;
                            this.canvas_state.file = file;
                        }
                        this.sync(window, cx);
                        cx.notify();
                    }
                })),
        );
        // Browser (the macOS web view) and simulator (a Mac with Xcode) open pinned at the top
        // right, beside this bar; their helper bar moves them onto the canvas.
        let pinned_new = |id: &'static str, icon: &'static str, label: &'static str, kind: &'static str| panel_button(id, icon, c("muted-foreground"), c("hover"))
            .aria_label(label).tooltip(strip_tooltip(label.into(), None))
            .on_click(cx.listener(move |this, _, window, cx| this.run_key("canvas.pane_new", json!({"kind":kind,"pinned":true}), window, cx)));
        tabs = control()
            .when(cfg!(target_os = "macos"), |d| d.child(pinned_new("canvas-browser", "icons/globe.svg", "Browser", "browser")))
            .when(crate::sim::available(), |d| d.child(pinned_new("canvas-sim", "icons/smartphone.svg", "Simulator", "sim")))
            .child(panel_button("canvas-tile", "icons/layout-grid.svg", c("muted-foreground"), c("hover"))
            .aria_label("Tile view").tooltip(strip_tooltip("Tile view".into(), Some("canvas.tile")))
            .on_click(cx.listener(|this, _, window, cx| this.tile_canvas(true, window, cx))))
            .child(tabs);
        let mut bar = self.window_drag(div().flex().flex_none().items_center().justify_between().w_full().h(px(BAR))
            .pl(px(bar_inset(cfg!(target_os = "macos"), self.fullscreen)))
            .bg(c(strip)), cx);
        if !expanded {
            // Like Claude's header, the canvas controls sit at the right end, beside the panel.
            bar = bar.child(crumbs.child(div().flex_1().h_full())).child(tabs.flex_none().px(px(8.)));
        }
        if self.connect_hint() {
            area = area.child(div().absolute().bottom_0().left_0().right_0().h(px(32.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(self.render_slot("canvas-keys", cx)));
        }
        let mut row = div()
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .when(!expanded, |d| d.child(match self.expanded_browser(c(bg)) { Some(page) => page, None => area.into_any_element() }));
        // The panel's border and resize grip run the whole window height, over the bar too.
        let mut grip = None;
        if state.file.is_some() || state.files {
            let full_width = self
                .bounds
                .borrow()
                .get("root")
                .map(|b| f32::from(b.size.width))
                .unwrap_or(viewport[0]);
            let width = state
                .doc
                .panel
                .width
                .unwrap_or(full_width * 0.45)
                .max(360.)
                .min(full_width * 0.75);
            let width = if expanded { full_width } else { width };
            let mut panel = div()
                .relative()
                .w(px(width))
                .h_full()
                .flex_none()
                .flex()
                .flex_col()
                .bg(c(bg));
            // The panel's left border: a 1 px line from the window top to bottom, in a 6 px
            // grab area; it lights up on hover or drag.
            grip = (!expanded).then(||
                div()
                    .id("canvas-panel-resize")
                    .group("canvas-panel-resize")
                    .absolute()
                    .right(px(width - 3.))
                    .top_0()
                    .w(px(6.))
                    .h_full()
                    .flex()
                    .justify_center()
                    .cursor(CursorStyle::ResizeLeftRight)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.canvas_state.resizing = true;
                            cx.stop_propagation();
                        }),
                    )
                    .child(div().w(px(1.)).h_full().bg(c("border"))
                        .group_hover("canvas-panel-resize", |d| d.bg(c("ring")))
                        .when(state.resizing, |d| d.bg(c("ring")))),
            );
            let mut tabs = div().id("canvas-tabs").role(gpui::Role::TabList).flex().flex_1().min_w_0().h_full().overflow_x_scroll();
            for tab in &state.tabs {
                let tab = tab.clone();
                let active = state.file.as_ref() == Some(&tab);
                let dirty = state.dirty.as_ref() == Some(&tab);
                let label = crate::subs::transcript_title(&tab).unwrap_or_else(|| std::path::Path::new(&tab).file_name().unwrap_or_default().to_string_lossy().into_owned());
                let activate = tab.clone();
                let close = tab.clone();
                let group = SharedString::from(format!("panel-tab-{tab}"));
                tabs = tabs.child(strip_tab(group.clone(), active, c("hover"), c(bg)).group(group.clone()).role(gpui::Role::Tab).aria_selected(active).aria_label(label.clone()).flex().items_center().h_full()
                    .w(px(180.)).min_w(px(96.)).px(px(12.)).gap(px(6.))
                    .text_size(px(12.)).text_color(c(if active { "foreground" } else { "muted-foreground" }))
                    .hover(|d| d.text_color(c("foreground"))).cursor_pointer()
                    .tooltip(strip_tooltip(tab, None))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.canvas_state.file.as_ref() == Some(&activate) { return; }
                        this.canvas_state.dirty = None;
                        this.canvas_state.file = Some(activate.clone());
                        this.focus.focus(window, cx);
                        this.sync(window, cx);
                        cx.notify();
                    }))
                    .child(svg().path("mods/files/file.svg").flex_none().size(px(14.)).text_color(c(if active { "foreground" } else { "muted-foreground" })).group_hover(group.clone(), |s| s.text_color(c("foreground"))))
                    .child(div().flex_1().min_w_0().truncate().child(label))
                    .child(div().relative().flex_none().size(px(24.)).flex().items_center().justify_center()
                        .when(dirty, |d| d.child(div().absolute().size(px(6.)).rounded_full().bg(c("foreground"))
                            .group_hover(group.clone(), |d| d.opacity(0.))))
                        .child(panel_button("tab-close", "mods/pane/close.svg", c("muted-foreground"), c("hover"))
                            .aria_label("Close file").when(dirty || !active, |d| d.opacity(0.).group_hover(group, |d| d.opacity(1.)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if this.canvas_state.dirty.as_ref() == Some(&close) { this.canvas_state.dirty = None; }
                                this.canvas_state.file = canvas::close_panel_tab(
                                    &mut this.canvas_state.tabs, this.canvas_state.file.as_deref(), &close);
                                this.sync(window, cx);
                                cx.notify();
                            })))));
            }
            // The panel's part of the bar shares its color, like Claude's side panel header.
            bar = bar.child(div().flex().flex_none().when(expanded, |d| d.flex_1()).when(!expanded, |d| d.w(px(width))).h_full().items_center()
                .pr(px(8.)).gap(px(4.))
                .bg(c(bg)).border_b_1().border_color(c("border"))
                .child(tabs)
                .child(panel_button("panel-expand", if expanded { "mods/pane/unzoom.svg" } else { "mods/pane/zoom.svg" }, c("muted-foreground"), c("hover"))
                    .aria_label("Expand or restore panel").tooltip(strip_tooltip(if expanded { "Restore panel" } else { "Expand panel" }.into(), None))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.canvas_state.panel_expanded = !this.canvas_state.panel_expanded;
                        this.sync(window, cx);
                        cx.notify();
                    })))
                .child(panel_button("canvas-panel-close", "mods/pane/close.svg", c("muted-foreground"), c("hover"))
                    .aria_label("Close panel").tooltip(strip_tooltip("Close panel".into(), Some("panel.close")))
                    .on_click(cx.listener(|this, _, window, cx| this.canvas_command("panel.close", window, cx)))));
            let root = self.panel_root();
            let path = state.file.as_deref().or(root.as_deref()).unwrap_or("");
            // Outside the tree's folder, paths read from home.
            // A sub-agent reads as itself, not as a file in its agent's data folder.
            let breadcrumb = match crate::subs::transcript_title(path) {
                Some(_) => "Sub-agent · read-only".to_owned(),
                None => canvas::breadcrumb(path, root.as_deref().filter(|r| std::path::Path::new(path).starts_with(r)).or(Some(&home()))),
            };
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(12.))
                    .h(px(36.))
                    .flex_none()
                    .border_b_1()
                    .border_color(c("border"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .text_size(px(12.))
                            .text_color(c("muted-foreground"))
                            .child(breadcrumb),
                    )
                    // An agent transcript is read, not edited.
                    .when(state.file.as_ref().is_some_and(|f| !f.ends_with(".jsonl")), |d| {
                        let editors = self.panel_editors();
                        let choice = state.doc.panel.editor.as_deref()
                            .filter(|key| *key == "here" || editors.iter().any(|e| e.key == *key))
                            .unwrap_or_else(|| editors.first().map(|e| e.key.as_str()).unwrap_or("here"));
                        let editor = editors.iter().find(|e| e.key == choice);
                        let label = editor.map(|e| format!("Open in {}", e.name)).unwrap_or_else(|| "Open here".into());
                        let icon = editor.map(|e| e.icon.clone()).unwrap_or_else(|| "mods/workspace-chrome/square-pen.svg".into());
                        let tint = editor.is_none_or(|e| e.tint);
                        let choice = choice.to_owned();
                        let path = state.file.clone().unwrap();
                        let menu_path = path.clone();
                        d.child(div().flex().items_center().h(px(26.)).border_1().border_color(c("border")).rounded(px(ui::RADIUS_SM)).overflow_hidden()
                            .child(div().id("panel-editor").flex().items_center().gap(px(6.)).px(px(8.)).h_full().cursor_pointer()
                                .text_size(px(12.)).text_color(c("foreground")).hover(|s| s.bg(c("hover")))
                                .on_click(cx.listener(move |this, _, window, cx| this.panel_editor(&json!({"choice":choice,"path":path}), window, cx)))
                                .child(icon_element(&icon, tint, px(14.), c("foreground"))).child(label))
                            .child(div().id("panel-editor-menu").flex().items_center().justify_center().w(px(25.)).h_full()
                                .border_l_1().border_color(c("border")).cursor_pointer().hover(|s| s.bg(c("hover")))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let editors: Vec<Value> = this.panel_editors().iter().map(|e| json!({"key":e.key,"name":e.name,"icon":e.icon,"tint":e.tint})).collect();
                                    this.open_menu("canvas-editors", json!({"path":menu_path,"editors":editors}), None, window, cx);
                                    this.check_editors(window, cx);
                                }))
                                .child(svg().path("mods/files/chevron-down.svg").size(px(12.)).text_color(c("muted-foreground")))))
                    })
                    .when(state.file.is_some(), |d| {
                        d.child(
                            panel_button(
                                "panel-pin",
                                "mods/workspace-chrome/pin.svg",
                                c("muted-foreground"),
                                c("hover"),
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    if let Some(path) = this.canvas_state.file.clone() {
                                        this.open_preview(
                                            json!({"path":path,"place":"canvas"}),
                                            window,
                                            cx,
                                        );
                                    }
                                },
                            )),
                        )
                    })
                    .child(
                        panel_button(
                            "panel-tree",
                            "mods/workspace-chrome/panel-right.svg",
                            c("muted-foreground"),
                            c("hover"),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.canvas_state.doc.panel.tree_hidden =
                                !this.canvas_state.doc.panel.tree_hidden;
                            this.canvas_changed(cx);
                            this.sync(window, cx);
                            cx.notify();
                        })),
                    ),
            );
            let mut body = div().flex().flex_1().min_h_0();
            if let Some((view, _)) = state
                .file
                .as_ref()
                .filter(|p| {
                    state.editing.as_ref() != Some(*p) && rendered(p)
                })
                .and_then(|p| state.previews.get(p))
            {
                let quote_path = state.file.clone().unwrap();
                let quote_view = view.clone();
                // Padding sits on the TextView so its scrollbar overlay reaches the panel edge.
                body = body.child(div().flex_1().min_w_0().h_full().child(gpui_base::TextView::new(view).scrollable(true).px(px(24.)).py(px(20.))
                    .image_source(|_| {
                        static BLANK: std::sync::LazyLock<Arc<gpui::Image>> = std::sync::LazyLock::new(|| Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Svg, b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'/>".to_vec())));
                        gpui::ImageSource::Image(BLANK.clone())
                    }).on_link_click(|_,_,_,_|{}))
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, e: &MouseUpEvent, window, cx| this.quote_selection(&quote_path, None, &quote_view, e.position, window, cx))));
            } else if state.file.is_some() {
                body = body.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .child(self.render_slot("canvas-file", cx)),
                );
            } else {
                body = body.child(
                    div().flex_1().flex().items_center().justify_center().text_size(px(12.))
                        .text_color(c("muted-foreground")).child("Select a file to preview it"),
                );
            }
            if !state.doc.panel.tree_hidden {
                body = body.child(
                    div()
                        .w(px(275.))
                        .h_full()
                        .flex_none()
                        .border_l_1()
                        .border_color(c("border"))
                        .child(self.render_slot("canvas-tree", cx)),
                );
            }
            panel = panel.child(body);
            row = row.child(panel);
        }
        div().relative().size_full().flex().flex_col().child(bar).child(row).children(grip)
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, window, cx| {
                if this.canvas_state.resizing {
                    let width = f32::from(window.viewport_size().width - e.position.x);
                    this.canvas_state.panel_expanded = false;
                    this.canvas_state.doc.panel.width = Some(width.clamp(
                        360.,
                        (f32::from(window.viewport_size().width) * 0.75).max(360.),
                    ));
                    this.canvas_changed(cx);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.canvas_state.resizing = false;
                    this.canvas_state.drag = None;
                    this.canvas_state.space_drag = None;
                }),
            )
            .into_any_element()
    }
}

/// Height of the bar the canvas draws across the window top (px at zoom 1).
const BAR: f32 = 36.;

/// Space left of the bar's Home tab: macOS traffic lights sit there, except in full screen.
fn bar_inset(macos: bool, fullscreen: bool) -> f32 {
    if macos && !fullscreen { 76. } else { 12. }
}

/// How long the zoom must be still before an expanded cell draws live text again.
const ZOOM_REST: Duration = Duration::from_millis(160);

/// How long a plain wheel pan keeps owning the wheel after its last event (covers momentum).
const WHEEL_GESTURE: Duration = Duration::from_millis(150);

/// Screen pixels per second to pan while dragging within 32 pixels of an edge.
fn edge_pan(pointer: Point<Pixels>, bounds: Bounds<Pixels>) -> canvas::Point {
    if !bounds.contains(&pointer) { return [0., 0.]; }
    let ramp = |distance: Pixels| (1. - f32::from(distance) / 32.).max(0.);
    let x = ramp(pointer.x - bounds.left()) - ramp(bounds.right() - pointer.x);
    let y = ramp(pointer.y - bounds.top()) - ramp(bounds.bottom() - pointer.y);
    let speed = 900. / x.hypot(y).max(1.);
    [x * speed, y * speed]
}

/// Edges and corners stay grabbable even when the pane is zoomed out.
fn resize_zone_sizes(zoom: f32) -> (f32, f32) {
    ((6. * zoom).max(8.), (12. * zoom).max(8.))
}

/// Panes paint by z, with pinned panes over every canvas pane.
pub(super) fn paint_order(p: &canvas::Pane) -> (bool, u64) { (p.pinned.is_some(), p.z) }

/// Whether the wheel with these modifiers zooms the canvas (Cmd on macOS, Ctrl elsewhere).
fn wheel_zooms(m: Modifiers) -> bool {
    if cfg!(target_os = "macos") { m.platform } else { m.control }
}

fn gesture_ended(phase: TouchPhase) -> bool {
    matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled)
}

/// Whether the canvas takes a wheel event before the cell under it: always when zooming, and
/// for a plain wheel while a pan that began on the canvas last moved less than `WHEEL_GESTURE` ago.
fn wheel_claimed(zoom: bool, last_pan: Option<std::time::Instant>, now: std::time::Instant) -> bool {
    zoom || last_pan.is_some_and(|t| now.saturating_duration_since(t) < WHEEL_GESTURE)
}

impl AppWindow {
    fn claims_wheel(&self, e: &ScrollWheelEvent) -> bool {
        wheel_claimed(wheel_zooms(e.modifiers), self.canvas_state.wheel_pan_at, std::time::Instant::now())
            || (gesture_ended(e.touch_phase) && self.canvas_state.wheel_zoom.is_some())
            || self.wire_cell_at(e.position, 0.).is_some_and(|pane| self.canvas_state.entered() != Some(&pane))
    }
}

/// Whether the core has answered for this machine's layout (with a document, or that it has none).
/// Before that the model has no spaces, and pruning would wipe every saved space card; `sync`
/// runs again when the answer arrives.
fn core_layout_answered(slot: Option<&crate::mirror_sync::Slot>) -> bool {
    slot.is_some_and(|s| s.document.is_some() || s.read_attempted)
}

/// The window slot of an expanded cell's terminal view.
pub(super) fn cell_slot(addr: &str) -> String {
    format!("canvas:{addr}")
}

/// Hover labels use the window-owned tooltip, including the current key binding when present.
struct StripTooltip(SharedString, Option<SharedString>);
impl Render for StripTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_base::Tooltip::new("strip-tooltip").m(px(4.)).px(px(8.)).py(px(4.))
            .rounded(px(6.)).bg(c("popover")).text_color(c("foreground")).text_size(px(12.))
            .child(gpui_base::h_flex().gap(px(8.)).child(self.0.clone())
                .when_some(self.1.clone(), |d, key| d.child(div().text_color(c("muted-foreground")).child(key))))
    }
}
fn strip_tooltip(label: String, command: Option<&str>) -> impl Fn(&mut Window, &mut App) -> AnyView + use<> {
    let command = command.map(str::to_owned);
    move |_, cx| {
        let key = command.as_deref().and_then(|command| action_key(&crate::keys::listing(), command));
        cx.new(|_| StripTooltip(label.clone().into(), key.map(Into::into))).into()
    }
}

fn action_key(keys: &[(String, String, String)], command: &str) -> Option<String> {
    action_binding(keys, command).map(|k| crate::keys::pretty(&k))
}

/// Prefer the binding used while navigating the canvas.
fn action_binding(keys: &[(String, String, String)], command: &str) -> Option<String> {
    keys.iter().filter(|(_, _, action)| action == command)
        .min_by_key(|(section, _, _)| match section.as_str() { "canvas-mode" => 0, "layout:main" => 1, "global" => 2, _ => 3 })
        .map(|(_, key, _)| key.clone())
}

/// A top-strip tab: radius 8 on top and, when active, outward bottom curves that join it
/// to what's below. GPUI can't round a corner outward, so each curve is an 8 px square of
/// the tab's `fill` under a square of the `strip` color rounded on the corner facing the tab.
fn strip_tab(id: impl Into<ElementId>, active: bool, fill: Hsla, strip: Hsla) -> Stateful<Div> {
    let curve = |left: bool| {
        div().absolute().bottom_0().size(px(ui::RADIUS)).bg(fill)
            .map(|d| if left { d.left(px(-ui::RADIUS)) } else { d.right(px(-ui::RADIUS)) })
            .child(div().size_full().bg(strip)
                .map(|d| if left { d.rounded_br(px(ui::RADIUS)) } else { d.rounded_bl(px(ui::RADIUS)) }))
    };
    div().id(id).relative().rounded_t(px(ui::RADIUS))
        .when(active, |d| d.bg(fill).child(curve(true)).child(curve(false)))
}

/// A 24 px plain icon button for canvas and panel controls.
fn panel_button(id: &'static str, icon: &'static str, fg: Hsla, hover: Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .group(id)
        .role(gpui::Role::Button)
        .size(px(24.))
        .flex_none()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ui::RADIUS_SM))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(svg().path(icon).size(px(14.)).text_color(fg).group_hover(id, |s| s.text_color(c("foreground"))))
}

/// What a terminal is called on its pill, panel tab and quote row: a name the
/// person gave it, else an agent's program name ("Claude Code"), else "terminal"
/// for a shell, else the command's basename ("uv"), never its flags. An agent's own
/// title is left out so the name is stable, including before the agent reports.
fn pill_label(given: Option<&str>, rec: &Value) -> String {
    let program = &rec["program"];
    let key = program["key"].as_str().unwrap_or_default();
    let shown = program["name"].as_str().unwrap_or_default();
    let generic = |s: &str| s.is_empty() || s == "shell" || s == "terminal" || s == key || s == shown;
    if let Some(name) = given.or(rec["name"].as_str()).map(str::trim).filter(|s| !generic(s)) {
        return name.to_owned();
    }
    if crate::programs::agent_program(program) || canvas_agent(rec) {
        return if shown.is_empty() { "agent" } else { shown }.to_owned();
    }
    if program["shell"] == true {
        return "terminal".into();
    }
    let mut argv: Vec<&str> = rec.pointer("/foreground/argv").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    // The app's launch wrapper (`zsh -lic '"$@"; exec "$0" -li' zsh <cmd>...`, see programs::resolve) names <cmd>.
    if argv.len() > 4 && argv[2].contains("\"$@\"") { argv.drain(..4); }
    match argv.first().and_then(|a| a.rsplit('/').next()).filter(|b| !b.is_empty()) {
        Some(base) => base.to_owned(),
        None if shown.is_empty() => "terminal".into(),
        None => shown.to_owned(),
    }
}

/// A program's icon and whether to draw it in one color (`tint`) or its own colors.
fn program_icon(rec: &Value) -> (String, bool) {
    let program = &rec["program"];
    let fallback = if canvas_agent(rec) { "icons/agent.svg" } else { "icons/shell.svg" };
    (program["icon"].as_str().unwrap_or(fallback).to_owned(), program["tint"].as_bool().unwrap_or(true))
}

/// Tinted glyphs are one-color masks; brand icons keep their colors as images.
fn icon_element(icon: &str, tint: bool, size: Pixels, color: Hsla) -> AnyElement {
    if tint {
        svg().path(SharedString::from(icon.to_owned())).size(size).flex_none().text_color(color).into_any_element()
    } else {
        img(icon.to_owned()).size(size).flex_none().into_any_element()
    }
}

fn canvas_agent(rec: &Value) -> bool {
    rec["program"]["agent"] == true
        || rec["status"]
            .as_str()
            .is_some_and(|s| matches!(s, "idle" | "working" | "blocked" | "done"))
}

/// The argv that starts what a live cell runs: its program's `run` among the startable agents (None for a plain shell).
fn like_run(rec: &Value, startable: &[Value]) -> Option<Vec<String>> {
    let key = rec["program"]["key"].as_str()?;
    let entry = startable.iter().find(|e| e["key"] == key)?;
    serde_json::from_value(entry["run"].clone()).ok()
}

/// The space a cell belongs to; unkept children have no pane, so they share their ancestor's.
fn cell_space<'a>(model: &'a model::Model, live: &HashMap<String,Value>, terminal: &str) -> Option<&'a model::Space> {
    let mut address = terminal;
    for _ in 0..=live.len() {
        if let Some(space) = model.pane_of(address).and_then(|p|model.space_of(&p)) { return Some(space); }
        address = live.get(address)?["parent"].as_str()?;
    }
    None
}

/// The spaces shown as top strip tabs, in tab order (⌘1..9 count these).
pub(super) fn strip_spaces(model: &model::Model) -> Vec<String> {
    model.spaces.iter().filter(|s| !root_space(s)).map(|s| s.id.clone()).collect()
}

/// The space ctrl-tab enters: the most recent other one (`back`: the oldest), among `ids`.
pub(super) fn recent_space(recent: &[String], ids: &[String], current: Option<&String>, back: bool) -> Option<String> {
    let mut known: Vec<&String> = recent.iter().filter(|r| ids.contains(r) && Some(*r) != current).collect();
    known.extend(ids.iter().filter(|i| !recent.contains(i) && Some(*i) != current));
    if back { known.last() } else { known.first() }.map(|s| (*s).clone())
}

/// The home "Canvas" space holds root terminals; it has no frame on the canvas.
fn root_space(s: &model::Space) -> bool {
    s.name == "Canvas" && s.folder == home() && s.machine.is_none()
}

/// Files the panel shows through the inert markdown renderer: markdown, and JSONL
/// (an agent transcript reads as a conversation).
fn rendered(path: &str) -> bool {
    std::path::Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("jsonl"))
}

fn preview_source(path: &str) -> String {
    match canvas::read_preview(std::path::Path::new(path)) {
        Ok(text) if std::path::Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("jsonl")) => {
            numbered_code(&crate::subs::transcript_markdown(&text, path).unwrap_or(text))
        }
        Ok(text)
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md")) =>
        {
            text
        }
        Ok(text) => numbered_code(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if std::path::Path::new(path).extension().is_some_and(|e| e == "jsonl") { "Waiting for transcript".into() }
            else { "File deleted".into() }
        },
        Err(_) => "Can't preview file".into(),
    }
}

fn numbered_code(text: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(i, line)| format!("    {:>4}  {line}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod preview_tests {
    #[test]
    fn transcripts_wait_for_the_file_and_keep_agent_markup_literal() {
        let path = std::env::temp_dir().join(format!("canopy-transcript-preview-{}.jsonl", std::process::id()));
        let path = path.to_str().unwrap();
        assert_eq!(super::preview_source(path), "Waiting for transcript");
        std::fs::write(path, serde_json::json!({"type":"assistant","message":{"content":"![image](https://example.com)\n<script>run()</script>"}}).to_string()).unwrap();
        let text = super::preview_source(path);
        assert!(text.contains("![image](https://example.com)"));
        assert!(text.lines().all(|line| line.starts_with("    ")));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn numbered_code_keeps_markdown_literal() {
        assert_eq!(
            super::numbered_code("```\n![image](https://example.com)\n\nend"),
            "       1  ```\n       2  ![image](https://example.com)\n       3  \n       4  end"
        );
        assert_eq!(super::numbered_code(""), "");
    }
}

fn format_quote(quote: &Value, cwd: Option<&str>) -> String {
    if quote["kind"] == "browser" { return browser::format_quote(quote, cwd); }
    let path = std::path::Path::new(quote["path"].as_str().unwrap_or_default());
    let display = cwd.and_then(|cwd| path.strip_prefix(cwd).ok()).unwrap_or(path);
    let mut header = format!("@{}", display.display());
    if let Some(body) = viewer_quote(quote, &header) { return with_message(quote, body); }
    if let Some(start) = quote["start"].as_u64() {
        header.push_str(&format!(":{start}"));
        if let Some(end) = quote["end"].as_u64().filter(|end| *end != start) { header.push_str(&format!("-{end}")); }
    }
    let text = quote["text"].as_str().unwrap_or_default().trim_end_matches(['\r', '\n']);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    // An image (a sim pane's annotation) is the file and what it shows, not a code block.
    let body = if ["png", "jpg", "jpeg"].iter().any(|e| ext.eq_ignore_ascii_case(e)) {
        format!("{header}\n{text}")
    } else if ext.eq_ignore_ascii_case("md") {
        format!("{header}\n{}", text.split('\n').map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n"))
    } else { format!("{header}\n```{ext}\n{text}\n```") };
    with_message(quote, body)
}

/// Appends the quote's optional message after a blank line.
fn with_message(quote: &Value, body: String) -> String {
    match quote["message"].as_str().map(str::trim).filter(|m| !m.is_empty()) {
        Some(message) => format!("{body}\n\n{message}"),
        None => body,
    }
}

/// The quote body for image and PDF panes (`kind` set), after the `@path` header.
fn viewer_quote(quote: &Value, header: &str) -> Option<String> {
    let text = quote["text"].as_str().unwrap_or_default().trim_end_matches(['\r', '\n']);
    match quote["kind"].as_str()? {
        "pdf" => Some(format!("{header} page {}\n{}", quote["page"].as_u64().unwrap_or_default(), text.split('\n').map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n"))),
        "image-region" => {
            let r = |i: usize| quote["region"][i].as_u64().unwrap_or_default();
            Some(format!("{header} region {},{} {}x{} {}", r(0), r(1), r(2), r(3), quote["crop"].as_str().unwrap_or_default()))
        }
        "image" => Some(header.to_owned()),
        _ => None,
    }
}

fn markdown_quote(path: &str, source: &str, text: &str) -> Value {
    let mut quote = json!({"path":path,"text":text});
    // shortcut: repeated rendered lines may match the wrong source lines, use source selection offsets when TextView exposes them.
    let lines: Vec<_> = source.lines().collect();
    let selected: Vec<_> = text.trim().lines().collect();
    if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
        if let Some(start) = lines.iter().position(|l| l.trim().contains(first.trim())) {
            if let Some(end) = lines[start..].iter().position(|l| l.trim().contains(last.trim())) {
                quote["start"] = json!(start + 1);
                quote["end"] = json!(start + end + 1);
            }
        }
    }
    quote
}

fn code_quote(path: &str, text: &str) -> Value {
    let mut start = None;
    let mut count = 0;
    let text = text.lines().enumerate().map(|(index, line)| {
        count = index + 1;
        let trimmed = line.trim_start();
        if let Some((number, code)) = trimmed.split_once("  ") {
            if let Ok(number) = number.parse::<usize>() {
                if number > index { start.get_or_insert(number - index); }
                return code;
            }
        }
        line
    }).collect::<Vec<_>>().join("\n");
    let end = start.map(|start| start + count.saturating_sub(1));
    json!({"path":path,"text":text,"start":start,"end":end})
}

#[cfg(test)]
mod quote_tests {
    use super::{code_quote, markdown_quote, format_quote};
    use serde_json::json;
    #[test]
    fn formats_viewer_quotes_with_their_source_location() {
        let pdf = json!({"kind":"pdf","path":"/work/a.pdf","page":2,"text":"first\nsecond"});
        assert_eq!(format_quote(&pdf, Some("/work")), "@a.pdf page 2\n> first\n> second");
        let mut region = json!({"kind":"image-region","path":"/work/a.png","region":[2,3,4,5],"crop":"/tmp/crop.png"});
        assert_eq!(format_quote(&region, Some("/work")), "@a.png region 2,3 4x5 /tmp/crop.png");
        region["message"] = json!("  explain this  ");
        assert_eq!(format_quote(&region, None), "@/work/a.png region 2,3 4x5 /tmp/crop.png\n\nexplain this");
        assert_eq!(format_quote(&json!({"kind":"image","path":"/work/a.png","text":"a.png"}), Some("/work")), "@a.png");
    }

    #[test]
    fn quote_sources_and_format() {
        assert_eq!(format_quote(&json!({"path":"/a.md","text":"a","pane":"p1"}), None), "@/a.md\n> a");
        let q = code_quote("/work/a.rs", "    12  fn main() {}\n    13    x");
        assert_eq!(q["text"], "fn main() {}\n  x");
        assert_eq!(format_quote(&q, Some("/work")), "@a.rs:12-13\n```rs\nfn main() {}\n  x\n```");
        let partial = code_quote("/work/a.rs", "main() {}\n    13    x");
        assert_eq!(partial["start"], 12);
        assert_eq!(partial["end"], 13);
        let q = markdown_quote("/work/a.md", "# Title\nhello\nworld", "hello\nworld");
        assert_eq!(format_quote(&q, None), "@/work/a.md:2-3\n> hello\n> world");
        let q = markdown_quote("/a.md", "hello", "missing");
        assert_eq!(format_quote(&q, None), "@/a.md\n> missing");
        assert_eq!(format_quote(&json!({"path":"/a.md","start":12,"end":12,"text":"a\n"}), None), "@/a.md:12\n> a");
        assert_eq!(format_quote(&json!({"path":"/a.md","text":"a","message":"  why?\n"}), None), "@/a.md\n> a\n\nwhy?");
        assert_eq!(format_quote(&json!({"path":"/a.md","text":"a","message":" "}), None), "@/a.md\n> a");
        assert_eq!(format_quote(&json!({"path":"/t/s.png","text":"Annotated screenshot","message":"fix this"}), None), "@/t/s.png\nAnnotated screenshot\n\nfix this");
    }
}

#[cfg(test)]
mod layout_answer_tests {
    use super::core_layout_answered;
    use crate::mirror_sync::Slot;
    #[test]
    fn space_cards_wait_for_the_cores_answer_even_when_it_has_no_layout() {
        assert!(!core_layout_answered(None));
        assert!(!core_layout_answered(Some(&Slot::default())), "a slow core start must not prune");
        let mut tried = Slot::default();
        tried.read_attempted = true;
        assert!(core_layout_answered(Some(&tried)), "an unavailable layout still lets the canvas sync");
    }
}

#[cfg(test)]
mod input_tests {
    use super::{AppWindow, WHEEL_GESTURE, edge_pan, wheel_claimed};
    use super::super::tests::{in_own_process, open_window, running_alone};
    use gpui::{Bounds, Entity, Modifiers, MouseButton, MouseDownEvent, PinchEvent, ScrollDelta,
        ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext, point, px, size};
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn a_pan_in_progress_keeps_the_wheel_until_it_goes_quiet() {
        let t = Instant::now();
        assert!(!wheel_claimed(false, None, t), "a fresh plain wheel belongs to the cell under it");
        assert!(wheel_claimed(true, None, t), "zoom always belongs to the canvas");
        assert!(wheel_claimed(false, Some(t), t + WHEEL_GESTURE / 2));
        assert!(!wheel_claimed(false, Some(t), t + WHEEL_GESTURE));
    }

    #[test]
    fn edge_pan_ramps_toward_each_canvas_edge() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(200.), px(200.)));
        let pan = |x, y| edge_pan(point(px(x), px(y)), bounds);
        assert_eq!(pan(110., 120.), [0., 0.]);
        assert_eq!(pan(42., 120.), [0., 0.]);
        assert_eq!(pan(26., 120.), [450., 0.]);
        assert_eq!(pan(209., 120.), [-871.875, 0.]);
        assert_eq!(pan(110., 20.), [0., 900.]);
        assert_eq!(pan(110., 204.), [0., -450.]);
        assert_eq!(pan(9., 120.), [0., 0.]);
        let [x, y] = pan(10., 20.);
        assert!((x.hypot(y) - 900.).abs() < 0.001);
    }

    const WHEEL: &str = "window::canvas_view::input_tests::wheel_over_panes";
    const RESIZE: &str = "window::canvas_view::input_tests::resize_over_panes";

    #[test]
    fn canvas_gestures_respect_the_entered_terminal() { in_own_process(WHEEL); }

    #[test]
    fn pane_edges_can_shrink_and_drag_past_the_view() { in_own_process(RESIZE); }

    fn panes(cx: &mut TestAppContext) -> (Entity<AppWindow>, &mut VisualTestContext) {
        let (view, cx) = open_window(cx, "canvas-input", "", json!({"id":"root","type":"canvas"}));
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| {
            this.model.add_space("root", "Canvas", &super::super::home(), crate::model::Tab::new("tab1", "", "pane1", "t1"));
            this.live.insert("t1".into(), json!({"state":"live", "pid":1}));
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{"size":[400,250]},"rect":[100,120,400,250],"z":1},
                {"id":"p1","kind":"image","props":{"path":"/missing.png"},"rect":[560,350,300,180],"z":0}
            ])).unwrap();
            this.canvas_state.mode = true;
            this.sync(window, cx);
            cx.notify();
        });
        cx.run_until_parked();
        (view, cx)
    }

    #[gpui::test]
    fn wheel_over_panes(cx: &mut TestAppContext) {
        if !running_alone(WHEEL) { return; }
        let (view, cx) = panes(cx);
        let origin = view.update(cx, |this, _| this.bounds.borrow()["canopy-canvas"].origin);
        let at = |x, y| origin + point(px(x), px(y));
        let position = at(200., 220.);
        let camera = |cx: &mut VisualTestContext| view.read_with(cx, |this, _| this.canvas_state.doc.camera);
        let none = Modifiers::default();
        let wheel = |cx: &mut VisualTestContext, position, y, touch_phase, modifiers| {
            cx.simulate_event(ScrollWheelEvent { position, delta: ScrollDelta::Pixels(point(px(0.), px(y))), modifiers, touch_phase });
        };
        let before = camera(cx);
        wheel(cx, at(520., 220.), 20., TouchPhase::Moved, none);
        assert_eq!(camera(cx).y, before.y - 20., "empty canvas pans once");
        wheel(cx, at(520., 220.), 0., TouchPhase::Ended, none);
        wheel(cx, position, 20., TouchPhase::Moved, none);
        assert_eq!(camera(cx).y, before.y - 40., "unentered terminal pans once");
        wheel(cx, position, 0., TouchPhase::Ended, none);

        cx.simulate_event(MouseDownEvent { position, modifiers: none, button: MouseButton::Left, click_count: 2, first_mouse: false });
        cx.simulate_mouse_up(position, MouseButton::Left, none);
        assert!(cx.update(|window, cx| view.read(cx).focused_cell().unwrap().focus.is_focused(window)));
        let before = camera(cx);
        wheel(cx, position, 30., TouchPhase::Moved, none);
        assert_eq!(camera(cx), before, "an entered terminal keeps plain scroll");

        let zoom = Modifiers { platform: cfg!(target_os = "macos"), control: !cfg!(target_os = "macos"), ..none };
        wheel(cx, position, 1000., TouchPhase::Moved, zoom);
        assert!(camera(cx).zoom > before.zoom);
        wheel(cx, position, 0., TouchPhase::Ended, none);
        assert!(view.update(cx, |this, _| this.canvas_state.motion.target.is_some()), "a zero-delta end starts settling at the zoom limit");

        let before = camera(cx);
        cx.simulate_event(PinchEvent { position, delta: -0.1, modifiers: none, phase: TouchPhase::Moved });
        assert!(camera(cx).zoom < before.zoom, "pinch reaches the canvas through an occluding pane");

        let notch = ScrollWheelEvent { position, delta: ScrollDelta::Lines(point(0., 50.)), modifiers: zoom, touch_phase: TouchPhase::Moved };
        cx.simulate_event(notch.clone());
        cx.executor().advance_clock(WHEEL_GESTURE / 2);
        cx.simulate_event(notch);
        cx.executor().advance_clock(WHEEL_GESTURE / 2);
        cx.run_until_parked();
        assert!(view.read_with(cx, |this, _| this.canvas_state.motion.target.is_none()), "new wheel input postpones settling");
        cx.executor().advance_clock(WHEEL_GESTURE / 2);
        cx.run_until_parked();
        assert!(view.read_with(cx, |this, _| this.canvas_state.motion.target.is_some()), "wheel input without an end settles after a quiet period");
    }

    #[gpui::test]
    fn resize_over_panes(cx: &mut TestAppContext) {
        if !running_alone(RESIZE) { return; }
        let (view, cx) = panes(cx);
        let bounds = view.update(cx, |this, _| this.bounds.borrow()["canopy-canvas"]);
        let at = |x, y| bounds.origin + point(px(x), px(y));
        let none = Modifiers::default();
        let left = MouseButton::Left;
        let rect = |cx: &mut VisualTestContext, id| view.read_with(cx, |this, _| this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap().rect);

        cx.simulate_mouse_down(at(104., 188.), left, none);
        cx.simulate_mouse_move(at(150., 188.), left, none);
        cx.simulate_mouse_up(at(150., 188.), left, none);
        assert_eq!(rect(cx, "t1"), [146., 120., 354., 250.]);

        cx.simulate_mouse_down(at(564., 430.), left, none);
        cx.simulate_mouse_move(at(600., 430.), left, none);
        cx.simulate_mouse_up(at(600., 430.), left, none);
        assert_eq!(rect(cx, "p1"), [596., 350., 264., 180.]);

        let position = point(bounds.left() + px(4.), bounds.top() + px(188.));
        cx.simulate_mouse_down(at(150., 188.), left, none);
        cx.simulate_mouse_move(position, left, none);
        let before = rect(cx, "t1");
        cx.update(|window, cx| window.simulate_next_frame(cx));
        let camera_x = view.read_with(cx, |this, _| this.canvas_state.doc.camera.x);
        assert!(camera_x < 0.);
        let after = rect(cx, "t1");
        assert!((after[0] - before[0] - camera_x).abs() < 0.001, "the resized edge follows the pointer as the canvas pans");
        cx.simulate_mouse_up(position, left, none);
        cx.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(view.read_with(cx, |this, _| this.canvas_state.doc.camera.x), camera_x, "release stops edge scrolling");
    }
}

#[cfg(test)]
mod pill_label_tests {
    use super::pill_label;
    use serde_json::json;
    #[test]
    fn pills_say_what_runs() {
        let claude = |title: &str| json!({"status":"idle","cwd":"/w/canopy","title":title,
            "program":{"key":"claude","name":"Claude Code","agent":true,"shell":false}});
        assert_eq!(pill_label(None, &claude("")), "Claude Code");
        assert_eq!(pill_label(None, &claude("Claude Code")), "Claude Code");
        assert_eq!(pill_label(None, &claude("~/w/canopy")), "Claude Code");
        assert_eq!(pill_label(None, &claude("canopy")), "Claude Code");
        // The agent's own title never names the cell.
        assert_eq!(pill_label(None, &claude("Fix pill labels")), "Claude Code");
        // Before the agent reports: no status, no agent flag, argv full of flags.
        let early = json!({"foreground":{"argv":["claude","--settings","{\"hooks\":{}}"]},"program":{"key":"claude","name":"Claude Code"}});
        assert_eq!(pill_label(None, &early), "Claude Code");
        assert_eq!(pill_label(Some("review"), &claude("Fix pill labels")), "review");
        assert_eq!(pill_label(Some("claude"), &claude("")), "Claude Code");
        let shell = json!({"foreground":{"argv":["zsh"]},"program":{"key":"shell","name":"zsh","shell":true}});
        assert_eq!(pill_label(Some(""), &shell), "terminal");
        let uv = json!({"foreground":{"argv":["uv","run","main.py"]},"program":{"key":"uv","name":"uv","shell":false}});
        assert_eq!(pill_label(None, &uv), "uv");
        let wrapped = json!({"foreground":{"argv":["/bin/zsh","-lic","\"$@\"; exec \"$0\" -li","/bin/zsh","claude"]},"program":{"key":"zsh","name":"zsh","shell":false}});
        assert_eq!(pill_label(None, &wrapped), "claude");
    }
}

#[cfg(test)]
mod spacing_membership_tests {
    use super::{cell_space, model};
    use std::collections::HashMap;
    use serde_json::json;
    #[test]
    fn children_inherit_spaces_without_looping_on_bad_parent_edges() {
        let mut model = model::Model::default();
        model.spaces.push(model::Space { id:"s".into(),name:"work".into(),folder:"/tmp".into(),machine:None,
            tabs:vec![model::Tab::new("tab","parent","pane","parent")],active:None,rest:Default::default() });
        let live = HashMap::from([
            ("child".into(),json!({"parent":"parent"})),
            ("grandchild".into(),json!({"parent":"child"})),
            ("cycle".into(),json!({"parent":"cycle"})),
        ]);
        assert_eq!(cell_space(&model,&live,"grandchild").unwrap().id,"s");
        assert!(cell_space(&model,&live,"cycle").is_none());
        assert!(cell_space(&model,&live,"missing").is_none());
    }
}

#[cfg(test)]
mod settings_tests {
    use super::{canvas, settings, point, px, TestAppContext, json};
    use super::super::tests::{in_own_process, open_window, running_alone};
    const TEST: &str = "window::canvas_view::settings_tests::drop_layout";

    #[test]
    fn drop_waits_for_release() { in_own_process(TEST); }

    #[test]
    fn pane_moves_follow_frames_and_flush_on_release() {
        in_own_process("window::canvas_view::settings_tests::queued_moves");
    }

    #[gpui::test]
    fn queued_moves(cx: &mut TestAppContext) {
        if !running_alone("window::canvas_view::settings_tests::queued_moves") { return; }
        let (view, cx) = open_window(cx, "probe", "", json!({"id":"root","type":"row","children":[]}));
        let x = |doc: &crate::canvas::Document| [doc.panes[0].rect[0], doc.panes[1].rect[0]];
        let solves = view.update_in(cx, |this, window, cx| {
            this.person.canvas_push = settings::CanvasPush::Live;
            this.canvas_state.doc.camera.zoom = 1.;
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{"size":[200,100]},"rect":[0,0,200,100],"z":0},
                {"id":"t2","kind":"terminal","props":{"size":[200,100]},"rect":[300,0,200,100],"z":1},
                {"id":"t3","kind":"terminal","props":{"size":[200,100]},"rect":[520,0,200,100],"z":2}
            ])).unwrap();
            this.canvas_state.selected = vec!["t1".into(), "t2".into()];
            this.begin_layout();
            this.canvas_state.drag = Some((Some("t1".into()), point(px(0.), px(0.))));
            let solves = this.canvas_state.layout_solves;
            for at in [90., 230., 200.] {
                assert!(this.queue_canvas_move(point(px(at), px(0.)), window, cx));
            }
            assert_eq!(x(&this.canvas_state.doc), [0., 300.]);
            assert_eq!(this.canvas_state.layout_solves, solves);
            solves
        });

        cx.update(|window, cx| window.simulate_next_frame(cx));
        view.update_in(cx, |this, window, cx| {
            assert_eq!(this.canvas_state.layout_solves, solves + 1);
            assert_eq!(x(&this.canvas_state.doc), [200., 500.]);
            assert_eq!(x(this.canvas_state.shown.as_ref().unwrap()), [200., 500.]);
            assert_ne!(this.canvas_state.doc.panes[2].rect, [520., 0., 200., 100.]);

            assert!(this.queue_canvas_move(point(px(245.), px(0.)), window, cx));
            this.end_canvas_drag(cx);
            assert_eq!(x(&this.canvas_state.doc), [245., 545.]);
            assert_eq!(this.canvas_state.layout_solves, solves + 2);
        });

        cx.update(|window, cx| window.simulate_next_frame(cx));
        view.update_in(cx, |this, window, cx| {
            assert_eq!(x(&this.canvas_state.doc), [245., 545.]);
            assert_eq!(this.canvas_state.layout_solves, solves + 2);

            this.canvas_state.doc.camera.zoom = 2.;
            this.canvas_state.doc.panes[2].pinned = Some([10., 20., 240., 120.]);
            this.begin_layout();
            this.canvas_state.preview_drag = Some(("t3".into(), None, point(px(0.), px(0.))));
            assert!(this.queue_canvas_move(point(px(30.), px(15.)), window, cx));
            this.end_canvas_drag(cx);
            assert_eq!(this.canvas_state.doc.panes[2].pinned, Some([40., 35., 240., 120.]));
            assert_eq!(this.canvas_state.layout_solves, solves + 2);
        });
    }

    #[gpui::test]
    fn drop_layout(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "probe", "", json!({"id":"root","type":"row","children":[]}));
        view.update_in(cx, |this, window, cx| {
            this.person.canvas_push = settings::CanvasPush::Drop;
            for resize in [false, true] {
                this.canvas_state.doc.panes = serde_json::from_value(json!([
                    {"id":"t1","kind":"terminal","props":{"size":[200,100]},"rect":[0,0,200,100],"z":0},
                    {"id":"t2","kind":"terminal","props":{"size":[200,100]},"rect":[400,0,200,100],"z":1}
                ])).unwrap();
                this.begin_layout();
                if resize {
                    this.canvas_state.cell_resize = Some(("t1".into(), point(px(0.),px(0.))));
                    *this.canvas_state.doc.panes[0].size_mut().unwrap() = Some([500.,100.]);
                } else {
                    this.canvas_state.drag = Some((Some("t1".into()), point(px(0.),px(0.))));
                    this.canvas_state.doc.panes[0].rect[0] = 400.;
                }
                let neighbour = this.canvas_state.doc.panes[1].rect;
                this.solve_layout(Some("t1"), None, cx);
                assert_eq!(this.canvas_state.doc.panes[1].rect, neighbour);
                this.end_canvas_drag(cx);
                assert_ne!(this.canvas_state.doc.panes[1].rect, neighbour);
                assert!(this.canvas_state.pending_layout.is_none());
            }
            // A click expansion has no active drag, so release solves immediately.
            *this.canvas_state.doc.panes[0].size_mut().unwrap() = None;
            this.live.insert("t1".into(), json!({}));
            this.person.terminal_size = [140,45];
            this.set_expanded("t1", true, window, cx);
            let body = crate::terminal::size_of_grid(140,45,window);
            assert_eq!(this.canvas_state.doc.panes[0].size(), Some([f32::from(body.width)+2., f32::from(body.height)+canvas::HEADER+2.]));
            assert!(this.canvas_state.pending_layout.is_none());
        });
    }
}

#[cfg(test)]
mod tile_tests {
    use super::{TestAppContext, json};
    use super::super::tests::{in_own_process, open_window, running_alone};
    const TEST: &str = "window::canvas_view::tile_tests::tile_toggles";

    #[test]
    fn tile_view_fills_the_view_and_puts_everything_back() { in_own_process(TEST); }

    #[gpui::test]
    fn tile_toggles(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "probe", "", json!({"id":"root","type":"row","children":[]}));
        view.update_in(cx, |this, window, cx| {
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{"size":[600,400]},"rect":[0,0,600,400],"z":0},
                {"id":"t2","kind":"terminal","props":{"size":[600,400]},"rect":[700,0,600,400],"z":1},
                {"id":"t3","kind":"terminal","props":{"size":[600,400]},"rect":[0,500,600,400],"z":2},
                {"id":"t4","kind":"terminal","props":{},"rect":[2000,2000,220,36],"z":3}
            ])).unwrap();
            for t in ["t1", "t2", "t3", "t4"] { this.live.insert(t.into(), json!({})); }
            let before: Vec<_> = this.canvas_state.doc.panes.iter().map(|c| (c.rect, c.size())).collect();
            this.person.tile_focus_grows = true;
            this.canvas_state.doc.camera = crate::canvas::Camera { x: 300., y: -40., zoom: 0.5 };
            this.canvas_state.selected = vec!["t2".into()];
            this.tile_canvas(true, window, cx);
            this.canvas_state.settle_for_test();
            let cards = &this.canvas_state.doc.panes;
            let area = |i: usize| cards[i].rect[2] * cards[i].rect[3];
            assert!(this.canvas_state.tile.is_some());
            assert!(area(1) > area(0) && area(1) > area(2), "the selected tile is the largest: {:?}", cards.iter().map(|c| c.rect).collect::<Vec<_>>());
            assert!(cards.iter().take(3).all(|c| c.size() == Some([c.rect[2], c.rect[3]])), "tiles stay expanded at their tile size");
            assert!(cards.iter().take(3).all(|c| c.rect[1] + c.rect[3] <= cards[3].rect[1]), "the pill lines up below the tiles");
            // Selecting another tile makes it the large one.
            this.canvas_state.selected = vec!["t1".into()];
            this.tile_canvas(false, window, cx);
            this.canvas_state.settle_for_test();
            let cards = &this.canvas_state.doc.panes;
            assert!(cards[0].rect[2] * cards[0].rect[3] > cards[1].rect[2] * cards[1].rect[3]);
            assert!(cards[0].rect[0] < cards[1].rect[0] && cards[0].rect[1] < cards[2].rect[1], "the active tile grows in its own slot");
            // Jittered rects (a click that nudged tiles) do not reorder them.
            let this_cards = &mut this.canvas_state.doc.panes;
            this_cards[0].rect[1] += 30.;
            this_cards[1].rect[1] -= 30.;
            this.tile_canvas(false, window, cx);
            this.canvas_state.settle_for_test();
            let cards = &this.canvas_state.doc.panes;
            assert!(cards[0].rect[0] < cards[1].rect[0] && cards[0].rect[1] == cards[1].rect[1], "same slots after jitter");
            // Off: everything is back where it was.
            this.tile_canvas(true, window, cx);
            this.canvas_state.settle_for_test();
            assert!(this.canvas_state.tile.is_none());
            assert_eq!(this.canvas_state.motion_target_for_test(), crate::canvas::Camera { x: 300., y: -40., zoom: 0.5 }, "the camera goes back too");
            let after: Vec<_> = this.canvas_state.doc.panes.iter().map(|c| (c.rect, c.size())).collect();
            assert_eq!(after, before);
            // Two selected cells tile alone: the others stay put, under them, and everything returns on exit.
            this.canvas_state.selected = vec!["t1".into(), "t3".into()];
            this.tile_canvas(true, window, cx);
            this.canvas_state.settle_for_test();
            let cards = &this.canvas_state.doc.panes;
            assert_eq!((cards[1].rect, cards[3].rect), (before[1].0, before[3].0), "non-members keep their place");
            assert!(cards[0].rect != before[0].0 && cards[2].rect != before[2].0, "members tile");
            assert!(cards[0].z > cards[1].z && cards[2].z > cards[1].z, "members draw above the rest");
            this.tile_canvas(true, window, cx);
            this.canvas_state.settle_for_test();
            let after: Vec<_> = this.canvas_state.doc.panes.iter().map(|c| (c.rect, c.size())).collect();
            assert_eq!(after, before);
        });
    }
}

#[cfg(test)]
mod bar_tests {
    use super::bar_inset;

    #[test]
    fn bar_leaves_room_for_traffic_lights_only_on_macos_outside_full_screen() {
        assert_eq!(bar_inset(true, false), 76.);
        assert_eq!(bar_inset(true, true), 12.);
        assert_eq!(bar_inset(false, false), 12.);
    }
}

#[cfg(test)]
mod like_tests {
    use super::like_run;
    use serde_json::json;
    #[test]
    fn like_resolves_to_the_programs_run_or_nothing() {
        let startable = [json!({"key":"uv","run":["uv","run","python"]})];
        assert_eq!(like_run(&json!({"program":{"key":"uv"}}), &startable), Some(vec!["uv".into(), "run".into(), "python".into()]));
        assert_eq!(like_run(&json!({"program":{"key":"shell"}}), &startable), None);
        assert_eq!(like_run(&json!({}), &startable), None);
    }
}

#[cfg(test)]
mod recent_space_tests {
    use super::recent_space;

    #[test]
    fn ctrl_tab_toggles_the_last_two_and_falls_back_to_tab_order() {
        let ids: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        let recent: Vec<String> = ["b", "a"].map(String::from).to_vec();
        assert_eq!(recent_space(&recent, &ids, Some(&"b".into()), false).as_deref(), Some("a"));
        assert_eq!(recent_space(&recent, &ids, Some(&"b".into()), true).as_deref(), Some("c"));
        assert_eq!(recent_space(&[], &ids, None, false).as_deref(), Some("a"));
        assert_eq!(recent_space(&["gone".into()], &ids[..1], Some(&"a".into()), false), None);
    }
}

#[cfg(test)]
mod pane_action_tests {
    use super::{TestAppContext, json};
    use super::super::tests::{in_own_process, open_window, running_alone};
    const TARGETS: &str = "window::canvas_view::pane_action_tests::targets_headless";

    #[test]
    fn pane_targets_stay_in_the_current_space() { in_own_process(TARGETS); }

    #[gpui::test]
    fn targets_headless(cx: &mut TestAppContext) {
        if !running_alone(TARGETS) { return; }
        let (view, cx) = open_window(cx, "pane-targets", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, window, _| {
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{},"rect":[0,0,220,36],"z":0},
                {"id":"t2","kind":"terminal","props":{},"space":"other","rect":[0,0,220,36],"z":1}
            ])).unwrap();
            this.canvas_state.selected = vec!["t1".into(), "t2".into(), "p1".into()];
            assert_eq!(this.targets(window), ["t1"]);
            this.canvas_state.doc.room = Some("other".into());
            assert_eq!(this.targets(window), ["t2"]);
            this.canvas_state.selected.clear();
            assert!(this.targets(window).is_empty());
        });
    }
    const CLOSE: &str = "window::canvas_view::pane_action_tests::close_headless";

    #[test]
    fn close_removes_every_selected_pane() { in_own_process(CLOSE); }

    #[gpui::test]
    fn close_headless(cx: &mut TestAppContext) {
        if !running_alone(CLOSE) { return; }
        let (view, cx) = open_window(cx, "pane-close", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, window, cx| {
            this.person.confirm_close_terminal = false;
            this.listed_once = true;
            this.documents.entry("".into()).or_default().read_attempted = true;
            for i in 1..=3 {
                let key = format!("t{i}");
                this.canvas_state.doc.place(&key, [i as f32 * 300., 0.], [220., 36.]);
                this.live.insert(key.clone(), json!({"address":key}));
                this.canvas_state.selected.push(key);
            }
            this.focus.focus(window, cx);
            this.run_key("canvas.close", json!({}), window, cx);
            assert_eq!(this.closing_terminals.len(), 3, "the shared close command must terminate every target");
            // Deliver the core listing after the requested terminations, without a real core.
            let remaining: Vec<_> = this.live.iter().filter(|(key, _)| !this.closing_terminals.contains_key(*key)).map(|(_, rec)| rec.clone()).collect();
            this.terminals_listed(json!({"terminals":remaining}), window, cx);
            assert!(this.canvas_state.doc.panes.is_empty(), "all three selected cards must be gone");
        });
    }

    const OVERLAP: &str = "window::canvas_view::pane_action_tests::overlap_headless";

    #[test]
    fn a_click_on_overlapping_panes_raises_only_the_front_one() { in_own_process(OVERLAP); }

    /// Real pointer events through hit testing: an expanded terminal over a boxed pane (the
    /// pane_card that sims, browsers and notes share) keeps the click where they overlap.
    #[gpui::test]
    fn overlap_headless(cx: &mut TestAppContext) {
        use gpui::{point, px, Modifiers};
        if !running_alone(OVERLAP) { return; }
        let (view, cx) = open_window(cx, "pane-overlap", "", json!({"id":"root","type":"canvas"}));
        cx.run_until_parked();
        cx.update(|window, cx| view.update(cx, |this, cx| {
            this.model.add_space("root", "Canvas", &super::super::home(), crate::model::Tab::new("tab1", "", "pane1", "t1"));
            this.live.insert("t1".into(), json!({"state":"live", "pid":1}));
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"t1","kind":"terminal","props":{"size":[600,400]},"rect":[0,0,600,400],"z":2},
                {"id":"p1","kind":"image","props":{"path":"/missing.png"},"rect":[300,100,400,400],"z":1}
            ])).unwrap();
            this.sync(window, cx);
            cx.notify();
        }));
        cx.run_until_parked();
        let origin = view.update(cx, |this, _| this.bounds.borrow()["canopy-canvas"].origin);
        let at = |x: f32, y: f32| origin + point(px(x), px(y));
        let front = |cx: &mut gpui::VisualTestContext| view.update(cx, |this, _| this.canvas_state.doc.panes.iter().max_by_key(|p| p.z).unwrap().id.clone());
        let none = Modifiers::default();

        cx.simulate_click(at(450., 250.), none);
        cx.run_until_parked();
        assert_eq!(front(cx), "t1", "a click where the terminal covers the pane must not raise the pane behind");

        cx.simulate_click(at(650., 300.), none);
        cx.run_until_parked();
        assert_eq!(front(cx), "p1", "a click on the pane's own part raises it");

        cx.simulate_click(at(450., 250.), none);
        cx.run_until_parked();
        assert_eq!(front(cx), "p1", "now in front, the pane keeps the overlap");

        cx.simulate_click(at(100., 250.), none);
        cx.run_until_parked();
        assert_eq!(front(cx), "t1", "a click on the terminal's own part raises it again");
    }
}


#[cfg(test)]
mod hint_tests {
    #[test]
    fn tooltip_keys_prefer_canvas_then_layout_then_global() {
        let mut rows: Vec<_> = [("mod:test", "q"), ("global", "cmd-x"), ("layout:main", "cmd-w"), ("canvas-mode", "x")]
            .into_iter().map(|(section, key)| (section.into(), key.into(), "canvas.close".into())).collect();
        for expected in ["x", "cmd-w", "cmd-x", "q"] {
            assert_eq!(super::action_key(&rows, "canvas.close"), Some(crate::keys::pretty(expected)));
            rows.pop();
        }
        assert_eq!(super::action_key(&rows, "canvas.close"), None);
    }
}
