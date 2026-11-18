//! Sim panes: an iOS Simulator's screen on the canvas. The screen is a screenshot
//! taken when the pane opens, on Refresh and on `canopy pane open`; nothing polls.
//! See docs/panes.md.
use super::*;
use crate::sim::{self, Device};

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
    /// The latest screenshot as a file, for `canopy pane read`.
    pub png: Option<std::path::PathBuf>,
    _task: Option<Task<()>>,
}

/// What a background refresh found.
enum Found {
    Pick(Vec<Device>),
    Gone,
    Off(Device),
    Shot(Device, Arc<RenderImage>, std::path::PathBuf),
}

/// Asks simctl about `udid` (empty: list devices to pick), boots it when asked,
/// and takes a screenshot of a booted device. Blocking.
fn look(pane: &str, udid: &str, boot: bool) -> Result<Found, String> {
    let devices = sim::devices()?;
    if udid.is_empty() { return Ok(Found::Pick(devices)); }
    let Some(mut device) = devices.into_iter().find(|d| d.udid == udid) else { return Ok(Found::Gone) };
    if boot && !device.booted {
        sim::boot(udid)?;
        device.booted = true;
        // shortcut: bootstatus returns before the home screen has drawn its icons; a fixed wait, until the live pane (P5b) shows frames.
        std::thread::sleep(std::time::Duration::from_secs(4));
    }
    if !device.booted { return Ok(Found::Off(device)); }
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
    Ok(Found::Shot(device, Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(rgba)])), path))
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
        // A phone screen (about 1:2.17) under the 36 px header.
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Sim { udid }, rect: [point[0], point[1], 320., 720.], z, space });
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        self.refresh_sim(&id, true, window, cx);
        id
    }

    /// Drops the views of closed sim panes and frees their screenshots.
    pub(super) fn forget_sims(&mut self, ids: &[String], cx: &mut App) {
        for id in ids {
            if let Some(SimView { screen: Screen::Shot(old), .. }) = self.canvas_state.sims.remove(id) { drop_shot(old, cx); }
        }
    }

    /// Asks simctl again for a sim pane; `boot` boots a shut down simulator first.
    /// The last screenshot stays up until the new one is ready.
    pub(super) fn refresh_sim(&mut self, id: &str, boot: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(udid) = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.sim()).map(str::to_owned) else { return };
        let view = self.canvas_state.sims.entry(id.to_owned()).or_insert(SimView { screen: Screen::Loading, device: None, png: None, _task: None });
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
                    Ok(Found::Pick(devices)) => { view.device = None; set_screen(view, Screen::Pick(devices), cx); }
                    Ok(Found::Gone) => { view.device = None; set_screen(view, Screen::Gone, cx); }
                    Ok(Found::Off(device)) => { view.device = Some(device); set_screen(view, Screen::Off, cx); }
                    Ok(Found::Shot(device, image, png)) => { view.device = Some(device); view.png = Some(png); set_screen(view, Screen::Shot(image), cx); }
                    Err(error) => {
                        set_screen(view, Screen::Failed(error.clone()), cx);
                        this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":error}}), window, cx);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Points a sim pane at a simulator (empty: back to the list) and shows it, booting it if needed.
    pub(super) fn point_sim(&mut self, id: &str, udid: String, window: &mut Window, cx: &mut Context<Self>) {
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

    /// canvas.sim_refresh, canvas.sim_device and canvas.sim_hub.
    pub(in crate::window) fn sim_command(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if name == "canvas.sim_hub" {
            let r = sim::open_device_hub();
            if let Err(e) = &r { self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":e}}), window, cx); }
            return self.answer(r.map(|_| json!({})));
        }
        let Some(id) = self.selected_sim() else { return self.fail(format!("{name}: select a simulator pane first")) };
        if name == "canvas.sim_device" { self.point_sim(&id, String::new(), window, cx); } else { self.refresh_sim(&id, false, window, cx); }
        self.answer(Ok(json!({})));
    }

    /// The body of a sim pane: its screen, or what stands in for it.
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

    /// The header title and detail of a sim pane.
    pub(super) fn sim_title(&self, card: &canvas::Pane) -> (String, String) {
        match self.sim_device(card) {
            Some(d) => (d.name.clone(), if d.booted { d.runtime.clone() } else { format!("{} · Shut down", d.runtime) }),
            None => ("Simulator".into(), String::new()),
        }
    }

    /// `canopy pane read` for a sim pane: the device and where its latest screenshot is.
    pub(super) fn sim_read(&self, id: &str, udid: &str) -> String {
        let view = self.canvas_state.sims.get(id);
        let device = view.and_then(|v| v.device.as_ref());
        let png = view.and_then(|v| v.png.as_ref()).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let mut attrs = vec![("udid", udid)];
        if let Some(d) = device { attrs.extend([("name", d.name.as_str()), ("runtime", d.runtime.as_str()), ("state", if d.booted { "booted" } else { "shutdown" })]); }
        if !png.is_empty() { attrs.push(("screenshot", png.as_str())); }
        let body = match (udid.is_empty(), device, png.is_empty()) {
            _ if let Some(Screen::Failed(error)) = view.map(|v| &v.screen) => format!("The last refresh failed: {error}"),
            _ if matches!(view.map(|v| &v.screen), Some(Screen::Loading | Screen::Booting)) => "Refreshing; read again in a moment.".to_owned(),
            (true, ..) => "No simulator picked yet.".to_owned(),
            (_, None, _) => "The simulator is not available.".to_owned(),
            (_, Some(d), false) => format!("{} ({}). The screenshot file shows its screen as last refreshed. Refresh it with: canopy pane open {id} {udid}", d.name, d.runtime),
            (_, Some(d), true) => format!("{} ({}) has no screenshot yet. Boot it and show it with: canopy pane open {id} {udid}", d.name, d.runtime),
        };
        canvas::envelope(id, "sim", &attrs, &body)
    }
}

fn set_screen(view: &mut SimView, screen: Screen, cx: &mut App) {
    if let Screen::Shot(old) = std::mem::replace(&mut view.screen, screen) { drop_shot(old, cx); }
}

/// Frees a replaced screenshot's texture once this window update ends.
fn drop_shot(image: Arc<RenderImage>, cx: &mut App) {
    cx.defer(move |cx| cx.drop_image(image, None));
}
