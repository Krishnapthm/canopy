//! Sim panes: an iOS Simulator on the canvas. A booted simulator is live on macOS (its
//! framebuffer, touch and keys, in Xcode's bezel; see sim_screen.rs) while the pane is shown;
//! a simctl screenshot, taken when the pane opens, on Refresh and on `canopy pane open`, is the
//! resting and fallback picture. Nothing polls. See docs/wiki/Simulator-Devices.md.
use super::*;
use crate::sim::{self, Device};
#[cfg(target_os = "macos")]
use super::sim_screen::{AgentStep, By, Live, SimEvent, SimScreen};

/// What a sim pane's body shows.
pub(super) enum Screen {
    Loading,
    /// No simulator picked yet: the available ones.
    Pick(Vec<Device>),
    /// The picked simulator is shut down.
    Off,
    Booting,
    Shot(Arc<RenderImage>),
    /// The udid is no longer an available simulator.
    Gone,
    NoXcode,
    Failed(String),
}

/// A sim pane's runtime state; nothing here is saved.
pub(super) struct SimView {
    pub screen: Screen,
    pub device: Option<Device>,
    /// Every available simulator as of the last refresh, for the picker menu.
    pub devices: Vec<Device>,
    /// The latest screenshot as a file, for `canopy pane read`.
    pub png: Option<std::path::PathBuf>,
    /// The device itself, once a booted simulator is known.
    #[cfg(target_os = "macos")]
    pub live: Option<Entity<SimScreen>>,
    /// The pop-out window showing the device.
    #[cfg(target_os = "macos")]
    pub popped: Option<WindowHandle<SimWindow>>,
    _task: Option<Task<()>>,
    #[cfg(target_os = "macos")]
    _events: Option<Subscription>,
}

/// What a background refresh found.
enum Found {
    Pick(Vec<Device>),
    Gone(Vec<Device>),
    Off(Device, Vec<Device>),
    Shot(Device, Vec<Device>, Arc<RenderImage>, std::path::PathBuf),
}

/// Asks simctl about `udid` (empty: list devices to pick), boots it when asked,
/// and takes a screenshot of a booted device. Blocking.
fn look(pane: &str, udid: &str, boot: bool) -> Result<Found, String> {
    let devices = sim::devices()?;
    sim::xcode_version();
    if udid.is_empty() { return Ok(Found::Pick(devices)); }
    let Some(mut device) = devices.iter().find(|d| d.udid == udid).cloned() else { return Ok(Found::Gone(devices)) };
    if boot && !device.booted {
        sim::boot(udid)?;
        device.booted = true;
    }
    if !device.booted { return Ok(Found::Off(device, devices)); }
    let path = std::env::temp_dir().join("canopy").join(format!("sim-{pane}.png"));
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("Can't save the screenshot: {e}"))?;
    // A dropped refresh's simctl keeps running: each one writes its own file, then renames it.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let fresh = path.with_file_name(format!("sim-{pane}-{nanos}.png"));
    let shot = sim::screenshot(udid, &fresh).and_then(|_| std::fs::read(&fresh).map_err(|e| format!("Can't read the screenshot: {e}")));
    let saved = if shot.is_ok() { std::fs::rename(&fresh, &path) } else { std::fs::remove_file(&fresh) };
    let png = shot?;
    saved.map_err(|e| format!("Can't save the screenshot: {e}"))?;
    let mut rgba = image::load_from_memory_with_format(&png, image::ImageFormat::Png).map_err(|e| format!("simctl screenshot: {e}"))?.into_rgba8();
    // GPUI draws BGRA.
    for p in rgba.pixels_mut() { p.0.swap(0, 2); }
    Ok(Found::Shot(device, devices, Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(rgba)])), path))
}

/// Finds a simulator for `canvas.pane_new` and `canvas.pane_open`: a udid, or
/// "booted" for the first booted one.
/// shortcut: blocks the UI thread for one simctl list (about 0.4 s) so the agent gets a real error; move off-thread if control replies become async.
pub(super) fn resolve(target: &str) -> Result<String, String> {
    if !sim::available() { return Err("Simulator panes need Xcode on macOS".into()); }
    let devices = sim::devices()?;
    let found = if target == "booted" { devices.iter().find(|d| d.booted) } else { devices.iter().find(|d| d.udid == target) };
    found.map(|d| d.udid.clone()).ok_or_else(|| if target == "booted" { "No simulator is booted".into() } else { format!("{target} is not an available simulator (xcrun simctl list devices available)") })
}

impl AppWindow {
    /// Starts a refresh for each sim pane without a view, and drops views of closed panes.
    pub(super) fn load_sims(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.canvas_state.doc.panes.iter().filter(|p| p.sim().is_some()).map(|p| p.id.clone()).collect();
        let closed: Vec<String> = self.canvas_state.sims.keys().filter(|id| !ids.contains(id)).cloned().collect();
        self.forget_sims(&closed, cx);
        for id in ids {
            if !self.canvas_state.sims.contains_key(&id) { self.refresh_sim(&id, false, window, cx); }
        }
    }

