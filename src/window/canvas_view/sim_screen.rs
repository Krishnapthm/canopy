//! The live device inside a sim pane (macOS): Xcode's bezel and hardware buttons around the
//! simulator's framebuffer, drawn with gpui's `surface()` straight from its IOSurface, plus
//! touch, keys, screenshots, recordings and annotations. It is its own entity so a frame redraws
//! only this view, and so a pop-out window can show it. src/sim_live.rs runs the helper.
//! See docs/panes.md.
use super::*;
use crate::sim::Device;
use crate::sim_live::{self, Event, Input, Phase};
use core_video::pixel_buffer::CVPixelBuffer;
use std::collections::VecDeque;

/// Space around the bezel inside the pane, in pixels.
pub(super) const MARGIN: f32 = 12.;
/// Activity lines kept for `canopy pane read`.
const LOG: usize = 12;

/// What the screen tells its pane.
pub(super) enum SimEvent {
    Toast(String),
    /// A toast that runs a command when clicked.
    Saved { title: String, path: PathBuf },
    /// Open the quote menu with this quote.
    Quote(Value),
    /// The header (status, recording time) changed.
    Changed,
    /// Live failed: show a fresh screenshot instead.
    Fallback,
}

/// The live state.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Live {
    Off,
    Starting,
    /// Frames arrive; `input` once touch and keys reach the guest.
    On { input: bool },
    Failed(String),
}

/// Who made an activity line, for agents reading the pane.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum By { You, Agent }

pub(super) struct Recording {
    child: std::process::Child,
    /// Closing it (stop, quit or crash) makes the wrapper SIGINT simctl, which saves the movie.
    stdin: Option<std::process::ChildStdin>,
    pub path: PathBuf,
    pub started: Instant,
    _tick: Task<()>,
}

/// An annotation in progress: a still of the screen and the marks drawn on it.
pub(super) struct Annotation {
    still: image::RgbaImage,
    image: Arc<RenderImage>,
    /// Strokes in 0..1 screen coordinates.
    strokes: Vec<Vec<[f32; 2]>>,
    drawing: bool,
}

pub(super) struct SimScreen {
    pub pane: String,
    pub device: Device,
    pub info: Option<sim_live::DeviceInfo>,
    /// The latest simctl screenshot: the resting and fallback picture.
    shot: Option<Arc<RenderImage>>,
    surface: Option<CVPixelBuffer>,
    helper: Option<sim_live::Helper>,
    pub live: Live,
    /// Shown on the canvas or in a pop-out window: only then does a helper run.
    pub shown: bool,
    /// An agent's input keeps the helper running until then, shown or not.
    agent_until: Option<Instant>,
    bounds: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    touching: bool,
    scrolling: Option<[f64; 2]>,
    focus: FocusHandle,
    pub log: VecDeque<String>,
    typed: Option<(Instant, String)>,
    pub recording: Option<Recording>,
    pub annotation: Option<Annotation>,
    pub popped: bool,
    /// The running helper's events; dropping it drops a stopped helper's late events.
    _pump: Option<Task<()>>,
    /// Agent input, each call after the one before it.
    _agent: Option<Task<()>>,
    /// Stops a helper an agent started, once agents go quiet.
    _agent_idle: Option<Task<()>>,
    _tasks: Vec<Task<()>>,
}

impl EventEmitter<SimEvent> for SimScreen {}

impl SimScreen {
    pub fn new(pane: String, device: Device, cx: &mut Context<Self>) -> Self {
        let device_type = device.device_type.clone();
        // shortcut: the bezel's corner fill takes the theme's canvas color now; a theme switch keeps the old fill until the pane reopens.
        let bg = Rgba::from(c("canvas"));
        let fill = ((bg.r * 255.).round() as u32) << 16 | ((bg.g * 255.).round() as u32) << 8 | (bg.b * 255.).round() as u32;
        let info = cx.spawn(async move |this, cx| {
            let info = cx.background_executor().spawn(async move { sim_live::device_info(&device_type, 3., fill) }).await;
            let _ = this.update(cx, |this, cx| { this.info = info; cx.notify(); });
        });
        Self {
            pane, device, info: None, shot: None, surface: None, helper: None, live: Live::Off, shown: false, agent_until: None,
            bounds: Rc::default(), touching: false, scrolling: None, focus: cx.focus_handle(), log: VecDeque::new(), typed: None,
            recording: None, annotation: None, popped: false, _pump: None, _agent: None, _agent_idle: None, _tasks: vec![info],
        }
    }

