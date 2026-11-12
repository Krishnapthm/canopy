// Modified for Canopy.
use super::*;
use crate::canvas::{self, Document};
use gpui::AnimationExt as _;

pub(super) struct State {
    pub doc: Document,
    pub panel: Option<String>,
    pub popped: HashMap<String, WindowHandle<PopOut>>,
    save: canvas::SaveSchedule,
    pub space_down: bool,
    resizing: bool,
    writable: bool,
    save_task: Option<Task<()>>,
    write_result: Option<std::sync::mpsc::Receiver<std::io::Result<()>>>,
    pill_sizes: RefCell<HashMap<String, (String, String, bool, canvas::Point)>>,
    pub error: Option<String>,
    pending_points: HashMap<String, canvas::Point>,
    space_drag: Option<(String, bool, Point<Pixels>)>,
    drag: Option<(Option<String>, Point<Pixels>)>,
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
            panel: None,
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
        }
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
                PopOut { terminal, _sub: None }
            })
        });
        match result {
            Ok(handle) => {
                let terminal = handle.read(cx).unwrap().terminal.clone();
                let block = self.model.pane_of(&addr).unwrap_or_else(|| "canvas-panel".into());
                let sub = self.setup_term(&terminal, &block, &addr, window, cx);
                let _ = handle.update(cx, |pop, _, _| pop._sub = Some(sub));
                self.canvas_state.popped.insert(addr, handle);
                self.canvas_state.panel = None;
                self.sync(window, cx);
            }
            Err(error) => self.fail(error.to_string()),
        }
    }

    pub(super) fn create_canvas(
        &mut self,
        args: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let point = args
            .get("point")
            .and_then(|v| serde_json::from_value::<canvas::Point>(v.clone()).ok())
            .filter(|p| p.iter().all(|v| v.is_finite()));
        let Some(point) = point else {
            return self.fail("invalid canvas point");
        };
        if args["kind"] == "space" {
            let Some(name) = args["name"].as_str() else {
                self.open_overlay(mods::new_overlay_id(), json!({"mod":"prompt", "kind":"dialog", "width":380, "props":{"title":"New space", "command":{"name":"canvas.create", "args":args}}}), window, cx);
                return;
            };
            let name = name.to_owned();
            let paths = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some("Space folder".into()),
            });
            let owner = window.window_handle().downcast::<AppWindow>().unwrap();
            cx.spawn(async move |this, cx| match paths.await {
                Ok(Ok(Some(paths))) => {
                    if let Some(folder) = paths.first() {
                        let folder = folder.to_string_lossy().into_owned();
                        let _ = owner.update(cx, |this, window, cx| {
                            if let Err(error) = this.require_document(None) {
                                this.canvas_state.error = Some(error);
                                return;
                            }
                            let id = this.model.fresh_id_on("sp", None);
                            this.model.spaces.push(model::Space {
                                id: id.clone(),
                                name,
                                folder,
                                machine: None,
                                tabs: vec![],
                                active: None,
                                rest: Default::default(),
                            });
                            let seed = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .subsec_nanos() as usize;
                            let color = canvas::unused_color(&this.canvas_state.doc.spaces, seed);
                            this.canvas_state.doc.spaces.push(canvas::Space {
                                space: id,
                                rect: [point[0], point[1], 960., 640.],
                                color: color.into(),
                                frame: true,
                            });
                            this.canvas_changed(cx);
                            this.flush_documents(false, window, cx);
                            cx.notify();
                        });
                    }
                }
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.canvas_state.error = Some(error.to_string());
                        cx.notify();
                    });
                }
                _ => {}
            })
            .detach();
            return;
        }
        let space = Self::owned(&args, "space");
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
                .find(|s| s.name == "Canvas" && s.folder == home() && s.machine.is_none())
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
                    if let Err(error) = self.place_canvas_card(addr, point, cx) {
                        self.fail(error);
                    }
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub(super) fn canvas_changed(&mut self, cx: &mut Context<Self>) {
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
    fn pill_rect(&self, card: &canvas::Card, cx: &App) -> canvas::Rect {
        let rec = self.live.get(&card.terminal).unwrap_or(&Value::Null);
        let agent = canvas_agent(rec);
        let kind = if agent {
            rec["program"]["key"].as_str().unwrap_or("agent")
        } else {
            "terminal"
        };
        let name = self
            .model
            .pane_of(&card.terminal)
            .and_then(|p| self.model.tab_of(&p))
            .map(|t| t.name.as_str())
            .filter(|s| !s.is_empty() && *s != "shell" && *s != kind)
            .or(rec["name"]
                .as_str()
                .filter(|s| !s.is_empty() && *s != "shell" && *s != kind))
            .or(rec["title"].as_str().filter(|s| !s.is_empty()))
            .or_else(|| {
                self.cwds
                    .get(&card.terminal)
                    .and_then(|s| s.rsplit('/').next())
            })
            .unwrap_or(&card.terminal);
        let text = format!("{kind} {}", name.replace(['\n', '\r'], " "));
        let family = settings::get().ui_font_family.to_string();
        let mut sizes = self.canvas_state.pill_sizes.borrow_mut();
        if let Some((label, cached_font, cached_agent, size)) = sizes.get(&card.terminal)
            && label == &text
            && cached_font == &family
            && *cached_agent == agent
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
        let size = [
            (width + 24. + 16. + 16. + if agent { 20. } else { 0. }).clamp(160., 320.),
            36.,
        ];
        sizes.insert(card.terminal.clone(), (text, family, agent, size));
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
                        .find(|s| s.name == "Canvas" && s.folder == home() && s.machine.is_none())
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
            if !(space.name == "Canvas" && space.folder == home() && space.machine.is_none())
                && !state.doc.spaces.iter().any(|s| s.space == space.id)
            {
                let x = state.doc.spaces.len() as f32 * 1056.;
                state.doc.spaces.push(canvas::Space {
                    space: space.id.clone(),
                    rect: [x, 0., 960., 640.],
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
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
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
                state.panel = None;
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
        self.canvas_changed(cx);
        self.sync(window, cx);
        cx.notify();
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
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
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
                    this.canvas_state.doc.camera.step(
                        if d.y > px(0.) { 1 } else { -1 },
                        [
                            f32::from(e.position.x - origin.x),
                            f32::from(e.position.y - origin.y),
                        ],
                    );
                } else {
                    this.canvas_state.doc.camera.pan([d.x.into(), d.y.into()]);
                }
                this.canvas_changed(cx);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_pinch(cx.listener(|this, e: &PinchEvent, _, cx| {
                if e.delta == 0. {
                    return;
                }
                let origin = this
                    .bounds
                    .borrow()
                    .get("canopy-canvas")
                    .map(|b| b.origin)
                    .unwrap_or_default();
                this.canvas_state.doc.camera.step(
                    if e.delta > 0. { 1 } else { -1 },
                    [
                        (e.position.x - origin.x).into(),
                        (e.position.y - origin.y).into(),
                    ],
                );
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
                    let space = this
                        .canvas_state
                        .doc
                        .spaces
                        .iter()
                        .rev()
                        .find(|s| {
                            point[0] >= s.rect[0]
                                && point[1] >= s.rect[1]
                                && point[0] <= s.rect[0] + s.rect[2]
                                && point[1] <= s.rect[1] + s.rect[3]
                        })
                        .map(|s| s.space.clone());
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
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, _| {
                if e.keystroke.key == "space" {
                    this.canvas_state.space_down = true;
                }
            }))
            .on_key_up(cx.listener(|this, e: &KeyUpEvent, _, _| {
                if e.keystroke.key == "space" {
                    this.canvas_state.space_down = false;
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    this.focus.focus(window, cx);
                    if this.canvas_state.space_down {
                        this.canvas_state.drag = Some((None, e.position));
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseDownEvent, _, _| {
                    this.canvas_state.drag = Some((None, e.position));
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
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
                    this.canvas_state.doc.camera.pan(d);
                }
                this.canvas_state.drag.as_mut().unwrap().1 = e.position;
                this.canvas_changed(cx);
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.canvas_state.drag = None;
                    this.canvas_state.space_drag = None;
                    this.canvas_state.resizing = false;
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, _, _, _| {
                    this.canvas_state.drag = None;
                    this.canvas_state.space_drag = None;
                    this.canvas_state.resizing = false;
                }),
            );
        for space in &state.doc.spaces {
            if self.model.spaces.iter().any(|s| {
                s.id == space.space
                    && s.name == "Canvas"
                    && s.folder == home()
                    && s.machine.is_none()
            }) {
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
                                .text_size(px(13. * camera.zoom))
                                .flex()
                                .gap(px(8. * camera.zoom))
                                .text_color(c("foreground"))
                                .child(label)
                                .child(div().text_color(c("muted-foreground")).child(
                                    model_space.map(|s| s.folder.clone()).unwrap_or_default(),
                                ))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                        this.canvas_state.space_drag =
                                            Some((id.clone(), false, e.position));
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
            let program = &rec["program"];
            let agent = canvas_agent(rec);
            let kind = if agent {
                program["key"].as_str().unwrap_or("agent")
            } else {
                "terminal"
            };
            let tab_name = self
                .model
                .pane_of(&addr)
                .and_then(|p| self.model.tab_of(&p))
                .map(|t| t.name.as_str())
                .filter(|s| !s.is_empty() && *s != "shell" && *s != kind);
            let name = tab_name
                .or(rec["name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && *s != "shell" && *s != kind))
                .or(rec["title"].as_str().filter(|s| !s.is_empty()))
                .or_else(|| self.cwds.get(&addr).and_then(|s| s.rsplit('/').next()))
                .unwrap_or(&addr);
            let [x, y] = camera.to_screen([card.rect[0], card.rect[1]]);
            let z = camera.zoom;
            let rename_addr = addr.clone();
            let mut pill = div()
                .id(SharedString::from(format!("canvas-{addr}")))
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(card.rect[2] * z))
                .h(px(36. * z))
                .rounded(px(8. * z))
                .border_1()
                .border_color(c(if state.panel.as_ref() == Some(&addr) {
                    "ring"
                } else {
                    "border"
                }))
                .bg(c("surface"))
                .flex()
                .items_center()
                .px(px(12. * z))
                .gap(px(8. * z))
                .text_size(px(13. * z))
                .text_color(c("foreground"))
                .overflow_hidden();
            if let Some(icon) = if agent {
                program["icon"].as_str().or(Some("icons/agent.svg"))
            } else {
                Some("icons/shell.svg")
            } {
                pill = pill.child(
                    svg()
                        .path(SharedString::from(
                            icon.to_string(),
                        ))
                        .size(px(16. * z))
                        .text_color(c("foreground")),
                );
            }
            pill = pill.child(kind.to_owned()).child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_ellipsis()
                    .text_color(c("muted-foreground"))
                    .child(name.to_owned())
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
                        .text_color(c("muted-foreground")),
                );
            }
            if state.popped.contains_key(&addr) {
                pill = pill.child(
                    svg()
                        .path("icons/external-link.svg")
                        .size(px(12. * z))
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
                    if this.canvas_state.space_down {
                        this.canvas_state.drag = Some((None, e.position));
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
            .bg(c("hover"));
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
        if state.panel.is_some() {
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
                    .left_0()
                    .top_0()
                    .w(px(6.))
                    .h_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.canvas_state.resizing = true;
                            cx.stop_propagation();
                        }),
                    ),
            );
            let addr = state.panel.as_deref().unwrap();
            let rec = self.live.get(addr).unwrap_or(&Value::Null);
            let program = &rec["program"];
            let agent = canvas_agent(rec);
            let kind = if agent {
                program["key"].as_str().unwrap_or("agent")
            } else {
                "terminal"
            };
            let name = self
                .model
                .pane_of(addr)
                .and_then(|p| self.model.tab_of(&p))
                .map(|t| t.name.as_str())
                .filter(|s| !s.is_empty() && *s != "shell" && *s != kind)
                .or(rec["name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && *s != "shell" && *s != kind))
                .or(rec["title"].as_str().filter(|s| !s.is_empty()))
                .or_else(|| self.cwds.get(addr).and_then(|s| s.rsplit('/').next()))
                .unwrap_or(addr);
            let icon = if agent {
                program["icon"].as_str().unwrap_or("icons/agent.svg")
            } else {
                "icons/shell.svg"
            };
            panel =
                panel.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(12.))
                        .h(px(36.))
                        .flex_none()
                        .border_b_1()
                        .border_color(c("border"))
                        .child(
                            svg()
                                .path(SharedString::from(
                                    icon.to_string(),
                                ))
                                .size(px(16.))
                                .text_color(c("foreground")),
                        )
                        .child(kind.to_owned())
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_ellipsis()
                                .text_color(c("muted-foreground"))
                                .child(name.to_owned()),
                        )
                        .child(
                            panel_button("canvas-panel-pop", "icons/external-link.svg", c("muted-foreground"), c("hover"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(addr) = this.canvas_state.panel.clone() {
                                        this.pop_canvas(addr, false, window, cx);
                                    }
                                })),
                        )
                        .child(
                            panel_button("canvas-panel-close", "mods/pane/close.svg", c("muted-foreground"), c("hover"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.canvas_command("panel.close", window, cx)
                                })),
                        ),
                );
            if let Some(t) = self.terms.get("canvas-panel") {
                panel = panel.child(div().flex_1().min_h_0().child(shown_terminal(&t.view, cx)));
            }
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

fn canvas_agent(rec: &Value) -> bool {
    rec["program"]["agent"] == true
        || rec["status"]
            .as_str()
            .is_some_and(|s| matches!(s, "idle" | "working" | "blocked" | "done"))
}
