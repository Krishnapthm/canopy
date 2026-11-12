// Modified for Canopy.
use super::*;
use crate::canvas::{self, Document};
use gpui::AnimationExt as _;

pub(super) struct State {
    pub doc: Document,
    previews: HashMap<String, (Entity<gpui_base::TextViewState>, Task<()>)>,
    preview_drag: Option<(String, bool, Point<Pixels>)>,
    pub panel: Option<String>,
    pub file: Option<String>,
    pub tabs: Vec<String>,
    /// The folder button opened the panel for the file tree, with no tab needed.
    pub files: bool,
    pub popped: HashMap<String, WindowHandle<PopOut>>,
    save: canvas::SaveSchedule,
    pub space_down: bool,
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
    rename: Option<(String, Entity<gpui_base::input::InputState>, Subscription)>,
    /// The preview text a quote popover was opened from, kept highlighted, and that popover's id.
    quote_mark: Option<(Entity<gpui_base::TextViewState>, String)>,
    /// The panel's terminal, file and tabs when it was closed, for the folder button to bring back.
    closed: Option<(Option<String>, Option<String>, Vec<String>)>,
}
#[derive(Default)]
struct Motion {
    target: Option<(canvas::Camera, canvas::Camera, std::time::Instant, canvas::Point)>,
    velocity: [f32; 2],
    sample: Option<std::time::Instant>,
    frame: bool,
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
            previews: HashMap::new(),
            preview_drag: None,
            panel: None,
            file: None,
            tabs: Vec::new(),
            files: false,
            popped: HashMap::new(),
            save: canvas::SaveSchedule::default(),
            space_down: false,
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
            closed: None,
        }
    }
    /// Ends any pan, card, space or panel-resize drag.
    pub(super) fn end_drag(&mut self) {
        self.preview_drag = None;
        self.drag = None;
        self.space_drag = None;
        self.resizing = false;
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
    /// The folder of the space last clicked on the canvas.
    fn panel_root(&self) -> Option<String> {
        let space = self.canvas_state.doc.panel.space.as_deref()?;
        self.model.space(space).map(|s| s.folder.clone())
    }
    /// The topmost space under a window position, if any.
    fn space_at(&self, position: Point<Pixels>) -> Option<String> {
        let origin = self.bounds.borrow().get("canopy-canvas").map(|b| b.origin).unwrap_or_default();
        let p = self.canvas_state.doc.camera.to_world([
            f32::from(position.x - origin.x),
            f32::from(position.y - origin.y),
        ]);
        canvas::space_at(&self.canvas_state.doc.spaces, p).map(|s| s.space.clone())
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
    pub(super) fn panel_mods(&self) -> Vec<(String, Instance)> {
        if !self.has_canvas()
            || (self.canvas_state.panel.is_none() && self.canvas_state.file.is_none() && !self.canvas_state.files)
        {
            return vec![];
        }
        let root = self.panel_root();
        let mut slots = vec![];
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
            if !std::path::Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                slots.push(("canvas-file".into(), Instance { name: "files".into(), props: json!({"canvas":"viewer","space":self.canvas_state.doc.panel.space,"root":std::path::Path::new(path).parent(),"file":std::path::Path::new(path).file_name().map(|p|p.to_string_lossy())}) }));
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
            None => {
                let size = self
                    .bounds
                    .borrow()
                    .get("canopy-canvas")
                    .map(|b| b.size)
                    .unwrap_or(window.viewport_size());
                self.canvas_state
                    .doc
                    .camera
                    .to_world([f32::from(size.width) / 2., f32::from(size.height) / 2.])
            }
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
            self.canvas_state.file = Some(path);
            self.focus.focus(window, cx);
            self.sync(window, cx);
            cx.notify();
            return;
        }
        let path = std::path::Path::new(path);
        if canvas::read_preview(path).is_err() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            self.open_overlay(
                mods::new_overlay_id(),
                json!({"kind":"toast","notification":{"title":format!("Can't preview {name}")}}),
                window,
                cx,
            );
            return;
        }
        let Ok(path) = path.canonicalize() else {
            return;
        };
        let path = path.to_string_lossy().into_owned();
        if let Some(card) = self
            .canvas_state
            .doc
            .previews
            .iter_mut()
            .find(|p| p.path == path)
        {
            card.rect[0] = point[0];
            card.rect[1] = point[1];
        } else {
            self.canvas_state.doc.previews.push(canvas::Preview {
                path,
                rect: [point[0], point[1], 560., 420.],
            });
        }
        self.load_previews(cx);
        self.canvas_changed(cx);
        cx.notify();
    }
    fn load_previews(&mut self, cx: &mut Context<Self>) {
        let mut cards = self.canvas_state.doc.previews.clone();
        if let Some(path) = &self.canvas_state.file {
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                cards.push(canvas::Preview {
                    path: path.clone(),
                    rect: [0., 0., 0., 0.],
                });
            }
        }
        self.canvas_state
            .previews
            .retain(|path, _| cards.iter().any(|c| &c.path == path));
        for card in cards {
            if self.canvas_state.previews.contains_key(&card.path) {
                continue;
            }
            let text = preview_source(&card.path);
            let view = cx.new(|cx| gpui_base::TextViewState::markdown(&text, cx));
            let path = card.path.clone();
            let folder = std::path::Path::new(&path)
                .parent()
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
                        .any(|p| p == &path || std::path::Path::new(&path).starts_with(p))
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
            self.canvas_state.previews.insert(card.path, (view, task));
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
                self.canvas_state.panel = Some(addr);
                self.sync(window, cx);
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
                    if this.live.contains_key(&closed_addr) {
                        this.canvas_state.panel = Some(closed_addr.clone());
                    }
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
                self.canvas_state.popped.insert(addr, handle);
                self.canvas_state.panel = None;
                self.sync(window, cx);
            }
            Err(error) => self.fail(error.to_string()),
        }
    }

    pub(super) fn rename_canvas_space(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(space) = self.model.space(&id) else { return };
        let value = space.name.clone();
        let input = cx.new(|cx| {
            let mut input = gpui_base::input::InputState::new(window, cx);
            input.set_value(value, window, cx);
            input.focus(window, cx);
            input.select_all(window, cx);
            input
        });
        let sub = cx.subscribe_in(&input, window, |this, input, event, window, cx| {
            if matches!(event, gpui_base::input::InputEvent::PressEnter { .. } | gpui_base::input::InputEvent::Blur) {
                if let Some((id, _, _)) = this.canvas_state.rename.take() {
                    let name = input.read(cx).value().to_string();
                    this.dispatch("space.rename", &json!({"id":id,"name":name}), true, window, cx);
                    cx.notify();
                }
            }
        });
        self.canvas_state.rename = Some((id, input, sub));
        cx.notify();
    }
    fn animate_camera(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                    if !active { state.motion.target = None; }
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
        let from = self.canvas_state.doc.camera;
        let mut to = from;
        to.zoom_at(from.zoom.clamp(canvas::ZOOMS[0], canvas::ZOOMS[11]), anchor);
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
            self.canvas_state.doc.spaces.push(canvas::Space { space: id, rect: [point[0], point[1], 960., 640.], color: color.into(), frame: true });
            self.canvas_changed(cx);
            self.flush_documents(false, window, cx);
            cx.notify();
            return;
        }
        if args.get("folder").is_some() { return self.fail("folder is only for kind space"); }
        // No space: the one under the point, so scripted creates land where they are placed.
        let space = Self::owned(&args, "space").or_else(|| {
            canvas::space_at(&self.canvas_state.doc.spaces, point).map(|s| s.space.clone()).filter(|id| self.model.space(id).is_some())
        });
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
                    if let Err(error) = self.place_canvas_card(addr, point, cx) {
                        self.fail(error);
                    }
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub(super) fn canvas_changed(&mut self, cx: &mut Context<Self>) {
        self.canvas_state.save.dirty = true;
        self.canvas_state.save_task = None;
        self.canvas_state.save.pending = false;
        // Debounced: each change restarts the 500 ms timer. Camera animation saves once from its last frame.
        if self.canvas_state.motion.target.is_some() || self.canvas_state.motion.frame { return; }
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
    fn pill_rect(&self, card: &canvas::Card, cx: &App) -> canvas::Rect {
        let rec = self.live.get(&card.terminal).unwrap_or(&Value::Null);
        // Room taken by the trailing status and pop-out glyphs (12 px each plus the gap).
        let trailing = 20 * (canvas_agent(rec) as u8 + self.canvas_state.popped.contains_key(&card.terminal) as u8);
        let text = self.pill_name(&card.terminal).replace(['\n', '\r'], " ");
        let family = settings::get().ui_font_family.to_string();
        let mut sizes = self.canvas_state.pill_sizes.borrow_mut();
        if let Some((label, cached_font, cached_trailing, size)) = sizes.get(&card.terminal)
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
        sizes.insert(card.terminal.clone(), (text, family, trailing, size));
        [card.rect[0], card.rect[1], size[0], size[1]]
    }
    /// Menus can place a newly created terminal without duplicating membership or geometry rules.
    pub(super) fn place_canvas_card(
        &mut self,
        terminal: &str,
        point: canvas::Point,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if !point.iter().all(|v| v.is_finite()) {
            return Err("invalid canvas point".into());
        }
        if !self.live.contains_key(terminal) && self.model.pane_of(terminal).is_none() {
            return Err("no running terminal".into());
        }
        let card = self
            .canvas_state
            .doc
            .cards
            .iter()
            .find(|c| c.terminal == terminal)
            .cloned()
            .unwrap_or(canvas::Card {
                terminal: terminal.into(),
                rect: [point[0], point[1], 240., 36.],
                z: 0,
            });
        let mut r = self.pill_rect(&card, cx);
        r[0] = point[0];
        r[1] = point[1];
        let space = self
            .model
            .pane_of(terminal)
            .and_then(|p| self.model.space_of(&p))
            .and_then(|s| {
                self.canvas_state
                    .doc
                    .spaces
                    .iter()
                    .find(|g| g.space == s.id)
            });
        if let Some(space) = space {
            r = canvas::clamp(r, space.rect);
        }
        self.canvas_state
            .doc
            .place(terminal, [r[0], r[1]], [card.rect[2], card.rect[3]]);
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
        if !self.has_canvas() || !self.listed_once {
            return;
        }
        self.load_previews(cx);
        let missing: Vec<_> = self
            .live
            .keys()
            .filter(|a| {
                self.model.pane_of(a).is_none()
                    && !self.changing_terminals.contains(*a)
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
            if let Err(error) = self.place_canvas_card(&addr, point, cx) {
                self.canvas_state.error = Some(error);
            }
        }
        let sizes: HashMap<_, _> = self
            .live
            .keys()
            .map(|a| {
                let r = self.pill_rect(
                    &canvas::Card {
                        terminal: a.clone(),
                        rect: [0., 0., 240., 36.],
                        z: 0,
                    },
                    cx,
                );
                (a.clone(), [r[2], r[3]])
            })
            .collect();
        let effective = |c: &canvas::Card| {
            let size = sizes.get(&c.terminal).copied().unwrap_or([240., 36.]);
            [c.rect[0], c.rect[1], size[0], size[1]]
        };
        if self.canvas_state.panel.as_ref().is_some_and(|a| {
            !self.live.contains_key(a)
                || !canvas::view_allows(
                    &self.canvas_state.doc.view,
                    self.live.get(a).is_some_and(canvas_agent),
                )
        }) {
            self.canvas_state.panel = None;
            self.focus.focus(window, cx);
        }
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
                    color: "blue".into(),
                    frame: true,
                });
                changed = true;
            }
        }
        let before = state.doc.cards.len();
        state.doc.cards.retain(|c| {
            self.live.contains_key(&c.terminal) || self.born.borrow().contains_key(&c.terminal)
        });
        changed |= before != state.doc.cards.len();
        let mut addresses: Vec<_> = self.live.keys().cloned().collect();
        addresses.sort_by_key(|a| (self.live[a]["created_ms"].as_u64().unwrap_or(0), a.clone()));
        // Parents are placed first even when a snapshot arrives out of order.
        for _ in 0..addresses.len().max(1) {
            let mut progress = false;
            for addr in &addresses {
                if state.doc.cards.iter().any(|c| &c.terminal == addr) {
                    continue;
                }
                let parent = self.live[addr]["parent"].as_str();
                if parent.is_some_and(|p| {
                    p != addr
                        && self.live.contains_key(p)
                        && !state.doc.cards.iter().any(|c| c.terminal == p)
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
                let area = space
                    .and_then(|s| state.doc.spaces.iter().find(|g| g.space == s.id))
                    .map(|s| s.rect)
                    .unwrap_or([0., 0., 960., 640.]);
                let occupied: Vec<_> = state.doc.cards.iter().map(effective).collect();
                let mut r = if let Some(p) =
                    parent.and_then(|p| state.doc.cards.iter().find(|c| c.terminal == p))
                {
                    let siblings: Vec<_> = state
                        .doc
                        .cards
                        .iter()
                        .filter(|c| {
                            self.live
                                .get(&c.terminal)
                                .and_then(|r| r["parent"].as_str())
                                == parent
                        })
                        .map(effective)
                        .collect();
                    canvas::child_position(effective(p), &siblings, sizes[addr])
                } else {
                    canvas::free_spot(area, &occupied, sizes[addr])
                };
                if let Some(geometry) =
                    space.and_then(|s| state.doc.spaces.iter_mut().find(|g| g.space == s.id))
                {
                    let minimum = canvas::min_size(geometry.rect, &[r]);
                    geometry.rect[2] = geometry.rect[2].max(minimum[0]);
                    geometry.rect[3] = geometry.rect[3].max(minimum[1]);
                    r = canvas::clamp(r, geometry.rect);
                }
                state.doc.place(addr, [r[0], r[1]], [r[2], r[3]]);
                progress = true;
                changed = true;
            }
            if !progress {
                break;
            }
        }
        if changed {
            self.canvas_changed(cx);
        }
    }
    pub(super) fn quote_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(path), Some(text)) = (a["path"].as_str(), a["text"].as_str()) else { return self.fail("quote needs path and text"); };
        if !std::path::Path::new(path).is_absolute() || text.is_empty() { return self.fail("quote needs an absolute path and non-empty text"); }
        if name == "quote.menu" {
            self.quote_menu(a, window, cx);
            return;
        }
        if a.get("argv").is_some() && a.get("terminal").is_none() {
            let mut args = json!({"argv":a["argv"],"space":self.canvas_state.doc.panel.space,"quote":a});
            if args["space"].is_null() { args.as_object_mut().unwrap().remove("space"); }
            self.create_canvas(args, window, cx);
            return;
        }
        let Some(addr) = a["terminal"].as_str().map(str::to_owned) else { return self.fail("quote.send needs terminal or argv"); };
        let quote = a.clone();
        cx.spawn_in(window, async move |this, cx| {
            let started = Instant::now();
            let mut opened = false;
            loop {
                let done = this.update_in(cx, |this, window, cx| {
                    if started.elapsed() >= Duration::from_secs(30) {
                        this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"Quote not sent: the agent did not get ready for a paste"}}), window, cx);
                        return true;
                    }
                    let Some(rec) = this.live.get(&addr).filter(|rec| canvas_agent(rec)) else { return false };
                    let text = format_quote(&quote, rec.pointer("/foreground/cwd").and_then(Value::as_str).or_else(|| rec["cwd"].as_str()));
                    // Open once: the person may close the panel or pick another pill while the agent starts.
                    if !opened {
                        opened = true;
                        if this.canvas_state.popped.contains_key(&addr) { this.pop_canvas(addr.clone(), true, window, cx); }
                        this.dispatch("panel.open", &json!({"terminal":addr}), true, window, cx);
                        this.sync(window, cx);
                    }
                    // Paste only into the target's own view, never whatever the panel shows now.
                    let Some(term) = this.terms.get("canvas-panel").filter(|t| t.address == addr) else { return false };
                    if !term.view.read(cx).paste_quote(&text) { return false; }
                    term.focus.focus(window, cx);
                    true
                });
                if !matches!(done, Ok(false)) { break; }
                cx.background_executor().timer(Duration::from_millis(100)).await;
            }
        }).detach();
    }

    /// Opens the quote popover: running agents as rows, the quoted file's space first.
    fn quote_menu(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> String {
        let path = std::path::Path::new(a["path"].as_str().unwrap_or_default());
        let home = self.model.spaces.iter().filter(|s| path.starts_with(&s.folder)).max_by_key(|s| s.folder.len()).map(|s| s.id.as_str());
        let mut rows: Vec<_> = self.live.iter().filter(|(_, rec)| canvas_agent(rec)).map(|(addr, rec)| {
            let name = self.pill_name(addr);
            let (icon, tint) = program_icon(rec);
            let elsewhere = self.model.pane_of(addr).and_then(|p| self.model.space_of(&p)).map(|s| s.id.as_str()) != home;
            ((elsewhere, name.clone(), addr.clone()), json!({"terminal":addr,"name":name,"icon":icon,"tint":tint}))
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

    /// A finished selection in a preview TextView opens the quote popover at the
    /// mouse-up point. The selection becomes a highlight, since any click (one on
    /// the popover too) clears window text selections.
    fn quote_selection(&mut self, path: &str, view: &Entity<gpui_base::TextViewState>, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let text = view.read(cx).selected_text();
        if text.trim().is_empty() { return; }
        let mut quote = if std::path::Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
            markdown_quote(path, &preview_source(path), &text)
        } else { code_quote(path, &text) };
        quote["point"] = json!({"x": f32::from(position.x), "y": f32::from(position.y)});
        let range = view.read(cx).selected_source_range().and_then(|r| view.read(cx).rendered_text().range_for_source(r));
        let id = self.quote_menu(&quote, window, cx);
        self.clear_quote_mark(cx);
        let marked = range.is_some_and(|range| view.update(cx, |v, cx| {
            let ok = v.set_range_highlights([gpui_base::RangeHighlight::new(range, c("selection"))], cx).is_ok();
            if ok { v.clear_selection(cx); }
            ok
        }));
        if marked { self.canvas_state.quote_mark = Some((view.clone(), id)); }
    }

    fn clear_quote_mark(&mut self, cx: &mut Context<Self>) {
        if let Some((view, _)) = self.canvas_state.quote_mark.take() {
            view.update(cx, |v, cx| v.clear_range_highlights(cx));
        }
    }

    /// A left press anywhere drops the quote highlight, except inside its open popover.
    pub(super) fn quote_press(&mut self, cx: &mut Context<Self>) {
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
        let fit_rects = if name == "canvas.fit" {
            self.canvas_state
                .doc
                .spaces
                .iter()
                .map(|s| s.rect)
                .chain(
                    self.canvas_state
                        .doc
                        .cards
                        .iter()
                        .map(|c| self.pill_rect(c, cx)),
                )
                .chain(self.canvas_state.doc.previews.iter().map(|p| p.rect))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let from = self.canvas_state.doc.camera;
        let state = &mut self.canvas_state;
        match name {
            "view.agents" | "view.all" | "view.terminals" => {
                state.doc.view = match name {
                    "view.agents" => canvas::View::Agents,
                    "view.terminals" => canvas::View::Terminals,
                    _ => canvas::View::All,
                };
                let agent = state
                    .panel
                    .as_ref()
                    .and_then(|a| self.live.get(a))
                    .is_some_and(canvas_agent);
                canvas::clear_hidden_focus(&state.doc.view, &mut state.panel, agent);
                if state.panel.is_none() {
                    self.focus.focus(window, cx);
                }
            }
            "panel.close" => {
                if !state.tabs.is_empty() {
                    state.closed = Some((state.panel.take(), state.file.take(), std::mem::take(&mut state.tabs)));
                }
                state.panel = None;
                state.file = None;
                state.files = false;
                self.focus.focus(window, cx);
            }
            "canvas.zoom_reset" => state.doc.camera.zoom_at(1., anchor),
            "canvas.fit" => {
                state
                    .doc
                    .camera
                    .fit(&fit_rects, [anchor[0] * 2., anchor[1] * 2.]);
            }
            _ => state
                .doc
                .camera
                .step(if name == "canvas.zoom_in" { 1 } else { -1 }, anchor),
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
        card: &canvas::Preview,
        view: &Entity<gpui_base::TextViewState>,
        camera: canvas::Camera,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = camera.to_screen([card.rect[0], card.rect[1]]);
        let path = std::path::Path::new(&card.path);
        let drag = card.path.clone();
        let close = drag.clone();
        let resize = drag.clone();
        let quote_path = card.path.clone();
        let quote_view = view.clone();
        (div().id(SharedString::from(format!("preview-{}", card.path))).absolute().left(px(p[0])).top(px(p[1]))
                .w(px(card.rect[2] * camera.zoom)).h(px(card.rect[3] * camera.zoom))
                .flex().flex_col().bg(c("surface")).border_1().border_color(c("border")).rounded(px(8.)).overflow_hidden()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().flex().items_center().gap(px(8.)).px(px(12.)).h(px(36. * camera.zoom)).flex_none().border_b_1().border_color(c("border"))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                        this.canvas_state.preview_drag = Some((drag.clone(), false, e.position)); cx.stop_propagation();
                    }))
                    .child(svg().path("mods/files/file.svg").size(px(16.)).text_color(c("foreground")))
                    .child(div().text_ellipsis().child(path.file_name().unwrap_or_default().to_string_lossy().into_owned()))
                    .child(div().flex_1().min_w_0().text_ellipsis().text_color(c("muted-foreground")).child(path.parent().unwrap_or(path).to_string_lossy().into_owned()))
                    .child(panel_button("preview-close", "mods/pane/close.svg", c("muted-foreground"), c("hover"))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.canvas_state.doc.previews.retain(|p| p.path != close);
                            this.canvas_state.previews.remove(&close);
                            this.canvas_changed(cx); cx.notify();
                        }))))
                .child(div().flex_1().min_h_0().overflow_hidden().px(px(16.)).py(px(12.)).child(gpui_base::TextView::new(view).scrollable(true)
                    .image_source(|_| {
                        static BLANK: std::sync::LazyLock<Arc<gpui::Image>> = std::sync::LazyLock::new(|| Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Svg, b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'/>".to_vec())));
                        gpui::ImageSource::Image(BLANK.clone())
                    })
                    .on_link_click(|_, _, _, _| {}))
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, e: &MouseUpEvent, window, cx| this.quote_selection(&quote_path, &quote_view, e.position, window, cx))))
                .child(div().absolute().bottom_0().right_0().size(px(12. * camera.zoom)).rounded_br(px(8.))
                    .border_r_2().border_b_2().border_color(c("muted-foreground")).cursor(CursorStyle::ResizeUpLeftDownRight)
                    .hover(|d| d.border_color(c("foreground")))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                        this.canvas_state.preview_drag = Some((resize.clone(), true, e.position)); cx.stop_propagation();
                    })))).into_any_element()
    }
    pub(super) fn render_canvas(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = &self.canvas_state;
        let camera = state.doc.camera;
        let bounds = self.bounds.borrow().get("canopy-canvas").copied();
        let viewport = bounds
            .map(|b| [f32::from(b.size.width), f32::from(b.size.height)])
            .unwrap_or([1200., 800.]);
        let maps = self.bounds.clone();
        let bg = self.person.canvas_color.as_deref().unwrap_or(
            if !settings::theme_is_light(&settings::get().theme) {
                "#1E1E1E"
            } else {
                "#F5F5F5"
            },
        );
        let mut area = div()
            .id("canopy-canvas")
            .relative()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .bg(c(bg))
            .map(|d| match (&state.drag, state.space_down) {
                (Some((None, _)), _) => d.cursor(CursorStyle::ClosedHand),
                (_, true) => d.cursor(CursorStyle::OpenHand),
                _ => d,
            })
            // Capture phase, before pills and space labels: middle drag, or left drag with
            // Space held, pans from anywhere on the canvas.
            .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                this.canvas_state.motion.target = None;
                this.canvas_state.motion.velocity = [0.; 2];
                this.canvas_state.motion.sample = Some(std::time::Instant::now());
                if e.button == MouseButton::Middle
                    || (e.button == MouseButton::Left && this.canvas_state.space_down)
                {
                    this.canvas_state.drag = Some((None, e.position));
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    move |b, _, _| {
                        maps.borrow_mut().insert("canopy-canvas".into(), b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
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
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, window, cx| {
                this.canvas_state.motion.target = None;
                this.canvas_state.motion.velocity = [0.; 2];
                let d = e.delta.pixel_delta(px(20.));
                if d.x == px(0.) && d.y == px(0.) {
                    return;
                }
                let b = this.bounds.borrow().get("canopy-canvas").copied();
                let origin = b.map(|b| b.origin).unwrap_or_default();
                if if cfg!(target_os = "macos") {
                    e.modifiers.platform
                } else {
                    e.modifiers.control
                } {
                    this.canvas_state.doc.camera.zoom_at(
                        canvas::rubber_band(this.canvas_state.doc.camera.zoom * canvas::zoom_factor(f32::from(d.y)*0.005)),
                        [
                            f32::from(e.position.x - origin.x),
                            f32::from(e.position.y - origin.y),
                        ],
                    );
                } else {
                    this.canvas_state.doc.camera.pan([d.x.into(), d.y.into()]);
                }
                if matches!(e.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    this.settle_zoom([f32::from(e.position.x-origin.x), f32::from(e.position.y-origin.y)], window, cx);
                }
                this.canvas_changed(cx);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_pinch(cx.listener(|this, e: &PinchEvent, window, cx| {
                this.canvas_state.motion.target = None;
                this.canvas_state.motion.velocity = [0.; 2];
                let origin = this
                    .bounds
                    .borrow()
                    .get("canopy-canvas")
                    .map(|b| b.origin)
                    .unwrap_or_default();
                this.canvas_state.doc.camera.zoom_at(
                    canvas::rubber_band(this.canvas_state.doc.camera.zoom * canvas::zoom_factor(e.delta)),
                    [
                        (e.position.x - origin.x).into(),
                        (e.position.y - origin.y).into(),
                    ],
                );
                if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    this.settle_zoom([(e.position.x-origin.x).into(), (e.position.y-origin.y).into()], window, cx);
                }
                this.canvas_changed(cx);
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
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
                // Reached only on empty canvas: pills, space labels and controls stop propagation.
                // A click without movement just focuses the canvas; moving pans.
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    this.focus.focus(window, cx);
                    this.canvas_state.drag = Some((None, e.position));
                    this.canvas_state.pan_moved = false;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                // The button came up outside the window: end any drag instead of panning on hover.
                if e.pressed_button.is_none()
                    && (this.canvas_state.drag.is_some() || this.canvas_state.space_drag.is_some())
                {
                    this.canvas_state.end_drag();
                    cx.notify();
                    return;
                }
                if let Some((path, resize, previous)) = this.canvas_state.preview_drag.clone() {
                    if e.pressed_button.is_none() {
                        this.canvas_state.end_drag();
                        return;
                    }
                    let z = this.canvas_state.doc.camera.zoom;
                    let d = [
                        f32::from(e.position.x - previous.x) / z,
                        f32::from(e.position.y - previous.y) / z,
                    ];
                    if let Some(card) = this
                        .canvas_state
                        .doc
                        .previews
                        .iter_mut()
                        .find(|p| p.path == path)
                    {
                        if resize {
                            card.rect[2] = (card.rect[2] + d[0]).max(240.);
                            card.rect[3] = (card.rect[3] + d[1]).max(120.);
                        } else {
                            card.rect[0] += d[0];
                            card.rect[1] += d[1];
                        }
                    }
                    this.canvas_state.preview_drag = Some((path, resize, e.position));
                    this.canvas_changed(cx);
                    cx.notify();
                    return;
                }
                if let Some((id, resize, previous)) = this.canvas_state.space_drag.clone() {
                    let zoom = this.canvas_state.doc.camera.zoom;
                    let delta = [
                        f32::from(e.position.x - previous.x) / zoom,
                        f32::from(e.position.y - previous.y) / zoom,
                    ];
                    let members: Vec<String> = this
                        .canvas_state
                        .doc
                        .cards
                        .iter()
                        .filter(|c| {
                            this.model
                                .pane_of(&c.terminal)
                                .and_then(|p| this.model.space_of(&p))
                                .is_some_and(|s| s.id == id)
                        })
                        .map(|c| c.terminal.clone())
                        .collect();
                    let rects: Vec<_> = this
                        .canvas_state
                        .doc
                        .cards
                        .iter()
                        .filter(|c| members.contains(&c.terminal))
                        .map(|c| this.pill_rect(c, cx))
                        .collect();
                    let doc = &mut this.canvas_state.doc;
                    if let Some(space) = doc.spaces.iter_mut().find(|s| s.space == id) {
                        if resize {
                            let min = canvas::min_size(space.rect, &rects);
                            space.rect[2] = (space.rect[2] + delta[0]).max(min[0]);
                            space.rect[3] = (space.rect[3] + delta[1]).max(min[1]);
                        } else {
                            canvas::move_space(space, &mut doc.cards, &members, delta);
                        }
                    }
                    this.canvas_state.space_drag = Some((id, resize, e.position));
                    this.canvas_changed(cx);
                    cx.notify();
                    return;
                }
                let Some((card, previous)) = this.canvas_state.drag.clone() else {
                    return;
                };
                let d = [
                    f32::from(e.position.x - previous.x),
                    f32::from(e.position.y - previous.y),
                ];
                if let Some(id) = card {
                    if let Some(card) = this
                        .canvas_state
                        .doc
                        .cards
                        .iter()
                        .find(|c| c.terminal == id)
                    {
                        let zoom = this.canvas_state.doc.camera.zoom;
                        let point = [card.rect[0] + d[0] / zoom, card.rect[1] + d[1] / zoom];
                        if let Err(error) = this.place_canvas_card(&id, point, cx) {
                            this.canvas_state.error = Some(error);
                        }
                    }
                } else {
                    let now = std::time::Instant::now();
                    let dt = this.canvas_state.motion.sample.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.016).max(0.001);
                    this.canvas_state.motion.velocity = [d[0]/dt, d[1]/dt];
                    this.canvas_state.motion.sample = Some(now);
                    this.canvas_state.doc.camera.pan(d);
                    this.canvas_state.pan_moved = true;
                }
                this.canvas_state.drag.as_mut().unwrap().1 = e.position;
                this.canvas_changed(cx);
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, window, cx| {
                    // A click that did not pan, inside a space: its files go to the panel tree.
                    let click = matches!(this.canvas_state.drag, Some((None, _))) && !this.canvas_state.pan_moved;
                    this.canvas_state.end_drag();
                    this.animate_camera(window, cx);
                    if let Some(space) = click.then(|| this.space_at(e.position)).flatten() {
                        this.show_space_files(space, window, cx);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, _, window, cx| {
                    this.canvas_state.end_drag();
                    this.animate_camera(window, cx);
                    cx.notify();
                }),
            );
        for space in &state.doc.spaces {
            if self
                .model
                .spaces
                .iter()
                .any(|s| s.id == space.space && root_space(s))
            {
                continue;
            }
            let id = space.space.clone();
            let resize_id = id.clone();
            let menu_id = id.clone();
            let model_space = self.model.space(&id);
            let label = model_space
                .map(|s| self.model.space_name(s, &home()))
                .unwrap_or_default();
            let [x, y] = camera.to_screen([space.rect[0], space.rect[1]]);
            area =
                area.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(space.rect[2] * camera.zoom))
                        .h(px(space.rect[3] * camera.zoom))
                        .rounded(px(12. * camera.zoom))
                        .child(
                            div()
                                .id(SharedString::from(format!("space-label-{id}")))
                                .absolute().top(px(-22.)).left_0().w_full().h(px(20.)).truncate()
                                .text_size(px(12.))
                                .flex()
                                .gap(px(8.))
                                .text_color(c("muted-foreground"))
                                .map(|d| {
                                    if let Some((editing, input, _)) = &state.rename {
                                        if editing == &id { return d.child(gpui_base::Input::new(input)); }
                                    }
                                    d.child(label)
                                })
                                .capture_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                                    if e.keystroke.key == "escape" {
                                        this.canvas_state.rename = None;
                                        this.focus.focus(window, cx);
                                        cx.stop_propagation(); cx.notify();
                                    }
                                }))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                        if this.canvas_state.rename.as_ref().is_some_and(|(editing, _, _)| editing == &id) {
                                            cx.stop_propagation(); return;
                                        }
                                        if e.click_count == 2 {
                                            this.rename_canvas_space(id.clone(), window, cx);
                                            cx.stop_propagation(); return;
                                        }
                                        this.canvas_state.space_drag =
                                            Some((id.clone(), false, e.position));
                                        this.show_space_files(id.clone(), window, cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                        this.open_menu(
                                            "canvas-space",
                                            json!({"id":menu_id}),
                                            Some(e.position),
                                            window,
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("space-resize-{resize_id}")))
                                .absolute()
                                .right_0()
                                .bottom_0()
                                .size(px(12. * camera.zoom))
                                .rounded_br(px(12. * camera.zoom))
                                .cursor(CursorStyle::ResizeUpLeftDownRight)
                                .border_r_2()
                                .border_b_2()
                                .border_color(c(&space.color).opacity(0.35))
                                .hover(|d| d.border_color(c(&space.color)))
                                .when(
                                    matches!(&state.space_drag, Some((s, true, _)) if *s == space.space),
                                    |d| d.border_color(c(&space.color)),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                        this.canvas_state.space_drag =
                                            Some((resize_id.clone(), true, e.position));
                                        cx.stop_propagation();
                                    }),
                                ),
                        )
                        .bg(c(&space.color).opacity(0.08))
                        .when(space.frame, |d| {
                            d.border_1().border_color(c(&space.color).opacity(0.6))
                        }),
                );
        }
        for card in &state.doc.previews {
            if canvas::visible(camera, card.rect, viewport) {
                if let Some((view, _)) = state.previews.get(&card.path) {
                    area = area.child(self.preview_card(card, view, camera, cx));
                }
            }
        }
        let cards: HashMap<_, _> = state
            .doc
            .cards
            .iter()
            .filter(|card| {
                canvas::view_allows(
                    &state.doc.view,
                    self.live
                        .get(&card.terminal)
                        .is_some_and(|r| canvas_agent(r)),
                )
            })
            .map(|card| {
                let size = state
                    .pill_sizes
                    .borrow()
                    .get(&card.terminal)
                    .map(|v| v.3)
                    .unwrap_or([240., 36.]);
                (
                    card.terminal.as_str(),
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
        let edges: Vec<_> = cards
            .values()
            .filter_map(|(child, rect)| {
                let parent = self.live.get(&child.terminal)?["parent"].as_str()?;
                let (_, parent_rect) = cards.get(parent)?;
                let ends = canvas::edge_ends(*parent_rect, *rect);
                canvas::edge_visible(camera, ends, viewport).then_some(ends)
            })
            .collect();
        area = area.child(
            canvas(
                |_, _, _| {},
                move |b, _, window, _| {
                    for (a, z) in &edges {
                        let a = camera.to_screen(*a);
                        let z = camera.to_screen(*z);
                        let p = |v: [f32; 2]| point(b.origin.x + px(v[0]), b.origin.y + px(v[1]));
                        let mut path = PathBuilder::stroke(px(1.));
                        path.move_to(p(a));
                        path.cubic_bezier_to(
                            p(z),
                            p([(a[0] + z[0]) / 2., a[1]]),
                            p([(a[0] + z[0]) / 2., z[1]]),
                        );
                        if let Ok(path) = path.build() {
                            window.paint_path(path, c("muted-foreground").opacity(0.5));
                        }
                        window.paint_quad(
                            fill(
                                Bounds::new(p([z[0] - 3., z[1] - 3.]), size(px(6.), px(6.))),
                                c("muted-foreground"),
                            )
                            .corner_radii(px(3.)),
                        );
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        let mut shown = shown;
        shown.sort_by_key(|c| c.z);
        for card in shown {
            let addr = card.terminal.clone();
            let drag_addr = addr.clone();
            let rec = self.live.get(&addr).unwrap_or(&Value::Null);
            let agent = canvas_agent(rec);
            let name = self.pill_name(&addr);
            let [x, y] = camera.to_screen([card.rect[0], card.rect[1]]);
            let z = camera.zoom;
            let rename_addr = addr.clone();
            let mut pill = div()
                .id(SharedString::from(format!("canvas-{addr}")))
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(card.rect[2] * z))
                .h(px(card.rect[3] * z))
                .rounded(px(8. * z))
                .border_1()
                .border_color(c(if state.panel.as_ref() == Some(&addr) {
                    "ring"
                } else {
                    "border"
                }))
                .bg(c("surface"))
                .flex()
                .items_start()
                .px(px(12. * z))
                .py(px(9. * z))
                .gap(px(8. * z))
                .text_size(px(13. * z))
                .line_height(px(18. * z))
                .text_color(c("foreground"))
                .overflow_hidden();
            let (icon, tint) = program_icon(rec);
            // Glyphs sit on the first line, so a two-line name keeps its icon top-left.
            pill = pill.child(div().flex_none().mt(px(1. * z)).child(icon_element(&icon, tint, px(16. * z), c("foreground"))));
            pill = pill.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .line_clamp(2)
                    .text_ellipsis()
                    .child(name)
                    .id(SharedString::from(format!("canvas-name-{addr}")))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                            if e.click_count == 2 {
                                if let Some(tab) = this
                                    .model
                                    .pane_of(&rename_addr)
                                    .and_then(|p| this.model.tab_of(&p))
                                    .map(|t| t.id.clone())
                                {
                                    this.run("tab.rename", json!({"id":tab}), window, cx);
                                }
                                cx.stop_propagation();
                            }
                        }),
                    ),
            );
            if agent && rec["status"] == "working" {
                pill = pill.child(
                    svg()
                        .path("mods/workspace-chrome/spinner.svg")
                        .size(px(12. * z))
                        .mt(px(3. * z))
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
                        .size(px(12. * z))
                        .mt(px(3. * z))
                        .text_color(c("muted-foreground")),
                );
            }
            if state.popped.contains_key(&addr) {
                pill = pill.child(
                    svg()
                        .path("icons/external-link.svg")
                        .size(px(12. * z))
                        .mt(px(3. * z))
                        .text_color(c("muted-foreground")),
                );
            }
            let menu_addr = addr.clone();
            let tab = self
                .model
                .pane_of(&addr)
                .and_then(|p| self.model.tab_of(&p))
                .map(|t| t.id.clone());
            pill = pill.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    this.open_menu(
                        "canvas-card",
                        json!({"terminal":menu_addr,"id":tab,"popped":this.canvas_state.popped.contains_key(&menu_addr)}),
                        Some(e.position),
                        window,
                        cx,
                    );
                    cx.stop_propagation();
                }),
            );
            area = area.child(pill.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    if this.canvas_state.popped.contains_key(&addr) {
                        this.pop_canvas(addr.clone(), false, window, cx);
                        cx.stop_propagation();
                        return;
                    }
                    this.canvas_state.drag = Some((Some(drag_addr.clone()), e.position));
                    cx.stop_propagation();
                    if e.click_count == 2 {
                        let center = this
                            .bounds
                            .borrow()
                            .get("canopy-canvas")
                            .map(|b| [f32::from(b.size.width) / 2., f32::from(b.size.height) / 2.])
                            .unwrap_or([400., 300.]);
                        if let Some(c) = this
                            .canvas_state
                            .doc
                            .cards
                            .iter()
                            .find(|c| c.terminal == drag_addr)
                        {
                            this.canvas_state.doc.camera = canvas::Camera {
                                x: c.rect[0] + this.pill_rect(c, cx)[2] / 2. - center[0],
                                y: c.rect[1] + 18. - center[1],
                                zoom: 1.,
                            };
                            this.canvas_changed(cx);
                        }
                    }
                    if let Some(p) = this.model.pane_of(&addr) {
                        this.model.focus_pane(&p);
                    }
                    this.canvas_state.panel = Some(addr.clone());
                    this.canvas_state.file = None;
                    this.sync(window, cx);
                    if let Some(t) = this.terms.get("canvas-panel") {
                        t.focus.focus(window, cx);
                    }
                    cx.notify();
                }),
            ));
        }
        let mut tabs = div()
            .absolute()
            .top(px(12.))
            .right(px(12.))
            .flex()
            .gap(px(4.))
            .p(px(4.))
            .rounded(px(8.))
            .bg(c("hover"))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        for (command, label, view) in [
            ("view.agents", "Agents", canvas::View::Agents),
            ("view.all", "All", canvas::View::All),
            ("view.terminals", "Terminals", canvas::View::Terminals),
        ] {
            tabs = tabs.child(
                div()
                    .id(command)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(12.))
                    .py(px(6.))
                    .rounded(px(6.))
                    .text_color(c("foreground"))
                    .when(state.doc.view == view, |d| d.bg(c("surface")))
                    .child(
                        svg()
                            .path(match command {
                                "view.agents" => "icons/bot.svg",
                                "view.all" => "icons/layers.svg",
                                _ => "icons/shell.svg",
                            })
                            .size(px(14.))
                            .text_color(c("foreground")),
                    )
                    .child(label)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.canvas_command(command, window, cx)
                    })),
            );
        }
        // Like Claude's toolbar: the folder opens or closes the files pane (tree and preview).
        let open = state.panel.is_some() || state.file.is_some() || state.files;
        tabs = tabs.child(
            div()
                .id("canvas-files")
                .flex()
                .items_center()
                .px(px(10.))
                .py(px(6.))
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|d| d.bg(c("surface")))
                .when(open, |d| d.bg(c("surface")))
                .child(svg().path("mods/files/folder.svg").size(px(14.)).text_color(c("foreground")))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if open {
                        this.canvas_command("panel.close", window, cx);
                    } else {
                        this.canvas_state.files = true;
                        this.canvas_state.doc.panel.tree_hidden = false;
                        // Bring back the tabs it closed with; files and terminals closed since are gone.
                        if let Some((panel, file, tabs)) = this.canvas_state.closed.take() {
                            let live = &this.live;
                            let state = &mut this.canvas_state;
                            state.tabs = tabs.into_iter().filter(|t| std::path::Path::new(t).is_absolute() || live.contains_key(t)).collect();
                            state.file = file;
                            state.panel = panel.filter(|p| live.contains_key(p));
                        }
                        this.sync(window, cx);
                        cx.notify();
                    }
                })),
        );
        area = area.child(tabs);
        let mut row = div()
            .size_full()
            .flex()
            .child(area)
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, window, cx| {
                if this.canvas_state.resizing {
                    let width = f32::from(window.viewport_size().width - e.position.x);
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
            );
        if state.panel.is_some() || state.file.is_some() || state.files {
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
            let mut panel = div()
                .relative()
                .w(px(width))
                .h_full()
                .flex_none()
                .flex()
                .flex_col()
                .border_l_1()
                .border_color(c("border"));
            panel = panel.child(
                div()
                    .id("canvas-panel-resize")
                    .absolute()
                    .left(px(-3.))
                    .top_0()
                    .w(px(6.))
                    .h_full()
                    .cursor(CursorStyle::ResizeLeftRight)
                    .hover(|d| d.bg(c("ring")))
                    .when(state.resizing, |d| d.bg(c("ring")))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.canvas_state.resizing = true;
                            cx.stop_propagation();
                        }),
                    ),
            );
            let addr = state.panel.as_deref().unwrap_or("");
            let name = self.pill_name(addr);
            let mut tabs = div()
                .id("canvas-tabs")
                .flex()
                .gap(px(4.))
                .px(px(8.))
                .py(px(6.))
                .h(px(40.))
                .flex_none()
                .overflow_x_scroll()
                .border_b_1()
                .border_color(c("border"));
            for tab in &state.tabs {
                let tab = tab.clone();
                let is_file = std::path::Path::new(&tab).is_absolute();
                let active = if is_file {
                    state.file.as_ref() == Some(&tab)
                } else {
                    state.file.is_none() && state.panel.as_ref() == Some(&tab)
                };
                let label = if is_file {
                    std::path::Path::new(&tab).file_name().unwrap_or_default().to_string_lossy().into_owned()
                } else {
                    self.pill_name(&tab)
                };
                let (icon, tint) = if is_file {
                    ("mods/files/file.svg".to_owned(), true)
                } else {
                    program_icon(self.live.get(&tab).unwrap_or(&Value::Null))
                };
                let activate = tab.clone();
                let close = tab.clone();
                tabs = tabs.child(
                    div()
                        .id(SharedString::from(format!("panel-tab-{tab}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .when(active, |d| d.bg(c("hover")))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if is_file {
                                this.canvas_state.file = Some(activate.clone());
                                this.focus.focus(window, cx);
                            } else {
                                this.canvas_state.panel = Some(activate.clone());
                                this.canvas_state.file = None;
                            }
                            this.sync(window, cx);
                            cx.notify();
                        }))
                        .child(icon_element(&icon, tint, px(14.), c("muted-foreground")))
                        .child(div().text_size(px(12.)).child(label))
                        .child(
                            panel_button(
                                "tab-close",
                                "mods/pane/close.svg",
                                c("muted-foreground"),
                                c("hover"),
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    let active = this
                                        .canvas_state
                                        .file
                                        .as_ref()
                                        .or(this.canvas_state.panel.as_ref())
                                        .cloned();
                                    let next = canvas::close_panel_tab(
                                        &mut this.canvas_state.tabs,
                                        active.as_deref(),
                                        &close,
                                    );
                                    this.canvas_state.file = next
                                        .as_ref()
                                        .filter(|p| std::path::Path::new(p).is_absolute())
                                        .cloned();
                                    if let Some(next) =
                                        next.filter(|p| !std::path::Path::new(p).is_absolute())
                                    {
                                        this.canvas_state.panel = Some(next);
                                    } else if this.canvas_state.tabs.is_empty() {
                                        this.canvas_state.panel = None;
                                    }
                                    this.sync(window, cx);
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
            panel = panel.child(tabs);
            let path = state
                .file
                .as_deref()
                .unwrap_or_else(|| self.cwds.get(addr).map(String::as_str).unwrap_or(&name));
            let root = self.panel_root();
            // Outside the tree's folder, paths read from home.
            let breadcrumb = canvas::breadcrumb(path, root.as_deref().filter(|r| std::path::Path::new(path).starts_with(r)).or(Some(&home())));
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
                    .when(state.file.is_none(), |d| {
                        d.child(
                            panel_button(
                                "canvas-panel-pop",
                                "icons/external-link.svg",
                                c("muted-foreground"),
                                c("hover"),
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    if let Some(addr) = this.canvas_state.panel.clone() {
                                        this.pop_canvas(addr, false, window, cx);
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
                    )
                    .child(
                        panel_button(
                            "canvas-panel-close",
                            "mods/pane/close.svg",
                            c("muted-foreground"),
                            c("hover"),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.canvas_command("panel.close", window, cx)
                        })),
                    ),
            );
            let mut body = div().flex().flex_1().min_h_0();
            if let Some((view, _)) = state
                .file
                .as_ref()
                .filter(|p| {
                    std::path::Path::new(p)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
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
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, e: &MouseUpEvent, window, cx| this.quote_selection(&quote_path, &quote_view, e.position, window, cx))));
            } else if state.file.is_some() {
                body = body.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .child(self.render_slot("canvas-file", cx)),
                );
            } else if let Some(t) = self.terms.get("canvas-panel").filter(|_| state.panel.is_some()) {
                body = body.child(div().flex_1().min_w_0().child(shown_terminal(&t.view, cx)));
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
        row.into_any_element()
    }
}