    /// The pane's latest screenshot; the pane frees its texture.
    pub fn set_shot(&mut self, shot: Option<Arc<RenderImage>>, cx: &mut Context<Self>) {
        self.shot = shot;
        cx.notify();
    }

    /// Starts or stops the helper to match `shown` (and recent agent input). A failed helper
    /// stays down until `retry`.
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        let want = (self.shown || self.agent_until.is_some_and(|t| t > Instant::now())) && self.device.booted;
        match (&self.live, want) {
            (Live::Off, true) => self.start(cx),
            (Live::Starting | Live::On { .. }, false) => self.stop(cx),
            _ => {}
        }
    }

    /// Tries live again after a failure.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if matches!(self.live, Live::Failed(_)) { self.live = Live::Off; }
        self.sync(cx);
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        let (tx, rx) = async_channel::unbounded();
        match sim_live::Helper::start(&self.device.udid, tx) {
            Ok(helper) => {
                let frame = helper.frame.clone();
                self.helper = Some(helper);
                self.live = Live::Starting;
                self._pump = Some(cx.spawn(async move |this, cx| {
                    while let Ok(event) = rx.recv().await {
                        if let Event::Frame = event {
                            frame.store(false, std::sync::atomic::Ordering::Release);
                        }
                        if this.update(cx, |this, cx| this.event(event, cx)).is_err() { break; }
                    }
                }));
            }
            Err(error) => self.fail(error, cx),
        }
        cx.emit(SimEvent::Changed);
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        // Dropping the helper closes its stdin and it exits; its last events go nowhere.
        self.helper = None;
        self._pump = None;
        self.live = Live::Off;
        self.touching = false;
        cx.emit(SimEvent::Changed);
        cx.notify();
    }

    fn fail(&mut self, error: String, cx: &mut Context<Self>) {
        self.helper = None;
        self._pump = None;
        self.surface = None;
        self.live = Live::Failed(error.clone());
        cx.emit(SimEvent::Toast(format!("Live simulator: {error}. Showing screenshots.")));
        cx.emit(SimEvent::Fallback);
        cx.emit(SimEvent::Changed);
        cx.notify();
    }

    fn event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Frame => cx.notify(),
            Event::Surface(surface) => {
                self.surface = Some(surface.take());
                if self.live == Live::Starting { self.live = Live::On { input: false }; cx.emit(SimEvent::Changed); }
                cx.notify();
            }
            Event::Input => {
                self.live = Live::On { input: true };
                cx.emit(SimEvent::Changed);
            }
            Event::Warn(text) => cx.emit(SimEvent::Toast(text)),
            Event::Exit(reason) => self.fail(reason.unwrap_or_else(|| "the helper stopped".into()), cx),
        }
    }

    fn input_ready(&self) -> bool { self.live == Live::On { input: true } }

    fn send(&mut self, input: Input) {
        if let Some(helper) = &mut self.helper { helper.send(input); }
    }

    /// Adds an activity line for `canopy pane read`.
    pub fn note(&mut self, by: By, what: String) {
        self.flush_typing();
        let who = match by { By::You => "you", By::Agent => "agent" };
        self.log.push_back(format!("{} {who} {what}", clock()));
        while self.log.len() > LOG { self.log.pop_front(); }
    }

    /// A run of typing becomes one line once the person pauses or does something else.
    fn flush_typing(&mut self) {
        if let Some((_, text)) = self.typed.take() {
            self.log.push_back(format!("{} you typed {text:?}", clock()));
            while self.log.len() > LOG { self.log.pop_front(); }
        }
    }

    fn typed(&mut self, text: &str) {
        match &mut self.typed {
            Some((at, run)) if at.elapsed() < Duration::from_secs(5) => { run.push_str(text); *at = Instant::now(); }
            _ => { self.flush_typing(); self.typed = Some((Instant::now(), text.to_owned())); }
        }
    }

    /// The screen's size in points (portrait), once known.
    pub fn points(&self) -> Option<[f32; 2]> {
        self.info.as_ref().map(|i| i.points).or_else(|| self.surface.as_ref().map(|s| [s.get_width() as f32 / 3., s.get_height() as f32 / 3.]))
    }

    fn fit(&self, room: [f32; 2]) -> Option<sim_live::Fit> {
        let points = self.points()?;
        Some(sim_live::fit(room, points, self.info.as_ref().and_then(|i| i.chrome.as_ref()), MARGIN))
    }

    /// The scale the device is drawn at now, in pixels per point.
    pub fn scale(&self) -> Option<f32> {
        let b = self.bounds.get()?;
        Some(self.fit([f32::from(b.size.width), f32::from(b.size.height)])?.scale)
    }

    /// A window position as 0..1 of the screen, or None outside it.
    fn to_screen(&self, at: Point<Pixels>) -> Option<[f64; 2]> {
        let b = self.bounds.get()?;
        let fit = self.fit([f32::from(b.size.width), f32::from(b.size.height)])?;
        fit.normalized([f32::from(at.x - b.origin.x), f32::from(at.y - b.origin.y)])
    }

    /// Like `to_screen`, clamped to the screen's edge, for a drag that leaves it.
    fn to_screen_clamped(&self, at: Point<Pixels>) -> Option<[f64; 2]> {
        let b = self.bounds.get()?;
        let fit = self.fit([f32::from(b.size.width), f32::from(b.size.height)])?;
        let [x, y, w, h] = fit.screen;
        let p = [f32::from(at.x - b.origin.x), f32::from(at.y - b.origin.y)];
        Some([((p[0] - x) / w).clamp(0., 1.) as f64, ((p[1] - y) / h).clamp(0., 1.) as f64])
    }

    fn points_of(&self, at: [f64; 2]) -> String {
        let [w, h] = self.points().unwrap_or([0., 0.]);
        format!("({}, {})", (at[0] * w as f64).round(), (at[1] * h as f64).round())
    }

    // ---- Pointer and keys ----

    fn touch_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.focus.focus(window, cx);
        let Some(at) = self.to_screen(e.position) else { return };
        if let Some(a) = &mut self.annotation {
            a.strokes.push(vec![[at[0] as f32, at[1] as f32]]);
            a.drawing = true;
            return cx.notify();
        }
        if !self.input_ready() { return self.retry(cx); }
        self.touching = true;
        self.send(Input::Touch { phase: Phase::Start, x: at[0], y: at[1] });
        let what = format!("tapped {}", self.points_of(at));
        self.note(By::You, what);
    }

    fn touch_move(&mut self, e: &MouseMoveEvent, cx: &mut Context<Self>) {
        if let Some(a) = &mut self.annotation {
            if a.drawing && let Some(at) = self.to_screen_clamped(e.position) {
                let a = self.annotation.as_mut().unwrap();
                a.strokes.last_mut().unwrap().push([at[0] as f32, at[1] as f32]);
                cx.notify();
            }
            return;
        }
        if !self.touching || e.pressed_button != Some(MouseButton::Left) { return; }
        if let Some(at) = self.to_screen_clamped(e.position) { self.send(Input::Touch { phase: Phase::Move, x: at[0], y: at[1] }); }
    }

    fn touch_up(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(a) = &mut self.annotation { a.drawing = false; return cx.notify(); }
        if !self.touching { return; }
        self.touching = false;
        if let Some(to) = self.to_screen_clamped(at) {
            self.send(Input::Touch { phase: Phase::End, x: to[0], y: to[1] });
            // A tap that moved is a swipe: say where it ended.
            let end = self.points_of(to);
            if let Some(line) = self.log.back_mut().filter(|l| l.contains(" you tapped ")) {
                if !line.ends_with(&end) { *line = format!("{} to {end}", line.replacen(" tapped ", " swiped from ", 1)); }
            }
        }
    }

    /// Two-finger scrolling on a trackpad drags the screen, as a finger would.
    fn scroll(&mut self, e: &ScrollWheelEvent, cx: &mut Context<Self>) {
        cx.stop_propagation();
        if !self.input_ready() || self.annotation.is_some() { return; }
        let ScrollDelta::Pixels(delta) = e.delta else { return };
        let Some(fit) = self.bounds.get().and_then(|b| self.fit([f32::from(b.size.width), f32::from(b.size.height)])) else { return };
        let [_, _, w, h] = fit.screen;
        match (e.touch_phase, self.scrolling) {
            // Momentum after the fingers lift arrives as Moved with no start: not a touch.
            (TouchPhase::Started, _) => {
                let Some(at) = self.to_screen(e.position) else { return };
                self.scrolling = Some(at);
                self.send(Input::Touch { phase: Phase::Start, x: at[0], y: at[1] });
                let what = format!("scrolled at {}", self.points_of(at));
                self.note(By::You, what);
            }
            (TouchPhase::Moved, Some([x, y])) => {
                let at = [(x + f64::from(f32::from(delta.x) / w)).clamp(0., 1.), (y + f64::from(f32::from(delta.y) / h)).clamp(0., 1.)];
                self.scrolling = Some(at);
                self.send(Input::Touch { phase: Phase::Move, x: at[0], y: at[1] });
            }
            (TouchPhase::Ended, Some([x, y])) => {
                self.scrolling = None;
                self.send(Input::Touch { phase: Phase::End, x, y });
            }
            _ => {}
        }
    }

    fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let k = &e.keystroke;
        if k.key == "escape" && !k.modifiers.modified() {
            // Escape hands the keyboard back to the canvas; an annotation ends first.
            if self.annotation.take().is_some() { return cx.notify(); }
            window.blur(cx);
            return cx.stop_propagation();
        }
        if !self.input_ready() { return; }
        // Command shortcuts stay the app's, except paste (typed in) and Simulator's Home.
        if k.modifiers.platform {
            if k.key == "v" && !k.modifiers.shift {
                if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) { self.type_text(&text, By::You); }
                return cx.stop_propagation();
            }
            if k.key == "h" && k.modifiers.shift { self.press("home", By::You); return cx.stop_propagation(); }
            return;
        }
        let key = (k.key_char.as_deref().and_then(|c| c.chars().next().filter(|_| c.chars().count() == 1)).and_then(sim_live::char_usage))
            .or_else(|| sim_live::named_key_usage(&k.key).map(|u| (u, false)));
        let Some((usage, shift)) = key else { return };
        let held: Vec<u32> = sim_live::MODIFIERS.iter().filter(|(name, _)| match *name {
            "shift" => shift || k.modifiers.shift, "control" => k.modifiers.control, "alt" => k.modifiers.alt, _ => false,
        }).map(|(_, u)| *u).collect();
        for &m in &held { self.send(Input::Key { usage: m, down: true }); }
        self.send(Input::Key { usage, down: true });
        self.send(Input::Key { usage, down: false });
        for &m in held.iter().rev() { self.send(Input::Key { usage: m, down: false }); }
        match k.key_char.as_deref().filter(|c| c.chars().count() == 1 && !k.modifiers.control) {
            Some(c) => self.typed(c),
            None => self.note(By::You, format!("pressed {}", k.key)),
        }
        cx.stop_propagation();
    }

    /// Types text on the simulator's hardware keyboard (US layout).
    pub fn type_text(&mut self, text: &str, by: By) -> usize {
        let (keys, skipped) = sim_live::type_text(text);
        for (usage, shift) in keys {
            if shift { self.send(Input::Key { usage: 0xe1, down: true }); }
            self.send(Input::Key { usage, down: true });
            self.send(Input::Key { usage, down: false });
            if shift { self.send(Input::Key { usage: 0xe1, down: false }); }
        }
        match by { By::You => self.typed(text), By::Agent => self.note(by, format!("typed {text:?}")) }
        skipped
    }

    /// Presses and releases a named hardware button (sim_live::BUTTONS).
    pub fn press(&mut self, name: &str, by: By) -> bool {
        let Some(&(_, page, usage)) = sim_live::BUTTONS.iter().find(|(n, ..)| *n == name) else { return false };
        self.send(Input::Button { page, usage, down: true });
        self.send(Input::Button { page, usage, down: false });
        self.note(by, format!("pressed {name}"));
        true
    }

    // ---- Agent input ----

    /// Runs an agent's input once touch reaches the guest, starting the helper if the pane is
    /// not shown; it then stays up for 15 seconds after the last input.
    pub fn agent(&mut self, steps: Vec<AgentStep>, cx: &mut Context<Self>) {
        self.agent_until = Some(Instant::now() + Duration::from_secs(15));
        if matches!(self.live, Live::Failed(_)) { self.live = Live::Off; }
        self.sync(cx);
        let before = self._agent.take();
        let run = cx.spawn(async move |this, cx| {
            if let Some(before) = before { before.await; }
            // Waits only while the helper starts; gives up after 20 s.
            let started = Instant::now();
            loop {
                let Ok(ready) = this.read_with(cx, |this, _| (this.input_ready(), matches!(this.live, Live::Failed(_)))) else { return };
                if ready.0 { break; }
                if ready.1 || started.elapsed() > Duration::from_secs(20) {
                    let _ = this.update(cx, |_, cx| cx.emit(SimEvent::Toast("An agent's input did not reach the simulator".into())));
                    return;
                }
                cx.background_executor().timer(Duration::from_millis(100)).await;
            }
            for step in steps {
                let _ = this.update(cx, |this, _| match step {
                    AgentStep::Input(input) => this.send(input),
                    AgentStep::Note(line) => this.note(By::Agent, line),
                    AgentStep::Type(text) => { this.type_text(&text, By::Agent); }
                    AgentStep::Press(name) => { this.press(&name, By::Agent); }
                });
            }
        });
        self._agent = Some(run);
        // Stop a helper the agent started once nothing has asked for it for a while.
        self._agent_idle = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(16)).await;
            let _ = this.update(cx, |this, cx| this.sync(cx));
        }));
    }

    // ---- Screenshot, recording, annotation ----

    /// Saves the screen to the Desktop as Simulator does, from the live framebuffer.
    pub fn screenshot(&mut self, by: By, cx: &mut Context<Self>) -> Result<PathBuf, String> {
        let surface = self.surface.as_ref().filter(|_| matches!(self.live, Live::On { .. })).ok_or("The simulator is not live: refresh the pane for a screenshot")?;
        let path = desktop().join(format!("Simulator Screenshot - {} - {}.png", self.device.name, stamp()));
        sim_live::save_png(surface, &path)?;
        self.note(by, format!("saved a screenshot to {}", path.display()));
        cx.emit(SimEvent::Saved { title: "Saved a screenshot".into(), path: path.clone() });
        Ok(path)
    }

    /// The live screen as a PNG at `path`, for `canopy pane read`; false when not live.
    pub fn save_now(&self, path: &std::path::Path) -> bool {
        self.surface.as_ref().filter(|_| matches!(self.live, Live::On { .. })).is_some_and(|s| sim_live::save_png(s, path).is_ok())
    }

    /// Starts recording the screen to the Desktop with `simctl io recordVideo`, or stops and
    /// saves a running recording.
    pub fn record(&mut self, by: By, cx: &mut Context<Self>) -> Result<Option<PathBuf>, String> {
        if let Some(mut recording) = self.recording.take() {
            recording.stdin = None;
            let path = recording.path.clone();
            let elapsed = recording.started.elapsed().as_secs();
            self.note(by, format!("recorded {}:{:02} to {}", elapsed / 60, elapsed % 60, path.display()));
            self._tasks.push(cx.spawn(async move |this, cx| {
                let done = cx.background_executor().spawn(async move { recording.child.wait().map(|s| s.success()) }).await;
                let _ = this.update(cx, |_, cx| match done {
                    Ok(true) => cx.emit(SimEvent::Saved { title: "Saved a screen recording".into(), path }),
                    _ => cx.emit(SimEvent::Toast("The screen recording could not be saved".into())),
                });
            }));
            cx.emit(SimEvent::Changed);
            return Ok(None);
        }
        if !self.device.booted { return Err("Boot the simulator to record it".into()); }
        let path = desktop().join(format!("Simulator Screen Recording - {} - {}.mov", self.device.name, stamp()));
        // SIGINT is how simctl finishes the movie. The wrapper sends it when its stdin closes,
        // so a recording also ends cleanly if Canopy quits or crashes.
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", r#"xcrun simctl io "$0" recordVideo --codec=h264 --force "$1" & p=$!; read _; kill -INT $p; wait $p"#, &self.device.udid])
            .arg(&path)
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
            .spawn().map_err(|e| format!("Can't start recording: {e}"))?;
        let stdin = child.stdin.take();
        // The header shows the time: a tick runs only while recording.
        let tick = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            if this.update(cx, |_, cx| cx.emit(SimEvent::Changed)).is_err() { break; }
        });
        self.recording = Some(Recording { child, stdin, path: path.clone(), started: Instant::now(), _tick: tick });
        self.note(by, format!("started recording to {}", path.display()));
        cx.emit(SimEvent::Changed);
        Ok(Some(path))
    }

    /// Starts annotating a still of the screen, or cancels the annotation.
    pub fn annotate(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.annotation.take().is_some() { cx.notify(); return Ok(()); }
        let still = self.surface.as_ref().filter(|_| matches!(self.live, Live::On { .. })).and_then(sim_live::rgba)
            .ok_or("The simulator is not live: nothing to annotate")?;
        let mut bgra = still.clone();
        for p in bgra.pixels_mut() { p.0.swap(0, 2); }
        let image = Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(bgra)]));
        self.annotation = Some(Annotation { still, image, strokes: vec![], drawing: false });
        cx.notify();
        Ok(())
    }

    /// Burns the marks into the still, saves it and offers it to an agent as a quote.
    fn send_annotation(&mut self, cx: &mut Context<Self>) {
        let Some(mut a) = self.annotation.take() else { return };
        let (w, h) = (a.still.width() as f32, a.still.height() as f32);
        // The marks are as thick on the image as on screen.
        let radius = (STROKE / 2. / self.scale().unwrap_or(1.) * self.info.as_ref().map_or(3., |i| i.scale)).max(2.);
        for stroke in &a.strokes { burn(&mut a.still, stroke.iter().map(|p| [p[0] * w, p[1] * h]).collect(), radius); }
        let path = std::env::temp_dir().join("canopy").join(format!("sim-{}-annotated-{}.png", self.pane, stamp().replace(' ', "-")));
        let saved = std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string()).and_then(|_| a.still.save(&path).map_err(|e| e.to_string()));
        cx.defer(move |cx| cx.drop_image(a.image, None));
        if let Err(e) = saved { return cx.emit(SimEvent::Toast(format!("Can't save the annotation: {e}"))); }
        let [pw, ph] = self.points().unwrap_or_default();
        self.note(By::You, format!("annotated the screen ({} marks): {}", a.strokes.len(), path.display()));
        let text = format!("Annotated screenshot of {} ({}), {}x{} points. The red marks show what to look at.", self.device.name, self.device.runtime, pw, ph);
        cx.emit(SimEvent::Quote(json!({"path": path, "text": text, "pane": self.pane})));
        cx.notify();
    }

    /// The header's status after the runtime: Live, Recording 0:12, Screenshot...
    pub fn status(&self) -> String {
        if let Some(r) = &self.recording {
            let s = r.started.elapsed().as_secs();
            return format!("Recording {}:{:02}", s / 60, s % 60);
        }
        match &self.live {
            Live::On { .. } => "Live".into(),
            Live::Starting => "Connecting".into(),
            Live::Failed(_) | Live::Off => "Screenshot".into(),
        }
    }
}

