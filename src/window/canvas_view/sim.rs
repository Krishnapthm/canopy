//! Sim panes: an iOS Simulator on the canvas. A booted simulator is live on macOS (its
//! framebuffer, touch and keys, in Xcode's bezel; see sim_screen.rs) while the pane is shown;
//! a simctl screenshot, taken when the pane opens, on Refresh and on `canopy pane open`, is the
//! resting and fallback picture. Nothing polls. See docs/wiki/Simulator-Devices.md.
use super::*;
use crate::sim::{self, Device};
#[cfg(target_os = "macos")]
use super::helper_bar::{pane_actions, sim_device_actions};
#[cfg(target_os = "macos")]
use super::sim_screen::{AgentStep, By, DeviceRoom, Live, SimEvent, SimScreen};

const DEFAULT_SIZE: [f32; 2] = [canophy::traits::SIM.w, canophy::traits::SIM.h];

/// Space for the shared pill and its bottom margin in world px, even when the pill is hidden.
#[cfg(target_os = "macos")]
pub(super) const DEVICE_BAR_ROW: f32 = 42.;

#[cfg(target_os = "macos")]
pub(super) fn device_room([w, h]: [f32; 2], zoom: f32) -> [f32; 2] {
    [w, (h - DEVICE_BAR_ROW * zoom).max(1.)]
}

/// The world size of a sim pane whose device (`outer` points, bezel included) draws at `scale`
/// world px per point. Only the frame stays in screen px and converts at the pane's `zoom`.
#[cfg(target_os = "macos")]
fn sim_pane_size(outer: [f32; 2], scale: f32, zoom: f32) -> [f32; 2] {
    let fixed = 2. * super::sim_screen::MARGIN + 2. / zoom;
    [outer[0] * scale + fixed, outer[1] * scale + fixed + canvas::HEADER + DEVICE_BAR_ROW]
}

#[cfg(target_os = "macos")]
fn bottom_bar_scale(zoom: f32, body_w: f32, buttons: usize) -> Option<f32> {
    // The pill follows the pane zoom up to the scale that fits. Its two borders and side margins stay fixed.
    let scaled_width = 24. * buttons as f32 + 2. * buttons.saturating_sub(1) as f32 + 4.;
    let max_scale = (body_w - 16. - 2.) / scaled_width;
    let scale = zoom.min(max_scale);
    (scale >= 0.5).then_some(scale)
}

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