/// A 24 px icon button for the side panel header.
fn panel_button(id: &'static str, icon: &'static str, fg: Hsla, hover: Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(svg().path(icon).size(px(14.)).text_color(fg))
}

/// What a terminal is called on its pill, panel tab and quote row: a name the
/// person gave it, else an agent's session title or program name ("Claude Code"),
/// else the command in front ("uv run main.py"), else "terminal".
fn pill_label(given: Option<&str>, rec: &Value) -> String {
    let program = &rec["program"];
    let key = program["key"].as_str().unwrap_or_default();
    let shown = program["name"].as_str().unwrap_or_default();
    let generic = |s: &str| s.is_empty() || s == "shell" || s == "terminal" || s == key || s == shown;
    if let Some(name) = given.or(rec["name"].as_str()).map(str::trim).filter(|s| !generic(s)) {
        return name.to_owned();
    }
    if canvas_agent(rec) {
        let folder = rec["cwd"].as_str().and_then(|c| c.rsplit('/').next()).unwrap_or_default();
        // An agent's title is its session name once it has one; paths and the folder are not.
        return rec["title"].as_str().map(str::trim)
            .filter(|t| !generic(t) && *t != folder && !t.contains('/'))
            .or(Some(shown).filter(|s| !s.is_empty()))
            .unwrap_or("agent")
            .to_owned();
    }
    if program["shell"] == true {
        return "terminal".into();
    }
    let mut argv: Vec<&str> = rec.pointer("/foreground/argv").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    // The app's launch wrapper (`zsh -lic '"$@"; exec "$0" -li' zsh <cmd>...`, see programs::resolve) names <cmd>.
    if argv.len() > 4 && argv[2].contains("\"$@\"") { argv.drain(..4); }
    if argv.is_empty() { return if shown.is_empty() { "terminal".into() } else { shown.to_owned() }; }
    // Long commands are cut by the pill's ellipsis.
    argv.join(" ").replace(['\n', '\r'], " ")
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

/// Pills share one width so the canvas reads as a tidy grid.
const PILL_WIDTH: f32 = 220.;

fn canvas_agent(rec: &Value) -> bool {
    rec["program"]["agent"] == true
        || rec["status"]
            .as_str()
            .is_some_and(|s| matches!(s, "idle" | "working" | "blocked" | "done"))
}

/// The home "Canvas" space holds root terminals; it has no frame on the canvas.
fn root_space(s: &model::Space) -> bool {
    s.name == "Canvas" && s.folder == home() && s.machine.is_none()
}

fn preview_source(path: &str) -> String {
    match canvas::read_preview(std::path::Path::new(path)) {
        Ok(text)
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md")) =>
        {
            text
        }
        Ok(text) => numbered_code(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "File deleted".into(),
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
    fn numbered_code_keeps_markdown_literal() {
        assert_eq!(
            super::numbered_code("```\n![image](https://example.com)\n\nend"),
            "       1  ```\n       2  ![image](https://example.com)\n       3  \n       4  end"
        );
        assert_eq!(super::numbered_code(""), "");
    }
}

fn format_quote(quote: &Value, cwd: Option<&str>) -> String {
    let path = std::path::Path::new(quote["path"].as_str().unwrap_or_default());
    let display = cwd.and_then(|cwd| path.strip_prefix(cwd).ok()).unwrap_or(path);
    let mut header = format!("@{}", display.display());
    if let Some(start) = quote["start"].as_u64() {
        header.push_str(&format!(":{start}"));
        if let Some(end) = quote["end"].as_u64().filter(|end| *end != start) { header.push_str(&format!("-{end}")); }
    }
    let text = quote["text"].as_str().unwrap_or_default().trim_end_matches(['\r', '\n']);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    let body = if ext.eq_ignore_ascii_case("md") {
        format!("{header}\n{}", text.split('\n').map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n"))
    } else { format!("{header}\n```{ext}\n{text}\n```") };
    match quote["message"].as_str().map(str::trim).filter(|m| !m.is_empty()) {
        Some(message) => format!("{body}\n\n{message}"),
        None => body,
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
    fn quote_sources_and_format() {
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
        assert_eq!(pill_label(None, &claude("Fix pill labels")), "Fix pill labels");
        assert_eq!(pill_label(Some("review"), &claude("Fix pill labels")), "review");
        assert_eq!(pill_label(Some("claude"), &claude("")), "Claude Code");
        let shell = json!({"foreground":{"argv":["zsh"]},"program":{"key":"shell","name":"zsh","shell":true}});
        assert_eq!(pill_label(Some(""), &shell), "terminal");
        let uv = json!({"foreground":{"argv":["uv","run","main.py"]},"program":{"key":"uv","name":"uv","shell":false}});
        assert_eq!(pill_label(None, &uv), "uv run main.py");
        let wrapped = json!({"foreground":{"argv":["/bin/zsh","-lic","\"$@\"; exec \"$0\" -li","/bin/zsh","claude"]},"program":{"key":"zsh","name":"zsh","shell":false}});
        assert_eq!(pill_label(None, &wrapped), "claude");
    }
}