/// One step of an agent's input.
#[derive(Debug)]
pub(super) enum AgentStep {
    Input(Input),
    Note(String),
    Type(String),
    Press(String),
}

/// Annotation marks: this wide on screen, in pixels.
const STROKE: f32 = 4.;
const MARK: Rgba = Rgba { r: 1., g: 0.23, b: 0.19, a: 1. };

/// Draws a polyline into an image as round dots along each segment.
fn burn(image: &mut image::RgbaImage, points: Vec<[f32; 2]>, radius: f32) {
    if points.is_empty() { return; }
    let mark = image::Rgba([255, 59, 48, 255]);
    let (w, h) = (image.width() as i32, image.height() as i32);
    let mut dot = |c: [f32; 2]| {
        let r = radius.ceil() as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if (dx * dx + dy * dy) as f32 > radius * radius { continue; }
                let (x, y) = (c[0] as i32 + dx, c[1] as i32 + dy);
                if (0..w).contains(&x) && (0..h).contains(&y) { image.put_pixel(x as u32, y as u32, mark); }
            }
        }
    };
    for pair in points.windows(2).chain(std::iter::once(&points[points.len().saturating_sub(1)..])) {
        let (a, b) = (pair[0], *pair.last().unwrap());
        let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) / (radius / 2.).max(1.)).ceil().max(1.) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            dot([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
}

fn desktop() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let desktop = home.join("Desktop");
    if desktop.is_dir() { desktop } else { home }
}