impl AppWindow {
    /// Finds a simulator by UDID, or the first booted one for `booted`.
    /// shortcut: blocks the UI thread for one simctl list (about 0.4 s) so the agent gets a real error; move off-thread if control replies become async.
    pub(super) fn resolve_sim(&self, target: &str) -> Result<String, String> {
        if !sim::available() { return Err("Simulator panes need Xcode on macOS".into()); }
        let devices = sim::devices()?;
        let found = if target == "booted" { devices.into_iter().find(|d| d.booted) } else { devices.into_iter().find(|d| d.udid == target) };
        let udid = found.map(|d| d.udid).ok_or_else(|| if target == "booted" { "No simulator is booted".into() } else { format!("{target} is not an available simulator (xcrun simctl list devices available)") })?;
        if self.canvas_state.shutting_down_sims.contains(&udid) { return Err("The simulator is shutting down".into()); }
        Ok(udid)
    }

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
    pub(super) fn add_sim_pane(&mut self, udid: String, point: canophy::Point, space: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        // A phone in its bezel (about 1:2) under the header, in world px like every pane.
        let [w, h] = DEFAULT_SIZE;
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Sim { udid }, rect: canophy::WorldRect::new(point[0], point[1], w, h), z, space, color: None, pinned: None });
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
                        s.stop(cx);
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
        if self.canvas_state.shutting_down_sims.contains(&udid) { return; }
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
            SimEvent::Changed => cx.notify(),
            SimEvent::Fallback => self.refresh_sim(pane, false, window, cx),
            SimEvent::FocusCanvas => {
                self.focus.focus(window, cx);
                window.activate_window();
            }
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
        if self.canvas_state.shutting_down_sims.contains(&udid) {
            return self.command_error("Choose simulator", "The simulator is shutting down", window, cx);
        }
        let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
        let canvas::Kind::Sim { udid: saved } = &mut pane.kind else { return };
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

    /// Stops the device helpers before shutdown, then resets the originating pane or closes it.
    fn shutdown_sim(&mut self, id: &str, close: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        if !sim::available() { return Err("Simulator panes need Xcode on macOS".into()); }
        let doc = &self.canvas_state.doc;
        let udid = doc.panes.iter().find(|p| p.id == id).and_then(|p| p.sim())
            .filter(|udid| !udid.is_empty()).ok_or("Choose a simulator first")?.to_owned();
        let panes: Vec<_> = doc.panes.iter().filter(|p| p.sim() == Some(&udid)).map(|p| p.id.clone()).collect();
        if panes.iter().any(|p| self.canvas_state.sims.get(p).is_some_and(|v| matches!(v.screen, Screen::Booting))) {
            return Err("Wait for the simulator to finish opening before shutting it down".into());
        }
        if !self.canvas_state.shutting_down_sims.insert(udid.clone()) {
            return Err("The simulator is already shutting down".into());
        }
        let device_name = self.canvas_state.sims.get(id).and_then(|v| v.device.as_ref())
            .filter(|d| d.udid == udid).map_or_else(|| "Simulator".to_owned(), |d| d.name.clone());
        #[cfg(target_os = "macos")]
        let recordings: Vec<_> = panes.iter().filter_map(|id| {
            let screen = self.canvas_state.sims.get(id)?.live.as_ref()?;
            screen.update(cx, |s, cx| s.finish_recording(By::You, cx))
        }).collect();
        self.forget_sims(&panes, cx);
        cx.notify();
        let id = id.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            #[cfg(target_os = "macos")]
            for saved in recordings {
                let event = saved.await;
                let _ = this.update_in(cx, |this, window, cx| this.sim_event(&id, &event, window, cx));
            }
            let target = udid.clone();
            let result = cx.background_executor().spawn(async move { sim::shutdown(&target) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.canvas_state.shutting_down_sims.remove(&udid);
                let current = this.canvas_state.doc.panes.iter().any(|p| p.id == id && p.sim() == Some(&udid));
                if result.is_ok() {
                    let affected: Vec<_> = this.canvas_state.doc.panes.iter().filter(|p| p.sim() == Some(&udid)).map(|p| p.id.clone()).collect();
                    this.forget_annotations(&affected, cx);
                    if current && close {
                        this.close_boxed_panes(std::slice::from_ref(&id), cx);
                    } else if current {
                        this.point_sim(&id, String::new(), window, cx);
                    }
                }
                this.load_sims(window, cx);
                let mut notification = match result {
                    Ok(()) => json!({"title":format!("{device_name} shut down")}),
                    Err(error) => json!({"title":format!("Could not shut down {device_name}"),"body":error,"level":"error"}),
                };
                if let Some(pane) = this.canvas_state.doc.panes.iter().find(|p| p.id == id || p.sim() == Some(&udid)) {
                    notification["action"] = json!({"name":"pane.focus","args":{"pane":pane.id}});
                }
                this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":notification}), window, cx);
                cx.notify();
            });
        }).detach();
        Ok(json!({}))
    }

    /// The `canvas.sim_*` commands; all but sim_hub, sim_reveal and sim_input act on the sim
    /// pane named by `pane`, else the selected one.
    pub(in crate::window) fn sim_command(&mut self, name: &str, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) {
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
        if name == "canvas.sim_shutdown" && !a["pane"].is_null() && named.is_none() {
            return self.fail("canvas.sim_shutdown: pane must name a simulator pane");
        }
        let Some(id) = named.or_else(|| self.selected_sim()) else { return self.fail(format!("{name}: select a simulator pane first, or name one with pane")) };
        match name {
            "canvas.sim_shutdown" => {
                let close = match &a["close"] {
                    Value::Null => false,
                    Value::Bool(close) => *close,
                    _ => return self.fail("canvas.sim_shutdown: close must be a boolean"),
                };
                let result = self.shutdown_sim(&id, close, window, cx);
                return self.answer(result);
            }
            #[cfg(target_os = "macos")]
            "canvas.sim_zoom" => {
                let result = self.zoom_sim(&id, a["step"].as_str().unwrap_or("actual"), person, window, cx);
                return self.answer(result);
            }
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

    /// Resets the pane or sizes its device in canvas points. In and out step the drawn scale.
    #[cfg(target_os = "macos")]
    fn zoom_sim(&mut self, id: &str, step: &str, person: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        if self.canvas_state.doc.docked(id) { return Err("Move the simulator to the canvas to change its size".into()); }
        let (size, result) = if step == "default" {
            (DEFAULT_SIZE, json!({}))
        } else {
            let s = self.canvas_state.sims.get(id).and_then(|v| v.live.as_ref())
                .ok_or("The device size is not known yet")?.read(cx);
            let (scale, points, zoom) = match step {
                "in" | "out" => {
                    let step = if step == "in" { 1.25 } else { 0.8 };
                    // The drawn scale is screen px per point; the pane's own is world px per point.
                    let pane_scale = self.canvas_state.doc.panes.iter().find(|p| p.id == id).map_or(1., |p| canophy::layout(self.canvas_state.doc.camera, p).scale);
                    ((s.scale().unwrap_or(1.) / pane_scale * step).clamp(0.2, 3.), s.points(), pane_scale)
                }
                // One world px per device point: the device's own size at 100%.
                "actual" => (1., s.live_points(), 1.),
                other => return Err(format!("Unknown simulator size {other:?}")),
            };
            let points = points.ok_or("The device size is not known yet")?;
            (sim_pane_size(s.outer_points(points), scale, zoom), json!({"scale":scale}))
        };
        self.set_sim_size(id, size);
        self.solve_layout(Some(id), None, cx);
        // Actual size is one world px per point, so it is only true on screen at 100%.
        if step == "actual" && person { self.show_full_size(id, window, cx); }
        self.canvas_changed(cx);
        cx.notify();
        Ok(result)
    }

    /// The known device's aspect, bezel included, and its pane chrome in world px at tile zoom 1.
    pub(super) fn sim_shape(&self, id: &str, cx: &App) -> Option<(f32, [f32; 2])> {
        #[cfg(target_os = "macos")]
        {
            let screen = self.canvas_state.sims.get(id)?.live.as_ref()?.read(cx);
            let outer = screen.outer_points(screen.points()?);
            Some((outer[0] / outer[1], sim_pane_size(outer, 0., 1.)))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, cx);
            None
        }
    }

    /// Snaps a finished resize to the same fit as the device inside the dragged bounds.
    #[cfg(target_os = "macos")]
    pub(super) fn snap_sim_size(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(s) = self.canvas_state.sims.get(id).and_then(|v| v.live.as_ref()).map(|s| s.read(cx)) else { return };
        let Some(points) = s.points() else { return };
        let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return };
        let layout = canophy::layout(self.canvas_state.doc.camera, pane);
        let room = device_room(layout.body_size(), layout.scale);
        let Some(fit) = s.fit_at(room, layout.scale) else { return };
        let scale = fit.scale / layout.scale;
        self.set_sim_size(id, sim_pane_size(s.outer_points(points), scale, layout.scale));
        self.solve_layout(Some(id), None, cx);
        self.canvas_changed(cx);
    }

    #[cfg(target_os = "macos")]
    /// Sizes a sim pane to `[w, h]` world px (a pinned pane draws at scale 1, so the same
    /// numbers in screen px).
    fn set_sim_size(&mut self, id: &str, [w, h]: [f32; 2]) {
        let state = &mut self.canvas_state;
        let Some(pane) = state.doc.panes.iter().find(|p| p.id == id) else { return };
        let (rect, pinned) = (canophy::WorldRect { w, h, ..pane.rect }, pane.pinned);
        state.step(canophy::Intent::Place(vec![(id.to_owned(), rect)]));
        if let Some([x, y, ..]) = pinned { state.step(canophy::Intent::Pin { id: id.to_owned(), screen: Some([x, y, w, h]) }); }
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
                    screen.update(cx, |s, cx| { s.set_room(None); s.popped = true; s.shown = true; s.sync(cx); cx.notify(); });
                    cx.notify();
                }
                Err(e) => this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":format!("Can't open a window: {e}")}}), window, cx),
            });
        });
        cx.notify();
    }

    /// A simulator bar's labels for its state: stop a recording, end an annotation,
    /// bring a popped-out device back.
    #[cfg(target_os = "macos")]
    pub(super) fn sim_relabel(&self, id: &str, mut actions: Vec<super::helper_bar::BarAction>, cx: &App) -> Vec<super::helper_bar::BarAction> {
        let Some(screen) = self.canvas_state.sims.get(id).and_then(|v| v.live.as_ref()) else { return actions };
        let s = screen.read(cx);
        for a in &mut actions {
            match a.command.as_str() {
                "canvas.sim_record" if s.recording.is_some() => { a.icon = "icons/square.svg".into(); a.label = "Stop recording".into(); }
                "canvas.sim_annotate" if self.canvas_state.annotations.get(id).is_some_and(|a| a.annotating()) => a.label = "Stop annotating".into(),
                "canvas.sim_pop" if s.popped => a.label = "Bring back".into(),
                _ => {}
            }
        }
        // Annotation replaces the bottom device bar, so put its picker beside Connect.
        if let Some(name) = self.annotation_picker(id)
            && let Some(at) = actions.iter().position(|a| a.command == "canvas.connect")
        {
            actions.insert(at, super::helper_bar::BarAction {
                icon: "icons/bot.svg".into(), label: name.into(), command: "canvas.annotate_agent".into(), args: json!({}),
            });
        }
        actions
    }

    /// The shared controls below the phone in the canvas or Files panel.
    #[cfg(target_os = "macos")]
    pub(super) fn sim_bottom_bar(&self, pane: &canvas::Pane, layout: &canophy::Layout, cx: &mut Context<Self>) -> Option<Div> {
        let state = &self.canvas_state;
        let id = &pane.id;
        if state.annotations.get(id).is_some_and(|a| a.annotating()) || self.bars_hidden() { return None; }
        let view = state.sims.get(id);
        if view.is_some_and(|view| view.popped.is_some()) { return None; }
        let send = self.kept_annotation_send(id, cx);
        // Shown on a fallback screenshot too: Refresh is how the live view comes back.
        let show_controls = (state.bar_hover.as_ref() == Some(id) || state.selected.contains(id))
            && pane.sim().is_some_and(|udid| !udid.is_empty())
            && view.is_some_and(|view| matches!(view.screen, Screen::Shot(_) | Screen::Loading)
                && view.live.as_ref().is_none_or(|s| pane.sim() == Some(s.read(cx).device.udid.as_str())));
        if !show_controls {
            return send.map(|button| super::helper_bar::bar_shell(1.).child(button).occlude());
        }
        let mut actions = sim_device_actions();
        if self.sim_device(pane).is_some_and(|device| device.booted) {
            actions.push(super::helper_bar::sim_shutdown_action(id));
        }
        // A pinned pane keeps its screen size (scale 1), so its bar does too.
        let s = bottom_bar_scale(layout.scale, layout.body[2], actions.len())?;
        let actions = self.sim_relabel(id, actions, cx);
        Some(self.bar(id, actions, s, false, cx).children(send).occlude())
    }

    /// The pop-out's device bar also offers Bring back.
    #[cfg(target_os = "macos")]
    pub(super) fn sim_bar(&self, id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.canvas_state.sims.get(id)?.live.as_ref()?;
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id)?;
        let mut actions = sim_device_actions();
        actions.push(pane_actions(pane, None, false).into_iter().find(|a| a.command == "canvas.sim_pop")?);
        let actions = self.sim_relabel(id, actions, cx);
        Some(self.bar(id, actions, 1., false, cx).children(self.kept_annotation_send(id, cx)).into_any_element())
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

    /// A sim pane's whole body as the canvas card and the panel draw it: the device, Annotate, and
    /// the bottom bar of device actions.
    pub(super) fn sim_card_body(&self, card: &canvas::Pane, layout: canophy::Layout, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(target_os = "macos")]
        let has_row = self.has_device_row(card, cx);
        #[cfg(target_os = "macos")]
        if let Some(view) = self.canvas_state.sims.get(&card.id)
            && let Some(screen) = &view.live
            && view.popped.is_none()
        {
            let room = if has_row { device_room(layout.body_size(), layout.scale) } else { layout.body_size() };
            screen.update(cx, |s, cx| {
                let room = Some(DeviceRoom { room, zoom: layout.scale });
                if s.set_room(room) { cx.notify(); }
            });
        }
        let body = self.sim_body(card, cx);
        #[cfg(not(target_os = "macos"))]
        let _ = layout;
        #[cfg(target_os = "macos")]
        if has_row {
            let body = div().absolute().top_0().left_0().right_0().bottom(px(DEVICE_BAR_ROW * layout.scale)).child(body);
            let body = self.annotate_body(card, body.into_any_element(), layout.body_size(), cx);
            return div().relative().size_full().child(body)
                .when_some(self.sim_bottom_bar(card, &layout, cx), |d, bar| d.child(
                    div().id("sim-device-controls").absolute().bottom(px(12. * layout.scale)).left_0().right_0()
                        .flex().items_center().overflow_x_scroll()
                        .child(bar.flex_none().map(|mut bar| {
                            bar.style().margin.left = Some(Length::Auto);
                            bar.style().margin.right = Some(Length::Auto);
                            bar
                        }))))
                .into_any_element();
        }
        body
    }

    #[cfg(target_os = "macos")]
    fn has_device_row(&self, pane: &canvas::Pane, cx: &App) -> bool {
        self.canvas_state.annotations.contains_key(&pane.id) ||
            self.canvas_state.sims.get(&pane.id).is_some_and(|view|
                view.popped.is_none() && matches!(view.screen, Screen::Shot(_) | Screen::Loading) &&
                    view.live.as_ref().is_none_or(|screen| pane.sim() == Some(screen.read(cx).device.udid.as_str())))
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
        if card.sim().is_some_and(|udid| self.canvas_state.shutting_down_sims.contains(udid)) {
            return center().child(note("Shutting down…".into())).into_any_element();
        }
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
    use super::{AgentStep, agent_steps, bottom_bar_scale, device_room, sim_pane_size};
    use crate::sim_live::{Input, fit};
    use serde_json::json;

    fn inputs(steps: &[AgentStep]) -> Vec<Input> {
        steps.iter().filter_map(|s| if let AgentStep::Input(i) = s { Some(*i) } else { None }).collect()
    }

    #[test]
    fn pane_size_keeps_the_requested_device_scale_after_fit() {
        let points = [402., 874.];
        let scale = 0.75;
        for z in [0.5, 1., 2.] {
            let [w, h] = sim_pane_size(points, scale, z);
            let room = device_room([w * z - 2., (h - crate::canvas::HEADER) * z - 2.], z);
            assert!((fit(room, points, None, super::super::sim_screen::MARGIN * z).scale - scale * z).abs() < 1e-4);
        }
    }

    #[test]
    fn device_scale_follows_canvas_zoom() {
        let points = [402., 874.];
        let margin = super::super::sim_screen::MARGIN;
        let [w, h] = [330., 680.];
        let expected = fit(device_room([w, h], 1.), points, None, margin).scale;
        for zoom in [0.5, 1., 2.] {
            let room = device_room([w * zoom, h * zoom], zoom);
            let scale = fit(room, points, None, margin * zoom).scale;
            assert!((scale / zoom - expected).abs() < 1e-4);
        }
    }

    #[test]
    fn bottom_bar_scale_fits_available_width() {
        assert_eq!(bottom_bar_scale(1., 400., 5), Some(1.));
        assert_eq!(bottom_bar_scale(2., 400., 5), Some(2.));
        let s = bottom_bar_scale(1., 100., 5).unwrap();
        assert!(s >= 0.5 && (5. * 24. + 4. * 2. + 2. * 2.) * s + 2. <= 84.);
        assert_eq!(bottom_bar_scale(1., 84., 5), Some(0.5));
        assert_eq!(bottom_bar_scale(1., 83., 5), None);
        assert_eq!(bottom_bar_scale(1., 50., 5), None);
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