    /// Adds a sim pane (udid empty: it lists simulators to pick) and starts showing it.
    pub(super) fn add_sim_pane(&mut self, udid: String, point: canvas::Point, space: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        // A phone in its bezel (about 1:2) under the 36 px header, 340 x 700 screen points at any zoom.
        let size = [340., 700.];
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Sim { udid, mode: canvas::SimMode::Fixed, screen: Some(size) }, rect: [point[0], point[1], size[0] / doc.camera.zoom, size[1] / doc.camera.zoom], z, space });
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        self.refresh_sim(&id, true, window, cx);
        id
    }

    /// Drops the views of closed sim panes: frees their screenshots, stops their helpers and
    /// recordings, and closes their pop-out windows.
    pub(super) fn forget_sims(&mut self, ids: &[String], cx: &mut App) {
        for id in ids {
            let Some(view) = self.canvas_state.sims.remove(id) else { continue };
            if let Screen::Shot(old) = view.screen { drop_shot(old, cx); }
            #[cfg(target_os = "macos")]
            {
                if let Some(screen) = &view.live {
                    screen.update(cx, |s, cx| {
                        if s.recording.is_some() { let _ = s.record(By::You, cx); }
                        s.shown = false;
                        s.sync(cx);
                    });
                }
                if let Some(handle) = view.popped { let _ = handle.update(cx, |_, window, _| window.remove_window()); }
            }
        }
    }

    /// Asks simctl again for a sim pane; `boot` boots a shut down simulator first.
    /// The last screenshot stays up until the new one is ready.
    pub(super) fn refresh_sim(&mut self, id: &str, boot: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(udid) = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.sim()).map(str::to_owned) else { return };
        let view = self.canvas_state.sims.entry(id.to_owned()).or_insert(SimView {
            screen: Screen::Loading, device: None, devices: vec![], png: None, _task: None,
            #[cfg(target_os = "macos")] live: None,
            #[cfg(target_os = "macos")] popped: None,
            #[cfg(target_os = "macos")] _events: None,
        });
        if !sim::available() {
            view.screen = Screen::NoXcode;
            return;
        }
        let booting = boot && !udid.is_empty() && !view.device.as_ref().is_some_and(|d| d.booted && d.udid == udid);
        if booting {
            set_screen(view, Screen::Booting, cx);
        } else if !matches!(view.screen, Screen::Shot(_)) {
            view.screen = Screen::Loading;
        }
        let pane = id.to_owned();
        view._task = Some(cx.spawn_in(window, async move |this, cx| {
            let look_pane = pane.clone();
            let found = cx.background_executor().spawn(async move { look(&look_pane, &udid, boot) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(view) = this.canvas_state.sims.get_mut(&pane) else { return };
                match found {
                    Ok(Found::Pick(devices)) => { view.device = None; view.devices = devices.clone(); set_screen(view, Screen::Pick(devices), cx); }
                    Ok(Found::Gone(devices)) => { view.device = None; view.devices = devices; set_screen(view, Screen::Gone, cx); }
                    Ok(Found::Off(device, devices)) => { view.device = Some(device); view.devices = devices; set_screen(view, Screen::Off, cx); }
                    Ok(Found::Shot(device, devices, image, png)) => {
                        view.device = Some(device);
                        view.devices = devices;
                        view.png = Some(png);
                        set_screen(view, Screen::Shot(image), cx);
                    }
                    Err(error) => {
                        set_screen(view, Screen::Failed(error.clone()), cx);
                        this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":error}}), window, cx);
                    }
                }
                #[cfg(target_os = "macos")]
                this.attach_live(&pane, window, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Gives a sim pane its device view once its simulator is known and booted, and hands it
    /// the latest screenshot; a different simulator gets a new one.
    #[cfg(target_os = "macos")]
    fn attach_live(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.canvas_state.sims.get_mut(pane) else { return };
        let Some(device) = view.device.clone() else { return };
        if view.live.as_ref().is_some_and(|s| s.read(cx).device.udid != device.udid) {
            let old = view.live.take().unwrap();
            view._events = None;
            old.update(cx, |s, cx| { s.shown = false; s.sync(cx); });
            if let Some(handle) = view.popped.take() { let _ = handle.update(cx, |_, window, _| window.remove_window()); }
        }
        let shot = if let Screen::Shot(image) = &view.screen { Some(image.clone()) } else { None };
        let screen = view.live.get_or_insert_with(|| cx.new(|cx| SimScreen::new(pane.to_owned(), device.clone(), cx))).clone();
        if view._events.is_none() {
            let id = pane.to_owned();
            view._events = Some(cx.subscribe_in(&screen, window, move |this, _, event, window, cx| this.sim_event(&id, event, window, cx)));
        }
        screen.update(cx, |s, cx| {
            s.device = device;
            s.set_shot(shot, cx);
            s.sync(cx);
        });
    }

    #[cfg(target_os = "macos")]
    fn sim_event(&mut self, pane: &str, event: &SimEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            SimEvent::Toast(title) => self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":title}}), window, cx),
            SimEvent::Saved { title, path } => self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{
                "title":title, "body":path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                "action":{"name":"canvas.sim_reveal","args":{"path":path}}}}), window, cx),
            SimEvent::Quote(quote) => {
                let mut quote = quote.clone();
                quote["point"] = json!({"x": f32::from(self.pointer.x), "y": f32::from(self.pointer.y)});
                self.quote_menu(&quote, window, cx);
            }
            SimEvent::Changed => cx.notify(),
            SimEvent::Fallback => self.refresh_sim(pane, false, window, cx),
        }
    }

    /// Runs a helper for every sim pane drawn now (or popped out) and stops the rest. Called
    /// after a canvas render with the panes it drew; changes only what differs.
    #[cfg(target_os = "macos")]
    pub(super) fn show_sims(&self, drawn: &[String], cx: &mut Context<Self>) {
        let changed: Vec<(Entity<SimScreen>, bool)> = self.canvas_state.sims.iter().filter_map(|(id, view)| {
            let screen = view.live.as_ref()?;
            let shown = drawn.contains(id) || view.popped.is_some();
            (screen.read(cx).shown != shown).then(|| (screen.clone(), shown))
        }).collect();
        if changed.is_empty() { return; }
        cx.defer(move |cx| for (screen, shown) in changed { screen.update(cx, |s, cx| { s.shown = shown; s.sync(cx); }); });
    }

    /// Points a sim pane at a simulator (empty: back to the list) and shows it, booting it if needed.
    pub(super) fn point_sim(&mut self, id: &str, udid: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
        let canvas::Kind::Sim { udid: saved, .. } = &mut pane.kind else { return };
        if *saved != udid {
            *saved = udid;
            self.canvas_changed(cx);
        }
        self.refresh_sim(id, true, window, cx);
    }

    /// The selected sim pane, for its helper bar commands.
    fn selected_sim(&self) -> Option<String> {
        let doc = &self.canvas_state.doc;
        self.canvas_state.selected.iter().rev().find(|id| doc.panes.iter().any(|p| &p.id == *id && p.sim().is_some())).cloned()
    }

    /// The `canvas.sim_*` commands; all but sim_hub, sim_reveal and sim_input act on the sim
    /// pane named by `pane`, else the selected one.
    pub(in crate::window) fn sim_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let toast = |this: &mut Self, e: &str, window: &mut Window, cx: &mut Context<Self>| this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":e}}), window, cx);
        match name {
            "canvas.sim_hub" => {
                let r = sim::open_device_hub();
                if let Err(e) = &r { toast(self, e, window, cx); }
                return self.answer(r.map(|_| json!({})));
            }
            "canvas.sim_reveal" => {
                let Some(path) = a["path"].as_str().filter(|p| std::path::Path::new(p).is_absolute()) else { return self.fail("canvas.sim_reveal needs an absolute path") };
                let r = std::process::Command::new("open").arg("-R").arg(path).status().map_err(|e| format!("Can't show {path}: {e}"));
                return self.answer(r.map(|_| json!({})));
            }
            "canvas.sim_input" => return self.sim_input(a, window, cx),
            _ => {}
        }
        let named = a["pane"].as_str().filter(|id| self.canvas_state.doc.panes.iter().any(|p| &p.id == id && p.sim().is_some())).map(str::to_owned);
        let Some(id) = named.or_else(|| self.selected_sim()) else { return self.fail(format!("{name}: select a simulator pane first, or name one with pane")) };
        match name {
            "canvas.sim_device" => return self.sim_picker(&id, window, cx),
            "canvas.sim_refresh" => {
                #[cfg(target_os = "macos")]
                if let Some(screen) = self.canvas_state.sims.get(&id).and_then(|v| v.live.clone()) { screen.update(cx, |s, cx| s.retry(cx)); }
                self.refresh_sim(&id, false, window, cx);
            }
            #[cfg(target_os = "macos")]
            _ => {
                let Some(screen) = self.canvas_state.sims.get(&id).and_then(|v| v.live.clone()) else {
                    return self.fail(format!("{name}: the simulator is not booted"));
                };
                let result = match name {
                    "canvas.sim_home" => { screen.update(cx, |s, _| s.press("home", By::You)); Ok(json!({})) }
                    "canvas.sim_screenshot" => screen.update(cx, |s, cx| s.screenshot(By::You, cx)).map(|p| json!({"path":p})),
                    "canvas.sim_record" => screen.update(cx, |s, cx| s.record(By::You, cx)).map(|p| json!({"path":p})),
                    "canvas.sim_annotate" => screen.update(cx, |s, cx| s.annotate(cx)).map(|_| json!({})),
                    "canvas.sim_zoom" => self.zoom_sim(&id, a["step"].as_str().unwrap_or("actual"), &screen, cx),
                    "canvas.sim_size" => self.sim_size(&id, a["size"].as_str(), &screen, window, cx),
                    "canvas.sim_pop" => { self.pop_sim(&id, &screen, window, cx); Ok(json!({})) }
                    _ => Err(format!("unknown command {name}")),
                };
                if let Err(e) = &result { toast(self, e, window, cx); }
                return self.answer(result);
            }
            #[cfg(not(target_os = "macos"))]
            _ => return self.fail("Live simulators need macOS"),
        }
        self.answer(Ok(json!({})));
    }

    /// The simulator picker: every available simulator, the current one checked, with Xcode's
    /// version on top. Picking one points the pane at it, booting it.
    fn sim_picker(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.canvas_state.sims.get(id) else { return self.answer(Ok(json!({}))) };
        let current = view.device.as_ref().map(|d| d.udid.as_str());
        let mut items = vec![json!([sim::xcode_version().unwrap_or_else(|| "Xcode".into()), null, {}])];
        for d in &view.devices {
            let mark = if Some(d.udid.as_str()) == current { "✓ " } else { "" };
            let state = if d.booted { " · Booted" } else { "" };
            items.push(json!([format!("{mark}{} · {}{state}", d.name, d.runtime), "canvas.pane_open", {"pane":id,"target":d.udid}]));
        }
        items.push(json!(["Open Device Hub", "canvas.sim_hub", {}]));
        self.open_menu("primary", json!({"items":items}), None, window, cx);
        self.answer(Ok(json!({})));
    }

    /// Resizes a sim pane so its device draws at 1 point per pixel ("actual"), or one step
    /// larger ("in") or smaller ("out"). The top left stays put; the new size is the pane's
    /// fixed screen size.
    #[cfg(target_os = "macos")]
    fn zoom_sim(&mut self, id: &str, step: &str, screen: &Entity<SimScreen>, cx: &mut Context<Self>) -> Result<Value, String> {
        let now = screen.read(cx).scale().unwrap_or(1.);
        let scale = match step { "in" => now * 1.25, "out" => now * 0.8, _ => 1. }.clamp(0.2, 3.);
        self.size_sim(id, scale, canvas::SimMode::Fixed, screen, cx)
    }

    /// Sets a sim pane's screen size so its device draws at `scale` display points per device
    /// point, and its mode. The top left stays put.
    #[cfg(target_os = "macos")]
    fn size_sim(&mut self, id: &str, scale: f32, new_mode: canvas::SimMode, screen: &Entity<SimScreen>, cx: &mut Context<Self>) -> Result<Value, String> {
        let s = screen.read(cx);
        let Some(points) = s.points() else { return Err("The device size is not known yet".into()) };
        let chrome = s.info.as_ref().and_then(|i| i.chrome.as_ref());
        let outer = chrome.map_or(points, |c| [c.size[0] + 2. * c.pad, c.size[1]]);
        let margin = 2. * super::sim_screen::MARGIN;
        let zoom = self.canvas_state.doc.camera.zoom;
        let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id) else { return Err(format!("no pane {id}")) };
        let canvas::Kind::Sim { mode, screen, .. } = &mut pane.kind else { return Err(format!("{id} is not a simulator pane")) };
        // The body is the pane minus its 36 px header and 1 px borders.
        let size = [outer[0] * scale + margin + 2., outer[1] * scale + margin + canvas::HEADER + 2.];
        (*mode, *screen) = (new_mode, Some(size));
        (pane.rect[2], pane.rect[3]) = (size[0] / zoom, size[1] / zoom);
        self.solve_layout(Some(id), None, cx);
        self.canvas_changed(cx);
        cx.notify();
        Ok(json!({"scale":scale}))
    }

    /// `canvas.sim_size`: Fixed keeps the pane's screen size; Actual draws the phone at its
    /// physical size on this window's display. No size toggles.
    #[cfg(target_os = "macos")]
    fn sim_size(&mut self, id: &str, want: Option<&str>, screen: &Entity<SimScreen>, window: &Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let want = match (want, self.sim_mode(id)) {
            (Some("actual"), _) | (None, Some(canvas::SimMode::Fixed)) => canvas::SimMode::Actual,
            (Some("fixed"), _) | (None, _) => canvas::SimMode::Fixed,
            (Some(other), _) => return Err(format!("size is actual or fixed, not {other}")),
        };
        match want {
            canvas::SimMode::Fixed => {
                if let Some(canvas::Kind::Sim { mode, .. }) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).map(|p| &mut p.kind) { *mode = want; }
                self.canvas_changed(cx);
                cx.notify();
                Ok(json!({"size":"fixed"}))
            }
            canvas::SimMode::Actual => {
                let scale = Self::actual_scale(window, screen, cx).map_err(|e| format!("Can't show this simulator at its real size: {e}"))?;
                self.size_sim(id, scale, want, screen, cx)?;
                Ok(json!({"size":"actual","scale":scale}))
            }
        }
    }

    /// Display points per device point that draw this device at its physical size on the
    /// window's display, or why that is not known.
    #[cfg(target_os = "macos")]
    fn actual_scale(window: &Window, screen: &Entity<SimScreen>, cx: &App) -> Result<f32, String> {
        let s = screen.read(cx);
        let info = s.info.as_ref().ok_or("the device size is not known yet")?;
        let pitch = info.pitch.filter(|p| *p > 0.).ok_or("Xcode lists no screen density for it")?;
        let display = window.display(cx).ok_or("the display is unknown")?;
        let id = u32::try_from(u64::from(display.id())).map_err(|_| "the display id is out of range")?;
        let ppi = crate::sim_live::points_per_inch(id, f32::from(display.bounds().size.width)).ok_or("macOS reports no physical size for this display")?;
        Ok(sim::actual_scale(ppi, info.scale, pitch))
    }

    /// A sim pane's size mode.
    fn sim_mode(&self, id: &str) -> Option<canvas::SimMode> {
        self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| match &p.kind { canvas::Kind::Sim { mode, .. } => Some(*mode), _ => None })
    }

    /// The window moved to another display: sim panes at their actual size are resized for it.
    /// Errors toast once per pane.
    #[cfg(target_os = "macos")]
    pub(in crate::window) fn display_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = window.display(cx).map(|d| u64::from(d.id()));
        let before = std::mem::replace(&mut self.last_display, now);
        if before.is_none() || before == now { return; }
        let ids: Vec<String> = self.canvas_state.doc.panes.iter().filter(|p| self.sim_mode(&p.id) == Some(canvas::SimMode::Actual)).map(|p| p.id.clone()).collect();
        for id in ids {
            let Some(screen) = self.canvas_state.sims.get(&id).and_then(|v| v.live.clone()) else { continue };
            let result = Self::actual_scale(window, &screen, cx)
                .map_err(|e| format!("Can't show this simulator at its real size: {e}"))
                .and_then(|scale| self.size_sim(&id, scale, canvas::SimMode::Actual, &screen, cx));
            if let Err(e) = result { self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":e}}), window, cx); }
        }
    }

    /// Pops a sim pane's device out into its own window, or brings it back.
    #[cfg(target_os = "macos")]
    fn pop_sim(&mut self, id: &str, screen: &Entity<SimScreen>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.canvas_state.sims.get_mut(id) else { return };
        if let Some(handle) = view.popped.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            screen.update(cx, |s, cx| { s.popped = false; cx.notify(); });
            return cx.notify();
        }
        let (name, runtime) = { let d = &screen.read(cx).device; (d.name.clone(), d.runtime.clone()) };
        let owner = window.window_handle().downcast::<AppWindow>().unwrap();
        let app = cx.entity().downgrade();
        let (pane, closed, shown) = (id.to_owned(), id.to_owned(), screen.clone());
        let outer = screen.read(cx).info.as_ref().and_then(|i| i.chrome.as_ref()).map_or([454., 908.], |c| [c.size[0] + 2. * c.pad, c.size[1]]);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(outer[0] * 0.8 + 40.), px(outer[1] * 0.8 + 80.)), cx))),
            ..Default::default()
        };
        let opened_by = owner;
        let id = id.to_owned();
        let screen = screen.clone();
        // Opening draws the window at once, and its helper bar reads this window: open it once
        // this update has ended, or gpui refuses the nested update.
        cx.defer(move |cx| {
            let opened = cx.open_window(options, move |window, cx| {
                window.set_window_title(&format!("{name} · {runtime}"));
                window.on_window_should_close(cx, move |_, cx| {
                    let _ = owner.update(cx, |this, _, cx| {
                        if let Some(view) = this.canvas_state.sims.get_mut(&closed) {
                            view.popped = None;
                            if let Some(s) = &view.live { s.update(cx, |s, cx| { s.popped = false; cx.notify(); }); }
                        }
                        cx.notify();
                    });
                    true
                });
                cx.new(|cx| {
                    // The bar's labels follow the pane's state.
                    let follow = app.upgrade().map(|app| cx.observe(&app, |_, _, cx| cx.notify()));
                    SimWindow { app, pane, screen: shown, _follow: follow }
                })
            });
            let _ = opened_by.update(cx, |this, window, cx| match opened {
                Ok(handle) => {
                    if let Some(view) = this.canvas_state.sims.get_mut(&id) { view.popped = Some(handle); }
                    screen.update(cx, |s, cx| { s.popped = true; s.shown = true; s.sync(cx); cx.notify(); });
                    cx.notify();
                }
                Err(e) => this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":format!("Can't open a window: {e}")}}), window, cx),
            });
        });
        cx.notify();
    }

    /// A sim pane's helper bar labels for its state: stop a recording, end an annotation,
    /// bring a popped-out device back.
    #[cfg(target_os = "macos")]
    pub(super) fn sim_relabel(&self, id: &str, mut actions: Vec<super::helper_bar::BarAction>, cx: &App) -> Vec<super::helper_bar::BarAction> {
        let Some(screen) = self.canvas_state.sims.get(id).and_then(|v| v.live.as_ref()) else { return actions };
        let s = screen.read(cx);
        for a in &mut actions {
            match a.command.as_str() {
                "canvas.sim_record" if s.recording.is_some() => { a.icon = "icons/square.svg".into(); a.label = "Stop recording".into(); }
                "canvas.sim_annotate" if s.annotation.is_some() => a.label = "Cancel annotation".into(),
                "canvas.sim_pop" if s.popped => a.label = "Bring back".into(),
                "canvas.sim_size" if self.sim_mode(id) == Some(canvas::SimMode::Actual) => a.label = "Fixed size".into(),
                _ => {}
            }
        }
        actions
    }

    /// The helper bar of a popped-out sim pane's window: the device's own actions.
    #[cfg(target_os = "macos")]
    pub(super) fn sim_bar(&self, id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        const KEEP: [&str; 6] = ["canvas.sim_home", "canvas.sim_screenshot", "canvas.sim_record", "canvas.sim_annotate", "canvas.sim_device", "canvas.sim_pop"];
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id)?;
        let actions = super::helper_bar::pane_actions(pane, None).into_iter().filter(|a| KEEP.contains(&a.command.as_str())).collect();
        let actions = self.sim_relabel(id, actions, cx);
        Some(self.bar(id, actions, 1., false, cx).into_any_element())
    }

    /// `canopy pane tap|swipe|type|press`: an agent's input for a sim pane, in device points.
    fn sim_input(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(not(target_os = "macos"))]
        { let _ = (a, window, cx); return self.fail("Live simulators need macOS"); }
        #[cfg(target_os = "macos")]
        {
            let Some(id) = a["pane"].as_str() else { return self.fail("canvas.sim_input needs pane") };
            if !self.canvas_state.doc.panes.iter().any(|p| p.id == id && p.sim().is_some()) { return self.fail(format!("{id} is not a simulator pane")); }
            let Some(screen) = self.canvas_state.sims.get(id).and_then(|v| v.live.clone()) else {
                return self.fail(format!("{id}: the simulator is not booted; boot it with canopy pane open {id} <udid>"));
            };
            let steps = match agent_steps(a, screen.read(cx).points()) { Ok(steps) => steps, Err(e) => return self.fail(e) };
            screen.update(cx, |s, cx| s.agent(steps, cx));
            let _ = window;
            self.answer(Ok(json!({})));
        }
    }

    /// The body of a sim pane: its device, or what stands in for it.
    pub(super) fn sim_body(&self, card: &canvas::Pane, cx: &mut Context<Self>) -> AnyElement {
        let id = card.id.clone();
        let name = self.sim_device(card).map_or("the simulator".to_owned(), |d| d.name.clone());
        let center = || div().size_full().flex().flex_col().items_center().justify_center().gap(px(12.)).p(px(16.)).bg(c("canvas"));
        let note = |text: String| div().text_center().text_color(c("muted-foreground")).child(text);
        let button = |label: &'static str, cx: &mut Context<Self>, action: fn(&mut Self, &str, &mut Window, &mut Context<Self>)| {
            let id = id.clone();
            div().id(SharedString::from(format!("sim-{id}-{label}"))).role(gpui::Role::Button).aria_label(label)
                .px(px(12.)).py(px(4.)).rounded(px(ui::RADIUS_SM)).border_1().border_color(c("border")).cursor_pointer()
                .hover(|d| d.bg(c("hover"))).child(label)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| action(this, &id, window, cx)))
        };
        let Some(view) = self.canvas_state.sims.get(&id) else { return center().into_any_element() };
        #[cfg(target_os = "macos")]
        if let Some(screen) = view.live.as_ref().filter(|_| matches!(view.screen, Screen::Shot(_) | Screen::Loading)) {
            if view.popped.is_some() {
                return center().child(note(format!("{name} is in its own window"))).child(button("Bring back", cx, |this, id, w, cx| {
                    if let Some(screen) = this.canvas_state.sims.get(id).and_then(|v| v.live.clone()) { this.pop_sim(id, &screen, w, cx); }
                })).into_any_element();
            }
            return div().size_full().bg(c("canvas")).child(AnyView::from(screen.clone()).cached(StyleRefinement::default().size_full())).into_any_element();
        }
        match &view.screen {
            Screen::Shot(image) => center().p(px(8.)).child(img(image.clone()).size_full().object_fit(gpui::ObjectFit::Contain)).into_any_element(),
            Screen::Loading => center().into_any_element(),
            Screen::Booting => center().child(note(format!("Opening {name}…"))).into_any_element(),
            Screen::Off => center().child(note(format!("{name} is shut down"))).child(button("Boot", cx, |this, id, w, cx| this.refresh_sim(id, true, w, cx))).into_any_element(),
            Screen::Gone => center().child(note("This simulator no longer exists".into())).child(button("Choose another", cx, |this, id, w, cx| this.point_sim(id, String::new(), w, cx))).into_any_element(),
            Screen::NoXcode => center().child(note("Simulator panes need Xcode on macOS".into())).into_any_element(),
            Screen::Failed(error) => center().child(note(error.clone())).child(button("Try again", cx, |this, id, w, cx| this.refresh_sim(id, false, w, cx))).into_any_element(),
            Screen::Pick(devices) if devices.is_empty() => center().child(note("No simulators. Add one in Xcode's Device Hub.".into())).into_any_element(),
            Screen::Pick(devices) => div().id(SharedString::from(format!("sim-{id}-list"))).size_full().overflow_y_scroll().p(px(4.)).bg(c("canvas"))
                .children(devices.iter().map(|d| {
                    let (pane, udid) = (id.clone(), d.udid.clone());
                    div().id(SharedString::from(format!("sim-{id}-{}", d.udid))).role(gpui::Role::Button).aria_label(d.name.clone())
                        .flex().flex_col().px(px(8.)).py(px(6.)).rounded(px(ui::RADIUS_SM)).cursor_pointer().hover(|s| s.bg(c("hover")))
                        .child(div().text_ellipsis().child(d.name.clone()))
                        .child(div().text_size(px(12.)).text_color(c("muted-foreground")).child(if d.booted { format!("{} · Booted", d.runtime) } else { d.runtime.clone() }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, window, cx| this.point_sim(&pane, udid.clone(), window, cx)))
                })).into_any_element(),
        }
    }

    /// The known device of a sim pane, while it still is the pane's simulator.
    fn sim_device(&self, card: &canvas::Pane) -> Option<&Device> {
        self.canvas_state.sims.get(&card.id)?.device.as_ref().filter(|d| card.sim() == Some(d.udid.as_str()))
    }

    /// The header title and detail of a sim pane: name, then runtime and live status.
    pub(super) fn sim_title(&self, card: &canvas::Pane, cx: &App) -> (String, String) {
        let Some(d) = self.sim_device(card) else { return ("Simulator".into(), String::new()) };
        if !d.booted { return (d.name.clone(), format!("{} · Shut down", d.runtime)); }
        #[cfg(target_os = "macos")]
        if let Some(screen) = self.canvas_state.sims.get(&card.id).and_then(|v| v.live.as_ref()) {
            return (d.name.clone(), format!("{} · {}", d.runtime, screen.read(cx).status()));
        }
        let _ = cx;
        (d.name.clone(), d.runtime.clone())
    }

    /// `canopy pane read` for a sim pane: the device, its screen as a fresh PNG, what you and
    /// agents did lately, and how to drive it.
    pub(super) fn sim_read(&self, id: &str, udid: &str, cx: &App) -> String {
        let view = self.canvas_state.sims.get(id);
        let device = view.and_then(|v| v.device.as_ref());
        let mut png = view.and_then(|v| v.png.as_ref()).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let mut attrs = vec![("udid", udid.to_owned())];
        let mut extra = String::new();
        #[cfg(target_os = "macos")]
        if let Some(screen) = view.and_then(|v| v.live.as_ref()).map(|s| s.read(cx)) {
            let fresh = std::env::temp_dir().join("canopy").join(format!("sim-{id}-now.png"));
            if std::fs::create_dir_all(fresh.parent().unwrap()).is_ok() && screen.save_now(&fresh) { png = fresh.to_string_lossy().into_owned(); }
            attrs.push(("live", (if matches!(screen.live, Live::On { input: true }) { "yes" } else { "no" }).to_owned()));
            if let Some([w, h]) = screen.points() { attrs.push(("points", format!("{w}x{h}"))); }
            if let Some(r) = &screen.recording { attrs.push(("recording", r.path.to_string_lossy().into_owned())); }
            if !screen.log.is_empty() { extra = format!("\n\nRecent activity (local time):\n{}", screen.log.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n")); }
            extra.push_str(&format!("\n\nDrive it in device points (0,0 is the top left): canopy pane tap {id} <x> <y>, canopy pane swipe {id} <x1> <y1> <x2> <y2> [ms], canopy pane type {id} <text>, canopy pane press {id} home|lock|siri|volume-up|volume-down|action."));
        }
        let _ = cx;
        if let Some(d) = device { attrs.extend([("name", d.name.clone()), ("runtime", d.runtime.clone()), ("state", (if d.booted { "booted" } else { "shutdown" }).to_owned())]); }
        if !png.is_empty() { attrs.push(("screenshot", png.clone())); }
        let body = match (udid.is_empty(), device, png.is_empty()) {
            _ if let Some(Screen::Failed(error)) = view.map(|v| &v.screen) => format!("The last refresh failed: {error}"),
            _ if matches!(view.map(|v| &v.screen), Some(Screen::Loading | Screen::Booting)) => "Refreshing; read again in a moment.".to_owned(),
            (true, ..) => "No simulator picked yet.".to_owned(),
            (_, None, _) => "The simulator is not available.".to_owned(),
            (_, Some(d), false) => format!("{} ({}). The screenshot file shows its screen.{extra}", d.name, d.runtime),
            (_, Some(d), true) => format!("{} ({}) has no screenshot yet. Boot it and show it with: canopy pane open {id} {udid}", d.name, d.runtime),
        };
        let attrs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        canvas::envelope(id, "sim", &attrs, &body)
    }
}

/// An agent's input request as steps, checked against the screen size in points.
#[cfg(target_os = "macos")]
fn agent_steps(a: &Value, points: Option<[f32; 2]>) -> Result<Vec<AgentStep>, String> {
    use crate::sim_live::Input;
    let [w, h] = points.ok_or("the screen size is not known yet; try again in a moment")?;
    let num = |key: &str| a[key].as_f64().ok_or_else(|| format!("canvas.sim_input: {key} must be a number"));
    let at = |x: f64, y: f64| -> Result<[f64; 2], String> {
        if !(0.0..=w as f64).contains(&x) || !(0.0..=h as f64).contains(&y) { return Err(format!("({x}, {y}) is outside the {w}x{h} point screen")); }
        Ok([x / w as f64, y / h as f64])
    };
    match a["action"].as_str().unwrap_or_default() {
        "tap" => {
            let (x, y) = (num("x")?, num("y")?);
            let p = at(x, y)?;
            Ok(vec![AgentStep::Input(Input::Gesture { ms: 60, from: p, to: p }), AgentStep::Note(format!("tapped ({x}, {y})"))])
        }
        "swipe" => {
            let (x1, y1, x2, y2) = (num("x1")?, num("y1")?, num("x2")?, num("y2")?);
            let (from, to) = (at(x1, y1)?, at(x2, y2)?);
            let ms = a["ms"].as_u64().unwrap_or(300).clamp(50, 5000);
            Ok(vec![AgentStep::Input(Input::Gesture { ms, from, to }), AgentStep::Note(format!("swiped from ({x1}, {y1}) to ({x2}, {y2}) in {ms} ms"))])
        }
        "type" => {
            let text = a["text"].as_str().filter(|t| !t.is_empty()).ok_or("canvas.sim_input: type needs text")?;
            let (_, skipped) = crate::sim_live::type_text(text);
            if skipped > 0 { return Err(format!("{skipped} characters can't be typed on a US hardware keyboard; paste them instead")); }
            Ok(vec![AgentStep::Type(text.to_owned())])
        }
        "press" => {
            let button = a["button"].as_str().unwrap_or_default();
            if !crate::sim_live::BUTTONS.iter().any(|(n, ..)| *n == button) {
                return Err(format!("unknown button {button:?}; use one of: {}", crate::sim_live::BUTTONS.iter().map(|b| b.0).collect::<Vec<_>>().join(", ")));
            }
            Ok(vec![AgentStep::Press(button.to_owned())])
        }
        other => Err(format!("canvas.sim_input: unknown action {other:?} (tap, swipe, type or press)")),
    }
}

/// A sim pane's device in its own window, with the pane's helper bar along the bottom.
#[cfg(target_os = "macos")]
pub(super) struct SimWindow {
    app: WeakEntity<AppWindow>,
    pane: String,
    screen: Entity<SimScreen>,
    _follow: Option<Subscription>,
}

#[cfg(target_os = "macos")]
impl Render for SimWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = self.app.update(cx, |app, cx| app.sim_bar(&self.pane, cx)).ok().flatten();
        div().size_full().flex().flex_col().bg(c("canvas")).text_color(c("foreground")).font_family(".SystemUIFont")
            .child(div().flex_1().min_h_0().child(AnyView::from(self.screen.clone())))
            .when_some(bar, |d, bar| d.child(div().flex().justify_center().pb(px(12.)).child(bar)))
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::{AgentStep, agent_steps};
    use crate::sim_live::Input;
    use serde_json::json;

    fn inputs(steps: &[AgentStep]) -> Vec<Input> {
        steps.iter().filter_map(|s| if let AgentStep::Input(i) = s { Some(*i) } else { None }).collect()
    }

    #[test]
    fn agent_points_map_to_the_screen() {
        let pts = Some([402., 874.]);
        let tap = agent_steps(&json!({"action":"tap","x":201,"y":437}), pts).unwrap();
        assert_eq!(inputs(&tap), [Input::Gesture { ms: 60, from: [0.5, 0.5], to: [0.5, 0.5] }]);
        assert_eq!(agent_steps(&json!({"action":"tap","x":500,"y":10}), pts).err().unwrap(), "(500, 10) is outside the 402x874 point screen");
        let swipe = agent_steps(&json!({"action":"swipe","x1":0,"y1":0,"x2":402,"y2":874,"ms":100}), pts).unwrap();
        assert_eq!(inputs(&swipe), [Input::Gesture { ms: 100, from: [0., 0.], to: [1., 1.] }]);
        assert!(agent_steps(&json!({"action":"type","text":"héllo"}), pts).unwrap_err().starts_with("1 characters"));
        assert!(agent_steps(&json!({"action":"press","button":"menu"}), pts).unwrap_err().starts_with("unknown button"));
        assert!(agent_steps(&json!({"action":"tap","x":1,"y":1}), None).is_err());
    }
}

fn set_screen(view: &mut SimView, screen: Screen, cx: &mut App) {
    if let Screen::Shot(old) = std::mem::replace(&mut view.screen, screen) { drop_shot(old, cx); }
}

/// Frees a replaced screenshot's texture once this window update ends.
fn drop_shot(image: Arc<RenderImage>, cx: &mut App) {
    cx.defer(move |cx| cx.drop_image(image, None));
}