/// Local time as "2026-11-18 at 10.16.05", Simulator's file name style.
fn stamp() -> String { local("%Y-%m-%d at %H.%M.%S") }

/// Local time as "10:16:05", for activity lines.
fn clock() -> String { local("%H:%M:%S") }

fn local(format: &str) -> String {
    let mut buf = [0u8; 64];
    let format = std::ffi::CString::new(format).unwrap();
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let n = libc::strftime(buf.as_mut_ptr().cast(), buf.len(), format.as_ptr(), &tm);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}

impl Render for SimScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measured = self.bounds.clone();
        let entity = cx.entity();
        let measure = canvas(
            move |b, window, cx| {
                // The first layout and every resize draw once more at the measured size.
                if measured.replace(Some(b)).is_none_or(|old| old.size != b.size) {
                    entity.update(cx, |_, cx| cx.notify());
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        ).absolute().size_full();
        let mut root = div().id(SharedString::from(format!("sim-screen-{}", self.pane))).size_full().relative()
            .track_focus(&self.focus).on_key_down(cx.listener(Self::key_down))
            .child(measure);
        let room = self.bounds.get().map(|b| [f32::from(b.size.width), f32::from(b.size.height)]);
        let Some(fit) = room.and_then(|room| self.fit(room)) else {
            // Before the size is known: the screenshot alone, contain-fitted.
            return root.when_some(self.shot.clone(), |d, shot| d.child(img(shot).size_full().p(px(MARGIN)).object_fit(ObjectFit::Contain)));
        };
        let at = |r: [f32; 4]| div().absolute().left(px(r[0])).top(px(r[1])).w(px(r[2])).h(px(r[3]));
        let chrome = self.info.as_ref().and_then(|i| i.chrome.as_ref());
        // Hardware buttons sit behind the body; a press pushes one in and shows its pressed art.
        if let Some(chrome) = chrome {
            for button in &chrome.buttons {
                let [bx, by, bw, bh] = button.rect;
                let rect = [fit.body[0] + bx * fit.scale, fit.body[1] + by * fit.scale, bw * fit.scale, bh * fit.scale];
                let push = button.press * fit.scale * if bx < 0. { 1. } else { -1. };
                let (page, usage, name) = (button.page, button.usage, button.name.clone());
                let group = SharedString::from(format!("sim-{}-{}", self.pane, button.name));
                root = root.child(at(rect).id(group.clone()).group(group.clone()).cursor_pointer()
                    .tooltip(strip_tooltip(button.label.clone(), None))
                    .child(img(button.image.clone()).size_full().group_active(group.clone(), |s| s.left(px(push))))
                    .child(img(button.down.clone()).absolute().top_0().left(px(push)).size_full().opacity(0.).group_active(group, |s| s.opacity(1.)))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.send(Input::Button { page, usage, down: true });
                        this.note(By::You, format!("pressed {name}"));
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, _: &MouseUpEvent, _, _| this.send(Input::Button { page, usage, down: false })))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(move |this, _: &MouseUpEvent, _, _| this.send(Input::Button { page, usage, down: false }))));
            }
        }
        // The screen: an annotation still, the live framebuffer, else the last screenshot.
        let screen = at(fit.screen).bg(gpui::black());
        root = root.child(match (&self.annotation, &self.surface, &self.shot) {
            (Some(a), ..) => screen.child(img(a.image.clone()).size_full()),
            (None, Some(surface), _) if matches!(self.live, Live::On { .. }) => screen.child(gpui::surface(surface.clone()).size_full()),
            (None, _, Some(shot)) => screen.child(img(shot.clone()).size_full()),
            _ => screen,
        });
        if let Some(chrome) = chrome { root = root.child(img(chrome.body.clone()).absolute().left(px(fit.body[0])).top(px(fit.body[1])).w(px(fit.body[2])).h(px(fit.body[3]))); }
        if let Some(a) = &self.annotation {
            let strokes = a.strokes.clone();
            let [sx, sy, sw, sh] = fit.screen;
            root = root.child(canvas(|_, _, _| {}, move |b, _, window, _| {
                for stroke in &strokes {
                    let mut path = gpui::PathBuilder::stroke(px(STROKE));
                    let p = |q: &[f32; 2]| point(b.origin.x + px(sx + q[0] * sw), b.origin.y + px(sy + q[1] * sh));
                    path.move_to(p(&stroke[0]));
                    if stroke.len() == 1 { path.line_to(p(&stroke[0]) + point(px(0.5), px(0.5))); }
                    for q in &stroke[1..] { path.line_to(p(q)); }
                    if let Ok(path) = path.build() { window.paint_path(path, MARK); }
                }
            }).absolute().size_full());
        }
        // Touch lands on the screen only; a drag keeps going outside it.
        let pane = self.pane.clone();
        root = root.child(at(fit.screen).id(SharedString::from(format!("sim-{pane}-touch")))
            .cursor(if self.annotation.is_some() { CursorStyle::Crosshair } else { CursorStyle::Arrow })
            .on_mouse_down(MouseButton::Left, cx.listener(Self::touch_down))
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| this.scroll(e, cx))))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| this.touch_move(e, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, e: &MouseUpEvent, _, cx| this.touch_up(e.position, cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, e: &MouseUpEvent, _, cx| this.touch_up(e.position, cx)));
        if let Some(a) = &self.annotation {
            let marks = a.strokes.len();
            let button = |id: &'static str, label: &'static str, primary: bool| div().id(id).px(px(10.)).py(px(4.)).rounded(px(ui::RADIUS_SM)).cursor_pointer()
                .when(primary, |d| d.bg(c("foreground")).text_color(c("background"))).when(!primary, |d| d.hover(|d| d.bg(c("hover"))))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(label);
            root = root.child(div().absolute().bottom(px(MARGIN)).left_0().right_0().flex().justify_center()
                .child(div().flex().items_center().gap(px(4.)).p(px(4.)).rounded(px(ui::RADIUS)).bg(c("popover")).border_1().border_color(c("border")).shadow_md().text_size(px(12.))
                    .child(div().px(px(6.)).text_color(c("muted-foreground")).child(if marks == 0 { "Draw on the screen".to_owned() } else { format!("{marks} mark{}", if marks == 1 { "" } else { "s" }) }))
                    .child(button("sim-annotate-clear", "Clear", false).on_click(cx.listener(|this, _, _, cx| { if let Some(a) = &mut this.annotation { a.strokes.clear(); } cx.notify(); })))
                    .child(button("sim-annotate-cancel", "Cancel", false).on_click(cx.listener(|this, _, _, cx| { let _ = this.annotate(cx); })))
                    .child(button("sim-annotate-send", "Send to agent", true).on_click(cx.listener(|this, _, _, cx| this.send_annotation(cx))))));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn burn_draws_a_thick_line() {
        let mut image = image::RgbaImage::new(20, 10);
        super::burn(&mut image, vec![[2., 5.], [17., 5.]], 1.5);
        assert_eq!(image.get_pixel(10, 5).0, [255, 59, 48, 255]);
        assert_eq!(image.get_pixel(10, 6).0, [255, 59, 48, 255]);
        assert_eq!(image.get_pixel(10, 9).0, [0, 0, 0, 0]);
        // A single dot is still a mark.
        let mut image = image::RgbaImage::new(5, 5);
        super::burn(&mut image, vec![[2., 2.]], 1.);
        assert_eq!(image.get_pixel(2, 2).0, [255, 59, 48, 255]);
    }
}
