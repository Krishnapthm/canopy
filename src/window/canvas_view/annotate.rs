//! The Annotate tool shared by browser and simulator panes: a still, marks and numbered comments.
use super::*;
use std::fmt::Write as _;

type Pos = [f32; 2];
type Rect = [f32; 4];
type Color = [u8; 4];
const COLORS: [(&str, Color); 5] = [
    ("Red", [239, 68, 68, 255]),
    ("Orange", [249, 115, 22, 255]),
    ("Yellow", [234, 179, 8, 255]),
    ("Green", [34, 197, 94, 255]),
    ("Blue", [59, 130, 246, 255]),
];
const PIN: Color = [37, 99, 235, 255];
/// Ends a delivery error once the text is in the agent's prompt: the annotation is then closed.
#[cfg(target_os = "macos")]
pub(super) const NOT_SUBMITTED: &str = "The annotation is in the agent's prompt, not submitted";
const STYLES: [&str; 11] = ["display", "position", "width", "height", "margin", "padding", "font-size", "font-weight", "text-align", "color", "background-color"];
/// The stroke widths the bar offers for the pen and box tools, as (label, width).
const STROKES: [(&str, f32); 3] = [("Thin", 2.), ("Medium", 4.), ("Thick", 8.)];

pub(super) enum Host {
    Browser { url: String, viewport: Option<Pos> },
    Sim { device: String, runtime: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tool { Select, Pen, Box, Text }

#[derive(Clone)]
pub(super) enum Mark {
    Pen { points: Vec<Pos>, color: Color, width: f32 },
    Box { rect: Rect, color: Color, width: f32 },
    Text { at: Pos, text: String, color: Color },
}

#[derive(Clone)]
pub(super) enum Anchor {
    Element(Value),
    Area(Rect),
    Text { text: String, selector: String, rects: Vec<Rect> },
    Point(Pos),
}

#[derive(Clone)]
pub(super) struct Comment {
    pub anchor: Anchor,
    pub text: String,
}

pub(super) enum Action {
    Mark(Mark),
    Comment(Comment),
    Edit { original: usize, comment: Option<Comment> },
}

pub(super) struct Annotate {
    pub pane: String,
    pub host: Host,
    pub still: image::RgbaImage,
    pub image: Arc<RenderImage>,
    pub page_size: Pos,
    pub done: Vec<Action>,
    pub undone: Vec<Action>,
    pub tool: Tool,
    pub color: Color,
    pub width: f32,
    pub live: bool,
    drag: Option<Gesture>,
    pub editing: Option<usize>,
    draft: Option<Draft>,
    measured: HashMap<String, Option<Rect>>,
    focus: FocusHandle,
    session: Rc<()>,
    was_live: bool,
    refreshing: bool,
    hint: bool,
    /// The agent the person picked next to the Annotate button; unused while exactly one is wired.
    target: Option<String>,
    /// The element under the pointer while Select is active: its page box and short name.
    hover: Option<(Rect, String)>,
    probe: Probe,
    confirming: Option<String>,
    outgoing: Option<Outgoing>,
    #[cfg(target_os = "macos")]
    held: Option<Held>,
}

/// The hover query to the page: at most one in flight, and the latest point waits behind it.
enum Probe {
    Idle,
    Asking,
    Queued(Pos),
    /// The pointer left while a query was in flight. Its response is stale.
    Away,
}

#[cfg(any(target_os = "macos", test))]
impl Probe {
    /// Requests a hover query, or queues the point behind the in-flight query.
    fn request(&mut self, point: Pos) -> bool {
        if matches!(self, Self::Idle) {
            *self = Self::Asking;
            true
        } else {
            *self = Self::Queued(point);
            false
        }
    }

    fn leave(&mut self) {
        if !matches!(self, Self::Idle) { *self = Self::Away; }
    }
}

struct Gesture {
    start: Pos,
    points: Vec<Pos>,
}

enum DraftKind {
    Comment { anchor: Anchor },
    Label { color: Color },
}

/// The open composer: a pill input at a point, with the Add/Send menu opened by hovering its button.
struct Draft {
    at: Pos,
    kind: DraftKind,
    input: Entity<gpui_base::input::TextareaState>,
    menu: bool,
    _enter: Subscription,
}

#[derive(Clone)]
pub(super) struct Outgoing {
    pub text: String,
    pub images: Vec<String>,
    pub sending: bool,
}

enum Item<'a> {
    Mark(&'a Mark),
    Comment(usize, &'a Comment),
}

impl Annotate {
    fn items(&self) -> Vec<Item<'_>> {
        let mut items = Vec::new();
        for (id, action) in self.done.iter().enumerate() {
            match action {
                Action::Mark(mark) => items.push(Item::Mark(mark)),
                Action::Comment(comment) => items.push(Item::Comment(id, comment)),
                Action::Edit { original, comment } => {
                    if let Some(index) = items.iter().position(|item| matches!(item, Item::Comment(id, _) if id == original)) {
                        if let Some(comment) = comment { items[index] = Item::Comment(*original, comment); }
                        else { items.remove(index); }
                    }
                }
            }
        }
        items
    }

    fn push(&mut self, action: Action) {
        self.outgoing = None;
        self.undone.clear();
        self.done.push(action);
    }

    fn undo(&mut self) {
        if let Some(action) = self.done.pop() {
            self.outgoing = None;
            self.undone.push(action);
        }
    }

    fn redo(&mut self) {
        if let Some(action) = self.undone.pop() {
            self.outgoing = None;
            self.done.push(action);
        }
    }

    fn anchor_rects(&self, anchor: &Anchor) -> Vec<Rect> {
        match anchor {
            Anchor::Element(el) => self.measured.get(el["selector"].as_str().unwrap_or_default())
                .copied().unwrap_or_else(|| numbers(&el["rect"])).into_iter().collect(),
            Anchor::Area(rect) => vec![*rect],
            Anchor::Text { rects, .. } => rects.clone(),
            Anchor::Point([x, y]) => vec![[*x, *y, 0., 0.]],
        }
    }
}

fn numbers<const N: usize>(value: &Value) -> Option<[f32; N]> {
    let array = value.as_array()?;
    if array.len() != N { return None; }
    let mut out = [0.; N];
    for (out, value) in out.iter_mut().zip(array) {
        *out = value.as_f64()? as f32;
        if !out.is_finite() || out.abs() > 1_000_000. { return None; }
    }
    Some(out)
}

fn page_string(v: &Value, limit: usize) -> String {
    v.as_str().unwrap_or_default().chars().filter(|c| !c.is_control()).take(limit).collect()
}

fn tidy_element(value: &Value) -> Option<Anchor> {
    let selector = page_string(&value["selector"], 1024);
    let rect: Rect = numbers(&value["rect"])?;
    if selector.is_empty() { return None; }
    let styles: serde_json::Map<_, _> = STYLES.into_iter().filter_map(|key| {
        let style = page_string(&value["styles"][key], 100);
        (!style.is_empty()).then(|| (key.to_owned(), json!(style)))
    }).collect();
    Some(Anchor::Element(json!({
        "selector": selector, "rect": rect,
        "tag": page_string(&value["tag"], 80), "text": page_string(&value["text"], 120),
        "html": page_string(&value["html"], 300), "styles": styles,
        "component": page_string(&value["component"], 200), "file": page_string(&value["file"], 500),
        "line": value["line"].as_u64(), "column": value["column"].as_u64(),
    })))
}

fn tidy_text(value: &Value) -> Option<Anchor> {
    let text = page_string(&value["text"], 300);
    let selector = page_string(&value["selector"], 1024);
    let rects: Vec<Rect> = value["rects"].as_array()?.iter().take(512).filter_map(numbers).collect();
    (!text.is_empty() && !selector.is_empty() && !rects.is_empty()).then_some(Anchor::Text { text, selector, rects })
}

/// Contain the page in the body, returning its screen scale and top-left point.
fn fit([x, y, w, h]: Rect, [pw, ph]: Pos) -> (f32, Pos) {
    let scale = (w / pw).min(h / ph);
    (scale, [x + (w - pw * scale) / 2., y + (h - ph * scale) / 2.])
}

fn rectangle(a: Pos, b: Pos) -> Rect {
    [a[0].min(b[0]), a[1].min(b[1]), (a[0] - b[0]).abs(), (a[1] - b[1]).abs()]
}

fn message(a: &Annotate) -> String {
    let [w, h] = a.page_size;
    let mut out = match &a.host {
        Host::Browser { url, viewport: Some([w, h]) } => format!("Annotated {url} (viewport {w:.0}x{h:.0})"),
        Host::Browser { url, viewport: None } => format!("Annotated {url} (responsive viewport {w:.0}x{h:.0})"),
        Host::Sim { device, runtime } => format!("Annotated {device} ({runtime}), {w:.0}x{h:.0} points"),
    };
    out.push_str("\nImage 1 is the annotated screenshot; numbered blue dots mark the comments.");
    let mut comment_no = 0;
    let mut comments = String::new();
    let mut labels = Vec::new();
    let mut counts = [0; 2];
    for item in a.items() {
        match item {
            Item::Comment(_, comment) => {
                comment_no += 1;
                let _ = writeln!(comments, "{comment_no}. {}", comment.text);
                match &comment.anchor {
                    Anchor::Element(el) => {
                        let field = |key, limit| page_string(&el[key], limit);
                        let _ = writeln!(comments, "   Element: {} {} \"{}\"", field("selector", 1024), field("tag", 80), field("text", 120));
                        let component = field("component", 200);
                        if !component.is_empty() {
                            let _ = write!(comments, "   Component: {component}");
                            let file = field("file", 500);
                            if !file.is_empty() {
                                let _ = write!(comments, " ({file}");
                                if let Some(line) = el["line"].as_u64() { let _ = write!(comments, ":{line}"); }
                                comments.push(')');
                            }
                            comments.push('\n');
                        }
                        let styles: Vec<_> = STYLES.iter().filter_map(|&key| {
                            let value = page_string(&el["styles"][key], 100);
                            (!matches!(value.as_str(), "" | "auto" | "normal" | "0px" | "none" | "start"))
                                .then(|| format!("{key}: {value}"))
                        }).collect();
                        if !styles.is_empty() { let _ = writeln!(comments, "   Styles: {}", styles.join("; ")); }
                        let _ = writeln!(comments, "   HTML: {}", field("html", 300));
                    }
                    Anchor::Area([x, y, w, h]) => { let _ = writeln!(comments, "   Area: x {x:.0}, y {y:.0}, {w:.0} x {h:.0}"); }
                    Anchor::Text { text, selector, .. } => { let _ = writeln!(comments, "   Text: \"{text}\" in {selector}"); }
                    Anchor::Point([x, y]) => { let _ = writeln!(comments, "   Point: x {x:.0}, y {y:.0}"); }
                }
            }
            Item::Mark(Mark::Text { at: [x, y], text, .. }) => labels.push(format!("\"{text}\" at {x:.0}, {y:.0}")),
            Item::Mark(Mark::Pen { .. }) => counts[0] += 1,
            Item::Mark(Mark::Box { .. }) => counts[1] += 1,
        }
    }
    out.push_str("\n\n");
    out.push_str(&comments);
    if !labels.is_empty() { let _ = writeln!(out, "Notes on the screenshot: {}", labels.join("; ")); }
    let marks: Vec<_> = counts.into_iter().zip(["pen", "box"])
        .filter(|(count, _)| *count != 0).map(|(count, name)| format!("{count} {name}")).collect();
    if !marks.is_empty() { let _ = write!(out, "Marks drawn: {}.", marks.join(", ")); }
    out.trim_end().to_owned()
}

fn outline([x, y, w, h]: Rect) -> [Pos; 4] {
    [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
}

fn burn(mut image: image::RgbaImage, page_size: Pos, marks: &[Mark], pins: &[Pos]) -> anyhow::Result<image::RgbaImage> {
    use tiny_skia::{Paint, PathBuilder, PixmapMut, Stroke, Transform};
    let (w, h) = image.dimensions();
    let scale = w as f32 / page_size[0];
    let transform = Transform::from_scale(scale, scale);
    // tiny-skia needs premultiplied RGBA, including on pages with a transparent background.
    for image::Rgba([r, g, b, alpha]) in image.pixels_mut() {
        for c in [r, g, b] { *c = ((u16::from(*c) * u16::from(*alpha) + 127) / 255) as u8; }
    }
    let mut pix = PixmapMut::from_bytes(image.as_mut(), w, h).ok_or_else(|| anyhow::anyhow!("The screenshot has no pixels"))?;
    let paint = |[r, g, b, a]: Color| {
        let mut p = Paint { anti_alias: true, ..Paint::default() };
        p.set_color_rgba8(r, g, b, a);
        p
    };
    let path = |points: &[Pos], closed: bool| {
        let mut path = PathBuilder::new();
        let (&[x, y], rest) = points.split_first()?;
        path.move_to(x, y);
        for &[x, y] in rest { path.line_to(x, y); }
        if closed { path.close(); }
        path.finish()
    };
    let dot = |pix: &mut PixmapMut<'_>, [x, y]: Pos, radius, color| {
        if let Some(path) = PathBuilder::from_circle(x, y, radius) {
            pix.fill_path(&path, &paint(color), tiny_skia::FillRule::Winding, transform, None);
        }
    };
    for mark in marks {
        let (outline, width, color) = match mark {
            Mark::Pen { points, width, color } if points.len() == 1 => {
                dot(&mut pix, points[0], width / 2., *color);
                continue;
            }
            Mark::Pen { points, width, color } => (path(points, false), width, color),
            Mark::Box { rect, width, color } => (path(&outline(*rect), true), width, color),
            // shortcut: tiny-skia has no text, add a font renderer to burn labels if needed.
            Mark::Text { .. } => continue,
        };
        if let Some(outline) = outline {
            let stroke = Stroke { width: *width, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Stroke::default() };
            pix.stroke_path(&outline, &paint(*color), &stroke, transform, None);
        }
    }
    for &at in pins { dot(&mut pix, at, 10., PIN); }
    for image::Rgba([r, g, b, alpha]) in image.pixels_mut() {
        if *alpha == 0 || *alpha == 255 { continue; }
        for c in [r, g, b] {
            let a = u16::from(*alpha);
            *c = ((u16::from(*c) * 255 + a / 2) / a).min(255) as u8;
        }
    }
    Ok(image)
}

#[cfg(target_os = "macos")]
struct Held {
    monitor: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
    release: async_channel::Sender<()>,
    _activation: Subscription,
}

#[cfg(target_os = "macos")]
impl Held {
    fn release(&mut self) {
        if let Some(monitor) = self.monitor.take() {
            // This token came from AppKit's local monitor API on the window thread.
            unsafe { objc2_app_kit::NSEvent::removeMonitor(&monitor) };
        }
        let _ = self.release.try_send(());
    }
}

#[cfg(target_os = "macos")]
impl Drop for Held {
    fn drop(&mut self) { self.release(); }
}

impl AppWindow {
    pub(in crate::window) fn annotation_command(&mut self, name: &str, args: &Value, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (name, args, window, cx);
            self.fail("Browser and simulator annotations need macOS");
        }
        #[cfg(target_os = "macos")]
        {
            let matches = |pane: &canvas::Pane| match name {
                "canvas.browser_annotate" => pane.url().is_some(),
                "canvas.sim_annotate" => pane.sim().is_some(),
                _ => self.canvas_state.annotations.contains_key(&pane.id),
            };
            let id = if let Some(id) = args["pane"].as_str() {
                self.canvas_state.doc.panes.iter().find(|p| p.id == id && matches(p)).map(|p| p.id.clone())
            } else {
                self.canvas_state.selected.iter().rev().find(|id| self.canvas_state.doc.panes.iter().any(|p| p.id == **id && matches(p))).cloned()
            };
            let Some(id) = id else { return self.fail("Select a browser or simulator pane, or name one with pane") };
            let result = match name {
                "canvas.annotate_send" => self.send_annotation(&id, window, cx),
                "canvas.annotate_action" => self.annotation_action(&id, args["action"].as_str().unwrap_or_default(), window, cx),
                "canvas.annotate_agent" => self.annotation_agent_menu(&id, window, cx),
                _ => self.annotate_toggle(&id, window, cx),
            };
            self.answer(result.map(|()| json!({})));
        }
    }

    pub(super) fn forget_annotations(&mut self, ids: &[String], cx: &mut Context<Self>) {
        for id in ids {
            self.canvas_state.annotation_starts.remove(id);
            self.clear_annotation_bounds(id);
            if let Some(a) = self.canvas_state.annotations.remove(id) {
                cx.defer(move |cx| cx.drop_image(a.image, None));
            }
        }
    }

    fn clear_annotation_bounds(&self, id: &str) {
        let mut bounds = self.bounds.borrow_mut();
        for prefix in ["annotate-body:", "cover:annotate:", "cover:annotate-hint:"] {
            bounds.remove(&format!("{prefix}{id}"));
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn annotate_body(&self, _: &canvas::Pane, body: AnyElement, _: Pos, _: &mut Context<Self>) -> AnyElement { body }
}

/// A canvas card's body size on screen at `zoom`: the pane minus its frame and header.
pub(super) fn card_body(pane: &canvas::Pane, zoom: f32) -> Pos {
    let [w, h] = pane.pinned.map_or([pane.rect[2] * zoom, pane.rect[3] * zoom], |p| [p[2], p[3]]);
    [(w - 2.).max(1.), (h - canvas::header_height(&pane.kind) - 2.).max(1.)]
}

#[cfg(target_os = "macos")]
fn render_image(mut still: image::RgbaImage) -> Arc<RenderImage> {
    for pixel in still.pixels_mut() { pixel.0.swap(0, 2); }
    Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(still)]))
}

#[cfg(target_os = "macos")]
fn annotation_hint_file() -> PathBuf {
    mods::app_dir().join("browser").join("annotate-hint-seen")
}

#[cfg(target_os = "macos")]
type Capture = (image::RgbaImage, Pos);

#[cfg(target_os = "macos")]
impl AppWindow {
    fn annotate_toggle(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.canvas_state.annotations.contains_key(id) {
            return self.annotation_close(id, window, cx);
        }
        if self.canvas_state.annotation_starts.contains_key(id) { return Ok(()); }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).cloned().ok_or("The pane closed")?;
        // A popped-out page is annotated in its own window, whatever space the canvas shows.
        let popped = self.browser_popped(id);
        if !popped && !self.canvas_state.shows_pane(&pane) { return Err("Bring the pane into view to annotate it".into()); }
        let zoom = self.canvas_state.doc.camera.zoom;
        let width = pane.pinned.map_or(pane.rect[2] * zoom, |p| p[2]) - 2.;
        let docked = self.canvas_state.doc.docked(id);
        let needs_room = !docked && (width < 520. || zoom < 0.75);
        let url = pane.url().unwrap_or_default().to_owned();
        match pane.kind {
            canvas::Kind::Browser { viewport, .. } => {
                use wry::WebViewExtMacOS as _;
                let was_live = self.raw_browser(id).is_some_and(|raw| !raw.raw().webview().isHidden());
                let opened = self.raw_browser(id).is_none();
                if popped {
                    // The window shows the page at full size: nothing on the canvas to wake or snap.
                    if opened { self.ensure_view(id, window, cx)?; }
                } else {
                    if opened { self.wake_for_annotation(id, window, cx)?; }
                    if !docked {
                        let expanded = self.canvas_state.browser_tab.clone();
                        if let Some(expanded) = &expanded {
                            self.run_key("canvas.browser_expand", json!({"pane": expanded}), window, cx);
                        }
                        if needs_room { self.snap_usable(id, cx); }
                        if needs_room || expanded.as_deref() == Some(id) { self.wake_for_annotation(id, window, cx)?; }
                    }
                }
                let session = Rc::new(());
                self.canvas_state.annotation_starts.insert(id.to_owned(), session.clone());
                let id = id.to_owned();
                self.capture_browser(&id.clone(), session.clone(), window, cx, move |this, capture, window, cx| {
                    if !this.canvas_state.annotation_starts.get(&id).is_some_and(|s| Rc::ptr_eq(s, &session)) { return; }
                    this.canvas_state.annotation_starts.remove(&id);
                    match capture {
                        Ok(capture) => {
                            let current = this.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(canvas::Pane::url);
                            let host = Host::Browser { url: current.unwrap_or(&url).to_owned(), viewport };
                            this.begin_annotation(id, host, capture, was_live, window, cx);
                        }
                        Err(error) => {
                            if was_live { let _ = this.wake_for_annotation(&id, window, cx); }
                            this.annotation_error(error, window, cx);
                        }
                    }
                });
            }
            canvas::Kind::Sim { .. } => {
                let view = self.canvas_state.sims.get(id).ok_or("The simulator is not booted")?;
                if view.popped.is_some() { return Err("Bring the simulator back to annotate it".into()); }
                if needs_room { self.snap_usable(id, cx); }
                let (host, capture) = self.capture_sim(id, cx)?;
                self.begin_annotation(id.to_owned(), host, capture, true, window, cx);
            }
            _ => return Err("Select a browser or simulator pane to annotate".into()),
        }
        Ok(())
    }

    fn capture_sim(&self, id: &str, cx: &App) -> Result<(Host, Capture), String> {
        let screen = self.canvas_state.sims.get(id).and_then(|s| s.live.as_ref()).ok_or("The simulator is not booted")?.read(cx);
        let page = screen.points().ok_or("The simulator size is not known yet")?;
        let still = screen.annotation_still().ok_or("The simulator is not live: nothing to annotate")?;
        let host = Host::Sim { device: screen.device.name.clone(), runtime: screen.device.runtime.clone() };
        Ok((host, (still, page)))
    }

    fn wake_for_annotation(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.wake_browser_result(id, window, cx).map_err(|error| {
            if error.contains("popped out") { "Bring the page back to annotate it".into() } else { error }
        })
    }

    fn capture_browser(&mut self, id: &str, session: Rc<()>, window: &mut Window, cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, Result<Capture, String>, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let id = id.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            let deadline = Instant::now() + Duration::from_secs(2);
            let size = loop {
                let (tx, rx) = async_channel::bounded(1);
                let pending = this.update_in(cx, |this, _, _| {
                    let active = this.annotation_capture_pending(&id, &session);
                    if active {
                        this.eval_browser_json(&id,
                            "return window.__canopy && document.readyState !== 'loading' ? window.__canopy.size() : null",
                            move |value| { let _ = tx.try_send(value); });
                    }
                    active
                });
                if !matches!(pending, Ok(true)) { return; }
                let remaining = deadline.saturating_duration_since(Instant::now());
                let reply = rx.recv().with_timeout(remaining, cx.background_executor()).await.ok().and_then(Result::ok).flatten();
                if let Some(size) = reply.as_ref().and_then(numbers::<2>).filter(|p| p[0] > 0. && p[1] > 0.) {
                    break Some(size);
                }
                if Instant::now() >= deadline { break None; }
                cx.background_executor().timer(Duration::from_millis(50)).await;
            };
            let (tx, rx) = async_channel::bounded(1);
            if this.update_in(cx, |this, _, cx| {
                if this.annotation_capture_pending(&id, &session) {
                    this.freeze_browser(&id, move |_, still, _| { let _ = tx.try_send(still); }, cx);
                }
            }).is_err() { return; }
            let result = if let Some(size) = size {
                rx.recv().await.ok().flatten().map(|still| (still, size))
                    .ok_or_else(|| "WebKit could not take the annotation screenshot".into())
            } else {
                Err("The page did not answer".into())
            };
            let _ = this.update_in(cx, |this, window, cx| done(this, result, window, cx));
        }).detach();
    }

    fn begin_annotation(&mut self, pane: String, host: Host, (still, page_size): Capture, was_live: bool, window: &mut Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        if self.browser_popped(&pane) || self.annotation_pane_visible(&pane) {
            focus.focus(window, cx);
            self.canvas_state.selected = vec![pane.clone()];
            self.canvas_state.space_down = false;
        }
        let a = Annotate {
            pane: pane.clone(), host, image: render_image(still.clone()), still, page_size,
            done: vec![], undone: vec![], tool: Tool::Select, color: COLORS[0].1, width: STROKES[1].1,
            live: false, drag: None, editing: None, draft: None, measured: HashMap::new(),
            focus, session: Rc::new(()), was_live, refreshing: false, hint: !annotation_hint_file().exists(),
            target: None, hover: None, probe: Probe::Idle, confirming: None, outgoing: None, held: None,
        };
        self.canvas_state.annotations.insert(pane, a);
        cx.notify();
    }

    fn annotation_error(&mut self, title: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.toast(Toast { title: title.into(), body: None, level: Level::Error, action: None }, window, cx);
    }

    fn annotation_close(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return Ok(()) };
        if a.outgoing.as_ref().is_some_and(|o| o.sending) { return Err("The annotation is being sent".into()); }
        if a.items().is_empty() { return self.end_annotation(id, false, window, cx); }
        if a.confirming.as_ref().is_some_and(|id| self.overlays.iter().any(|o| o.id == *id)) { return Ok(()); }
        // shortcut: the dialog opens in the main window even for a popped-out page, draw it in the pop-out if that confuses.
        let overlay = mods::new_overlay_id();
        a.confirming = Some(overlay.clone());
        self.open_overlay(overlay, json!({
            "mod":"confirm", "kind":"dialog", "width":340, "props":{
                "title":"Discard your annotations?", "accept":"Discard", "cancel":"Keep editing", "destructive":true,
                "command":{"name":"canvas.annotate_action","args":{"pane":id,"action":"discard"}}
            }
        }), window, cx);
        Ok(())
    }

    pub(super) fn end_annotation(&mut self, id: &str, sent: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let Some(a) = self.canvas_state.annotations.remove(id) else { return Ok(()) };
        self.clear_annotation_bounds(id);
        let (browser, wake, live) = (matches!(a.host, Host::Browser { .. }), sent || a.was_live, a.live);
        let image = a.image.clone();
        drop(a);
        cx.defer(move |cx| cx.drop_image(image, None));
        self.focus.focus(window, cx);
        cx.notify();
        // A popped page shows again when its window redraws (`popped_body`).
        if browser && !self.browser_popped(id) {
            if wake && self.annotation_pane_visible(id) { self.wake_for_annotation(id, window, cx)?; }
            else if live { self.rest_browser(id, cx); }
        }
        Ok(())
    }

    /// A normal browser rest also ends the native Space hold.
    pub(super) fn annotation_host_rested(&mut self, id: &str) {
        if let Some(held) = self.canvas_state.annotations.get_mut(id).and_then(|a| a.held.as_mut()) { held.release(); }
    }

    fn annotation_hold(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        use objc2_app_kit::{NSEvent, NSEventMask};
        let Some(a) = self.canvas_state.annotations.get(id) else { return Ok(()) };
        if a.live || a.refreshing || a.draft.is_some() || a.outgoing.as_ref().is_some_and(|o| o.sending) { return Ok(()); }
        let browser = matches!(a.host, Host::Browser { .. });
        let session = a.session.clone();
        let (release, pressed) = async_channel::bounded(1);
        let tx = release.clone();
        let handler = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| {
            // 49 is the macOS hardware virtual key for Space.
            if unsafe { event.as_ref() }.keyCode() == 49 { let _ = tx.try_send(()); }
            event.as_ptr()
        });
        // A local monitor observes only this application's events and returns them unchanged.
        let monitor = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyUp, &handler) }
            .ok_or("Cannot watch for Space release")?;
        let tx = release.clone();
        let activation = cx.observe_window_activation(window, move |_, window, _| {
            if !window.is_window_active() { let _ = tx.try_send(()); }
        });
        let held = Held { monitor: Some(monitor), release, _activation: activation };
        if browser {
            // shortcut: in the pop-out the live page covers the bar while Space is held (no mask there), mask it if that bothers.
            if !self.browser_popped(id) { self.wake_for_annotation(id, window, cx)?; }
            if let Some(raw) = self.raw_browser(id) { let _ = raw.raw().focus_parent(); }
        }
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return Ok(()) };
        a.drag = None;
        a.probe.leave();
        a.hover = None;
        a.live = true;
        a.held = Some(held);
        let id = id.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            if pressed.recv().await.is_ok() {
                let _ = this.update_in(cx, |this, window, cx| {
                    if this.annotation_is(&id, &session) { this.annotation_release(&id, window, cx); }
                });
            }
        }).detach();
        cx.notify();
        Ok(())
    }

    fn annotation_release(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get_mut(id).filter(|a| a.held.is_some()) else { return };
        a.held.take();
        a.live = false;
        a.refreshing = true;
        a.focus.focus(window, cx);
        let sim = matches!(a.host, Host::Sim { .. });
        cx.notify();
        if sim {
            if let Some(screen) = self.canvas_state.sims.get(id).and_then(|v| v.live.as_ref()) {
                screen.update(cx, |s, _| s.touch_up(window.mouse_position()));
            }
            let result = self.capture_sim(id, cx).map(|(_, capture)| capture);
            self.replace_annotation_still(id, result, window, cx);
            return;
        }
        self.annotation_refreeze(id, window, cx);
    }

    /// Takes a fresh browser still and re-measures the commented elements, which may have moved.
    fn annotation_refreeze(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        a.refreshing = true;
        let session = a.session.clone();
        cx.notify();
        let id = id.to_owned();
        self.capture_browser(&id.clone(), session.clone(), window, cx, move |this, result, window, cx| {
            if !this.annotation_is(&id, &session) { return; }
            let success = result.is_ok();
            this.replace_annotation_still(&id, result, window, cx);
            if !success { return; }
            let a = this.canvas_state.annotations.get_mut(&id).unwrap();
            let mut selectors: Vec<_> = a.done.iter().chain(&a.undone).filter_map(|action| match action {
                Action::Comment(c) | Action::Edit { comment: Some(c), .. } => match &c.anchor {
                    Anchor::Element(el) => el["selector"].as_str().map(str::to_owned),
                    _ => None,
                },
                _ => None,
            }).collect();
            selectors.sort_unstable();
            selectors.dedup();
            if selectors.is_empty() { return; }
            a.refreshing = true;
            this.annotation_query(&id.clone(), format!("return window.__canopy.rects({})", json!(selectors)), window, cx,
                move |this, value, _, cx| {
                    let a = this.canvas_state.annotations.get_mut(&id).unwrap();
                    if let Some(values) = value.and_then(|v| v.as_array().cloned()) {
                        for (selector, value) in selectors.into_iter().zip(values) { a.measured.insert(selector, numbers(&value)); }
                    }
                    a.refreshing = false;
                    cx.notify();
                });
        });
    }

    fn annotation_capture_pending(&self, id: &str, session: &Rc<()>) -> bool {
        self.annotation_is(id, session)
            || self.canvas_state.annotation_starts.get(id).is_some_and(|s| Rc::ptr_eq(s, session))
    }

    fn annotation_is(&self, id: &str, session: &Rc<()>) -> bool {
        self.canvas_state.annotations.get(id).is_some_and(|a| Rc::ptr_eq(&a.session, session))
    }

    fn replace_annotation_still(&mut self, id: &str, result: Result<Capture, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        a.refreshing = false;
        match result {
            Ok((still, size)) => {
                let old = std::mem::replace(&mut a.image, render_image(still.clone()));
                cx.defer(move |cx| cx.drop_image(old, None));
                (a.still, a.page_size) = (still, size);
                let mut moved = false;
                if let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id)
                    && let canvas::Kind::Browser { viewport, .. } = &pane.kind
                {
                    let url = pane.url().unwrap_or_default().to_owned();
                    moved = matches!(&a.host, Host::Browser { url: old, .. } if *old != url);
                    a.host = Host::Browser { url, viewport: *viewport };
                }
                if !a.outgoing.as_ref().is_some_and(|o| o.sending) { a.outgoing = None; }
                if moved {
                    self.annotation_error("The page changed while it was live: comments keep their places on the old page", window, cx);
                }
            }
            Err(error) => self.annotation_error(error, window, cx),
        }
        cx.notify();
    }

    fn annotation_query(&self, id: &str, script: String, window: &mut Window, cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, Option<Value>, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let Some(a) = self.canvas_state.annotations.get(id) else { return };
        let (id, session) = (id.to_owned(), a.session.clone());
        let (tx, rx) = async_channel::bounded(1);
        self.eval_browser_json(&id, &script, move |value| { let _ = tx.try_send(value); });
        cx.spawn_in(window, async move |this, cx| {
            let value = rx.recv().with_timeout(Duration::from_secs(2), cx.background_executor()).await.ok().and_then(Result::ok).flatten();
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.annotation_is(&id, &session) { return; }
                if value.is_none() { this.annotation_error("The page did not answer", window, cx); }
                done(this, value, window, cx);
            });
        }).detach();
    }
}

#[cfg(target_os = "macos")]
const TOOLS: [(Tool, &str, &str, &str); 4] = [
    (Tool::Select, "select", "Select", "icons/mouse-pointer-2.svg"),
    (Tool::Pen, "pen", "Pen", "icons/pencil.svg"),
    (Tool::Box, "box", "Box", "icons/square.svg"),
    (Tool::Text, "text", "Text", "icons/type.svg"),
];

#[cfg(target_os = "macos")]
impl Gesture {
    fn end(&self) -> Pos { *self.points.last().unwrap_or(&self.start) }

    fn mark(&self, tool: Tool, color: Color, width: f32) -> Option<Mark> {
        match tool {
            Tool::Pen => Some(Mark::Pen { points: self.points.clone(), color, width }),
            Tool::Box => {
                let rect = rectangle(self.start, self.end());
                (rect[2] > 0. && rect[3] > 0.).then_some(Mark::Box { rect, color, width })
            }
            Tool::Select | Tool::Text => None,
        }
    }
}

#[cfg(target_os = "macos")]
fn swatch([r, g, b, a]: Color) -> gpui::Rgba {
    gpui::Rgba { r: r as f32 / 255., g: g as f32 / 255., b: b as f32 / 255., a: a as f32 / 255. }
}

/// An icon-only bar button.
#[cfg(target_os = "macos")]
fn annotation_button(id: impl Into<ElementId>, icon: &'static str, label: &str, active: bool) -> Stateful<Div> {
    div().id(id).role(gpui::Role::Button).aria_label(label.to_owned()).size(px(24.)).flex_none()
        .flex().items_center().justify_center().rounded(px(ui::RADIUS_SM)).cursor_pointer()
        .hover(|d| d.bg(c("hover"))).when(active, |d| d.bg(c("hover")))
        .child(svg().path(icon).size(px(14.)).text_color(c("foreground")))
        .tooltip(strip_tooltip(label.to_owned(), None))
}

#[cfg(target_os = "macos")]
fn annotation_rect([x, y, w, h]: Rect) -> Div {
    div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h))
}

/// A numbered blue pin centred on (x, y) in body pixels.
#[cfg(target_os = "macos")]
fn pin(x: f32, y: f32, size: f32, number: usize) -> Div {
    annotation_rect([x - size / 2., y - size / 2., size, size]).flex().items_center().justify_center().rounded_full()
        .bg(swatch(PIN)).text_color(gpui::white()).text_size(px(11.)).child(number.to_string())
}

#[cfg(target_os = "macos")]
fn paint_mark(mark: &Mark, origin: Point<Pixels>, scale: f32, window: &mut Window) {
    let path = |points: &[Pos], mut path: gpui::PathBuilder, closed| {
        let mut points = points.iter().map(|&[x, y]| origin + point(px(x * scale), px(y * scale)));
        path.move_to(points.next()?);
        for p in points { path.line_to(p); }
        if closed { path.close(); }
        path.build().ok()
    };
    let stroke = |width| gpui::PathBuilder::stroke(px(width * scale));
    let (path, color) = match mark {
        Mark::Pen { points, color, width } if points.len() == 1 => {
            let [x, y] = points[0];
            let size = px(width * scale);
            let bounds = Bounds::new(origin + point(px(x * scale) - size / 2., px(y * scale) - size / 2.), gpui::size(size, size));
            window.paint_quad(gpui::quad(bounds, size / 2., swatch(*color), px(0.), gpui::transparent_black(), Default::default()));
            return;
        }
        Mark::Pen { points, color, width } => (path(points, stroke(*width), false), color),
        Mark::Box { rect, color, width } => (path(&outline(*rect), stroke(*width), true), color),
        Mark::Text { .. } => return,
    };
    if let Some(path) = path { window.paint_path(path, swatch(*color)); }
}

#[cfg(target_os = "macos")]
impl AppWindow {
    /// Draws the still over the host body, or the native content while Space is held.
    /// `[w, h]` is the body's screen size: a card's (`card_body`) or the pop-out window's.
    pub(super) fn annotate_body(&self, pane: &canvas::Pane, native: AnyElement, [w, h]: Pos, cx: &mut Context<Self>) -> AnyElement {
        let id = &pane.id;
        let Some(a) = self.canvas_state.annotations.get(id) else { return native };
        let mark_key = format!("annotate-body:{id}");
        let bounds = self.bounds.clone();
        let target = id.clone();
        let mut root = div().id(SharedString::from(format!("annotate-{id}"))).relative().size_full()
            .track_focus(&a.focus)
            .child(canvas(move |b, _, _| { bounds.borrow_mut().insert(mark_key, b); }, |_, _, _, _| {}).absolute().size_full())
            .capture_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                let mods = e.keystroke.modifiers;
                let drafting = this.canvas_state.annotations.get(&target).is_some_and(|a| a.draft.is_some());
                // The composer's own input handles undo, Cmd+Enter and Space while it is open.
                let result = if mods.platform && !mods.alt && !mods.control {
                    match e.keystroke.key.as_str() {
                        "z" if !drafting => this.annotation_action(&target, if mods.shift { "redo" } else { "undo" }, window, cx),
                        "enter" if !drafting => this.send_annotation(&target, window, cx),
                        _ => return,
                    }
                } else if mods.modified() {
                    return;
                } else {
                    match e.keystroke.key.as_str() {
                        "space" if !drafting => if e.is_held { Ok(()) } else { this.annotation_hold(&target, window, cx) },
                        // The main window catches Esc first (`annotation_escape`); this serves the pop-out.
                        "escape" => if this.annotation_cancel_draft(&target, window, cx) { Ok(()) } else { this.annotation_close(&target, window, cx) },
                        _ => return,
                    }
                };
                if let Err(error) = result { this.annotation_error(error, window, cx); }
                cx.stop_propagation();
            }));
        {
            let (move_id, up_id) = (id.clone(), id.clone());
            let this = cx.weak_entity();
            root = root.child(canvas(|_, _, _| {}, move |_, _, window, _| {
                let moving = this.clone();
                let move_id = move_id.clone();
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, _, cx| {
                    if phase != gpui::DispatchPhase::Capture || e.pressed_button != Some(MouseButton::Left) { return; }
                    let _ = moving.update(cx, |this, cx| this.annotation_gesture_move(&move_id, e.position, cx));
                });
                let up_id = up_id.clone();
                let releasing = this.clone();
                window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
                    if phase != gpui::DispatchPhase::Capture || e.button != MouseButton::Left { return; }
                    let _ = releasing.update(cx, |this, cx| this.annotation_gesture_end(&up_id, e.position, window, cx));
                });
            }).absolute().size_full());
        }
        if a.live {
            root = root.child(native);
        } else {
            let (target, hovering, leaving) = (id.clone(), id.clone(), id.clone());
            root = root.child(div().id("annotation-still").absolute().size_full().bg(c("canvas"))
                .cursor(if a.tool == Tool::Select { CursorStyle::PointingHand } else { CursorStyle::Crosshair })
                .child(img(a.image.clone()).size_full().object_fit(gpui::ObjectFit::Contain))
                .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, window, cx| this.annotation_hover(&hovering, e.position, window, cx)))
                .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                    if *over { return; }
                    if let Some(a) = this.canvas_state.annotations.get_mut(&leaving) {
                        a.probe.leave();
                        if a.hover.take().is_some() { cx.notify(); }
                    }
                }))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    let body = this.bounds.borrow().get(&format!("annotate-body:{target}")).copied();
                    if let Some(a) = this.canvas_state.annotations.get_mut(&target)
                        && !a.refreshing && a.draft.is_none() && !a.outgoing.as_ref().is_some_and(|o| o.sending)
                        && let Some((start, _)) = annotation_point(body, a.page_size, e.position, false)
                    {
                        a.focus.focus(window, cx);
                        a.drag = Some(Gesture { start, points: vec![start] });
                        cx.notify();
                    }
                    cx.stop_propagation();
                })));
            let (scale, [ox, oy]) = fit([0., 0., w, h], a.page_size);
            let screen = |[x, y]: Pos| [ox + x * scale, oy + y * scale];
            let box_at = |[x, y, w, h]: Rect| {
                let [x, y] = screen([x, y]);
                annotation_rect([x, y, w * scale, h * scale])
            };
            let mut marks = Vec::new();
            let mut pins = 0;
            for item in a.items() {
                match item {
                    Item::Mark(Mark::Text { at, text, color }) => {
                        let [x, y] = screen(*at);
                        root = root.child(div().absolute().left(px(x)).top(px(y)).text_size(px((14. * scale).max(9.)))
                            .text_color(swatch(*color)).child(text.clone()));
                    }
                    Item::Mark(mark) => marks.push(mark.clone()),
                    Item::Comment(index, comment) => {
                        pins += 1;
                        let rects = a.anchor_rects(&comment.anchor);
                        if matches!(comment.anchor, Anchor::Text { .. } | Anchor::Area(_)) {
                            for &r in &rects { root = root.child(box_at(r).bg(gpui::rgba(0x2563eb38))); }
                        }
                        let Some(&[x, y, ..]) = rects.first() else { continue };
                        if !(0. ..=a.page_size[0]).contains(&x) || !(0. ..=a.page_size[1]).contains(&y) { continue; }
                        let [x, y] = screen([x, y]);
                        let target = id.clone();
                        root = root.child(pin(x, y, (20. * scale).max(16.), pins)
                            .id(("annotation-pin", index)).role(gpui::Role::Button).aria_label(format!("Comment {pins}")).cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, window, cx| this.annotation_edit(&target, index, window, cx))));
                    }
                }
            }
            if let Some(mark) = a.drag.as_ref().and_then(|drag| drag.mark(a.tool, a.color, a.width)) { marks.push(mark); }
            root = root.child(canvas(|_, _, _| {}, move |b, _, window, _| {
                let origin = b.origin + point(px(ox), px(oy));
                for mark in &marks { paint_mark(mark, origin, scale, window); }
            }).absolute().size_full());
            if a.tool == Tool::Select {
                if let Some(drag) = &a.drag {
                    let area = rectangle(drag.start, drag.end());
                    if area[2] > 0. && area[3] > 0. {
                        root = root.child(box_at(area).border_1().border_color(swatch(PIN)).bg(gpui::rgba(0x2563eb26)));
                    }
                } else if a.draft.is_none() && let Some((rect, label)) = &a.hover {
                    // The element the next click picks, with its short name above the box.
                    let [x, y] = screen([rect[0], rect[1]]);
                    root = root.child(box_at(*rect).border_1().border_color(swatch(PIN)).bg(gpui::rgba(0x2563eb1f)))
                        .child(div().absolute().left(px(x)).top(px(if y >= 18. { y - 18. } else { y + 2. })).h(px(16.)).px(px(5.))
                            .flex().items_center().rounded(px(ui::RADIUS_SM)).bg(swatch(PIN)).text_color(gpui::white())
                            .text_size(px(10.)).child(label.clone()));
                }
            }
            if let Some(draft) = &a.draft {
                let mut at = screen(draft.at);
                if let DraftKind::Comment { anchor } = &draft.kind {
                    let rects = a.anchor_rects(anchor);
                    for rect in rects.iter().copied().filter(|r| r[2] > 0. && r[3] > 0.) {
                        root = root.child(box_at(rect).border_1().border_color(swatch(PIN)).bg(gpui::rgba(0x2563eb38)));
                    }
                    // A new comment drops its numbered pin as soon as the element is picked.
                    if a.editing.is_none() && let Some(&[x, y, ..]) = rects.first() {
                        at = screen([x, y]);
                        root = root.child(pin(at[0], at[1], (20. * scale).max(16.), pins + 1));
                    }
                }
                root = root.child(self.annotation_composer(id, draft, at, [w, h], cx));
            }
        }
        root.child(div().absolute().bottom(px(12.)).left_0().w_full().flex().flex_col().items_center().gap(px(8.))
            .when(a.hint, |controls| controls.child(self.annotation_hint(id, a, w, cx)))
            .child(self.annotation_bar(id, a, cx)))
            .into_any_element()
    }

    /// Swaps the component file the page reported (a dev-server URL or a bare name) for its path on
    /// disk when it can be found: under the folders of agents wired to the pane, the pane's space,
    /// then every space. Otherwise the page's own value stays.
    fn resolve_component_file(&self, id: &str, mut anchor: Anchor) -> Anchor {
        if let Anchor::Element(el) = &mut anchor && let Some(file) = el["file"].as_str().filter(|f| !f.is_empty()) {
            let wires = crate::wire::all(&self.canvas_state.doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
            let mut roots: Vec<PathBuf> = crate::wire::neighbors(&wires, id).iter().filter_map(|t| self.cwds.get(t)).map(PathBuf::from).collect();
            let space = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.space.as_deref()).and_then(|s| self.model.space(s));
            roots.extend(space.map(|s| PathBuf::from(&s.folder)));
            roots.extend(self.model.spaces.iter().map(|s| PathBuf::from(&s.folder)));
            if let Some(path) = super::browser::resolve_source(file, &roots) { el["file"] = json!(path); }
        }
        anchor
    }

    /// The pointer moved over the still with Select on: asks the page which element is there and
    /// shows its box. One query at a time; a move that arrives meanwhile waits behind it.
    fn annotation_hover(&mut self, id: &str, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.bounds.borrow().get(&format!("annotate-body:{id}")).copied();
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        if !matches!(a.host, Host::Browser { .. }) || a.tool != Tool::Select || a.draft.is_some() || a.drag.is_some()
            || a.refreshing || a.live || a.outgoing.as_ref().is_some_and(|o| o.sending) { return; }
        let Some((point, _)) = annotation_point(body, a.page_size, at, false) else {
            a.probe.leave();
            if a.hover.take().is_some() { cx.notify(); }
            return;
        };
        if a.probe.request(point) { self.annotation_probe(id, point, window, cx); }
    }

    fn annotation_probe(&mut self, id: &str, [x, y]: Pos, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get(id) else { return };
        let (id, session) = (id.to_owned(), a.session.clone());
        let (tx, rx) = async_channel::bounded(1);
        self.eval_browser_json(&id, &format!("return window.__canopy.hit({x},{y})"), move |value| { let _ = tx.try_send(value); });
        cx.spawn_in(window, async move |this, cx| {
            let value = rx.recv().with_timeout(Duration::from_secs(2), cx.background_executor()).await.ok().and_then(Result::ok).flatten();
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.annotation_is(&id, &session) { return; }
                let a = this.canvas_state.annotations.get_mut(&id).unwrap();
                match std::mem::replace(&mut a.probe, Probe::Idle) {
                    Probe::Idle | Probe::Away => {}
                    Probe::Asking => {
                        a.hover = value.and_then(|v| Some((numbers::<4>(&v["rect"])?, page_string(&v["label"], 60))));
                        cx.notify();
                    }
                    Probe::Queued(point) => {
                        a.probe = Probe::Asking;
                        this.annotation_probe(&id, point, window, cx);
                    }
                }
            });
        }).detach();
    }

    fn annotation_gesture_move(&mut self, id: &str, at: Point<Pixels>, cx: &mut Context<Self>) {
        let body = self.bounds.borrow().get(&format!("annotate-body:{id}")).copied();
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        if let Some(drag) = &mut a.drag
            && let Some((p, _)) = annotation_point(body, a.page_size, at, true)
            && drag.end() != p
        {
            if a.tool != Tool::Pen { drag.points.truncate(1); }
            drag.points.push(p);
        } else { return; }
        cx.stop_propagation();
        cx.notify();
    }

    fn annotation_gesture_end(&mut self, id: &str, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.bounds.borrow().get(&format!("annotate-body:{id}")).copied();
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        let Some(mut drag) = a.drag.take() else { return };
        cx.stop_propagation();
        cx.notify();
        let Some((end, scale)) = annotation_point(body, a.page_size, at, true) else { return };
        if drag.end() != end { drag.points.push(end); }
        let dragged = (end[0] - drag.start[0]).hypot(end[1] - drag.start[1]) * scale > 4.;
        let browser = matches!(a.host, Host::Browser { .. });
        let area = rectangle(drag.start, end);
        match a.tool {
            Tool::Text => {
                let color = a.color;
                self.annotation_draft(id, drag.start, DraftKind::Label { color }, None, "", window, cx);
            }
            Tool::Pen | Tool::Box => {
                if let Some(mark) = drag.mark(a.tool, a.color, a.width) { a.push(Action::Mark(mark)); }
            }
            Tool::Select if browser => {
                a.refreshing = true;
                a.probe.leave();
                a.hover = None;
                let [x, y] = drag.start;
                let [ex, ey] = end;
                let query = if dragged {
                    format!("return window.__canopy.isText({x},{y}) ? window.__canopy.textAt({x},{y},{ex},{ey}) : null")
                } else {
                    format!("return await window.__canopy.at({x},{y})")
                };
                let id = id.to_owned();
                self.annotation_query(&id.clone(), query, window, cx, move |this, value, window, cx| {
                    this.canvas_state.annotations.get_mut(&id).unwrap().refreshing = false;
                    let Some(value) = value else { return cx.notify() };
                    let anchor = if dragged { Some(tidy_text(&value).unwrap_or(Anchor::Area(area))) } else { tidy_element(&value).map(|a| this.resolve_component_file(&id, a)) };
                    if let Some(anchor) = anchor {
                        this.annotation_draft(&id, end, DraftKind::Comment { anchor }, None, "", window, cx);
                    } else {
                        this.annotation_error("No page element at this point", window, cx);
                    }
                    cx.notify();
                });
            }
            Tool::Select => {
                let anchor = if dragged { Anchor::Area(area) } else { Anchor::Point(end) };
                self.annotation_draft(id, end, DraftKind::Comment { anchor }, None, "", window, cx);
            }
        }
    }

    fn annotation_edit(&mut self, id: &str, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get(id) else { return };
        if a.refreshing || a.outgoing.as_ref().is_some_and(|o| o.sending) { return; }
        let Some(comment) = a.items().into_iter().find_map(|item| match item {
            Item::Comment(i, comment) if i == index => Some(comment.clone()), _ => None,
        }) else { return };
        let Some(&[x, y, ..]) = a.anchor_rects(&comment.anchor).first() else { return };
        let Comment { anchor, text } = comment;
        self.annotation_draft(id, [x, y], DraftKind::Comment { anchor }, Some(index), &text, window, cx);
    }

    fn annotation_draft(&mut self, id: &str, at: Pos, kind: DraftKind, editing: Option<usize>, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        let placeholder = if matches!(kind, DraftKind::Label { .. }) { "Type a label" } else { "Add a comment" };
        let input = cx.new(|cx| {
            // Enter submits; Shift+Enter adds a line, and the pill grows up to four.
            let mut input = gpui_base::input::TextareaState::new(window, cx).auto_grow(1, 4).submit_on_enter(true).placeholder(placeholder);
            input.set_value(value.to_owned(), window, cx);
            input.focus(window, cx);
            input
        });
        let target = id.to_owned();
        let enter = cx.subscribe_in(&input, window, move |this, _, event, window, cx| {
            if let gpui_base::input::InputEvent::PressEnter { secondary, shift: false } = event {
                this.annotation_commit(&target, *secondary, window, cx);
            }
        });
        a.editing = editing;
        a.hover = None;
        a.draft = Some(Draft { at, kind, input, menu: false, _enter: enter });
        cx.notify();
    }

    /// Esc in the composer drops the draft and keeps the annotation. Returns whether there was one.
    fn annotation_cancel_draft(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return false };
        if a.draft.take().is_none() { return false; }
        a.editing = None;
        a.focus.focus(window, cx);
        cx.notify();
        true
    }

    /// Enter in the composer (Add) or Cmd+Enter (Send): saves the draft, then optionally sends the queue.
    /// Saving an existing comment with no text deletes it; Send with no text just sends what is queued.
    fn annotation_commit(&mut self, id: &str, send: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return };
        let Some(draft) = a.draft.as_ref() else { return };
        let text: String = draft.input.read(cx).value().trim().chars().filter(|c| *c == '\n' || !c.is_control()).collect();
        if text.is_empty() && a.editing.is_none() {
            if !(send && !a.items().is_empty()) {
                return self.annotation_error("Enter some text first", window, cx);
            }
            a.draft = None;
        } else {
            let draft = a.draft.take().unwrap();
            let action = match (draft.kind, a.editing.take()) {
                (DraftKind::Label { color }, _) => Action::Mark(Mark::Text { at: draft.at, text, color }),
                (DraftKind::Comment { anchor }, Some(original)) => Action::Edit { original, comment: (!text.is_empty()).then_some(Comment { anchor, text }) },
                (DraftKind::Comment { anchor }, None) => Action::Comment(Comment { anchor, text }),
            };
            a.push(action);
        }
        a.focus.focus(window, cx);
        cx.notify();
        if send && let Err(error) = self.send_annotation(id, window, cx) { self.annotation_error(error, window, cx); }
    }

    /// The comment pill next to its pin: a round button that Adds, with an Add/Send menu while it is hovered.
    fn annotation_composer(&self, id: &str, draft: &Draft, [x, y]: Pos, [w, h]: Pos, cx: &mut Context<Self>) -> Stateful<Div> {
        const PILL: f32 = 240.;
        const MENU: f32 = 56.;
        let width = (PILL + 34.).min(w);
        let comment = matches!(draft.kind, DraftKind::Comment { .. });
        let top = (y + 14.).clamp(MENU, (h - 56.).max(MENU));
        let (add, send, enter, leave, open) = (id.to_owned(), id.to_owned(), id.to_owned(), id.to_owned(), id.to_owned());
        let row = |label: &'static str, keys: &'static str| div().flex().items_center().justify_between().gap(px(14.)).h(px(24.)).px(px(8.))
            .rounded(px(ui::RADIUS_SM)).cursor_pointer().hover(|d| d.bg(c("hover")))
            .child(label).child(div().text_color(c("muted-foreground")).child(keys));
        let menu = div().id("composer-menu").flex().flex_col().p(px(2.)).bg(c("popover")).border_1().border_color(c("border"))
            .rounded(px(ui::RADIUS)).shadow_md().text_size(px(12.)).text_color(c("foreground")).occlude()
            .child(row("Add", "↵").id("composer-add").on_click(cx.listener(move |this, _, window, cx| this.annotation_commit(&add, false, window, cx))))
            .child(row("Send", "⌘↵").id("composer-send-all").on_click(cx.listener(move |this, _, window, cx| this.annotation_commit(&send, true, window, cx))));
        div().id("annotation-composer").absolute().left(px((x + 12.).clamp(0., (w - width).max(0.)))).top(px(top - MENU)).w(px(width))
            .flex().flex_col().justify_end().gap(px(4.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // Leaving the whole composer, menu included, closes the menu.
            .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                if *over { return; }
                if let Some(d) = this.canvas_state.annotations.get_mut(&leave).and_then(|a| a.draft.as_mut()).filter(|d| d.menu) {
                    d.menu = false;
                    cx.notify();
                }
            }))
            .child(div().h(px(MENU - 4.)).flex().items_end().justify_end().when(draft.menu && comment, |d| d.child(menu)))
            .child(div().flex().items_end().gap(px(6.)).occlude()
                .child(div().flex_1().min_w_0().px(px(12.)).py(px(6.)).bg(c("popover")).border_1().border_color(c("ring"))
                    .rounded(px(16.)).shadow_md().text_size(px(13.)).text_color(c("foreground"))
                    .child(gpui_base::Textarea::new(&draft.input)))
                .child(div().id("composer-send").role(gpui::Role::Button).aria_label("Add").size(px(28.)).flex_none()
                    .flex().items_center().justify_center().rounded_full().bg(c("foreground")).cursor_pointer()
                    .child(svg().path("icons/send.svg").size(px(14.)).text_color(c("popover")))
                    .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                        if let Some(d) = this.canvas_state.annotations.get_mut(&enter).and_then(|a| a.draft.as_mut()) && *over && !d.menu {
                            d.menu = true;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| this.annotation_commit(&open, false, window, cx)))))
    }
}

#[cfg(target_os = "macos")]
fn annotation_point(body: Option<Bounds<Pixels>>, page: Pos, at: Point<Pixels>, clamp: bool) -> Option<(Pos, f32)> {
    let body = body?;
    let rect = [body.origin.x.into(), body.origin.y.into(), body.size.width.into(), body.size.height.into()];
    let (scale, [ox, oy]) = fit(rect, page);
    if scale <= 0. { return None; }
    let [x, y] = [(f32::from(at.x) - ox) / scale, (f32::from(at.y) - oy) / scale];
    if clamp { return Some(([x.clamp(0., page[0]), y.clamp(0., page[1])], scale)); }
    ((0. ..page[0]).contains(&x) && (0. ..page[1]).contains(&y)).then_some(([x, y], scale))
}

#[cfg(target_os = "macos")]
impl AppWindow {
    fn annotation_action(&mut self, id: &str, action: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let Some(a) = self.canvas_state.annotations.get_mut(id) else { return Ok(()) };
        if a.outgoing.as_ref().is_some_and(|o| o.sending) { return Err("The annotation is being sent".into()); }
        if let Some(addr) = action.strip_prefix("agent:") {
            if !self.annotation_agents(id).iter().any(|agent| agent.0 == addr) {
                return Err("That agent is no longer running".into());
            }
            self.canvas_state.selected = vec![id.to_owned()];
            let a = self.canvas_state.annotations.get_mut(id).unwrap();
            a.target = Some(addr.to_owned());
            if let Some(draft) = &a.draft {
                draft.input.read(cx).focus_handle(cx).focus(window, cx);
            } else if !a.live {
                a.focus.focus(window, cx);
            }
            cx.notify();
            return Ok(());
        }
        match action {
            "discard" if a.confirming.is_some() => return self.end_annotation(id, false, window, cx),
            "close" => return self.annotation_close(id, window, cx),
            "send" => return self.send_annotation(id, window, cx),
            "hint" => {
                let path = annotation_hint_file();
                std::fs::create_dir_all(path.parent().unwrap()).and_then(|()| std::fs::write(path, []))
                    .map_err(|error| format!("Cannot save the annotation hint preference: {error}"))?;
                for a in self.canvas_state.annotations.values_mut() { a.hint = false; }
                cx.notify();
                return Ok(());
            }
            _ if a.refreshing || a.live || a.draft.is_some() => return Ok(()),
            "undo" => a.undo(),
            "redo" => a.redo(),
            action => {
                if let Some((_, color)) = COLORS.iter().find(|(name, _)| name.eq_ignore_ascii_case(action)) {
                    a.color = *color;
                } else if let Some((_, width)) = STROKES.iter().find(|(name, _)| name.eq_ignore_ascii_case(action)) {
                    a.width = *width;
                } else if let Some((tool, ..)) = TOOLS.iter().find(|(_, token, _, _)| *token == action) {
                    a.tool = *tool;
                    a.probe.leave();
                    a.hover = None;
                } else {
                    return Err("Unknown annotation action".into());
                }
            }
        }
        if !a.live && a.draft.is_none() { a.focus.focus(window, cx); }
        cx.notify();
        Ok(())
    }

    /// `canvas.browser_pick`: start annotating, or switch the running annotation to Select.
    pub(super) fn annotation_pick(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        match self.canvas_state.annotations.get_mut(id) {
            Some(a) => { a.tool = Tool::Select; cx.notify(); Ok(()) }
            None => self.annotate_toggle(id, window, cx),
        }
    }

    /// A click handler that runs a bar action in the annotation of `id`.
    fn annotation_click(&self, id: &str, action: impl Into<String>, cx: &mut Context<Self>) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let (pane, action) = (id.to_owned(), action.into());
        cx.listener(move |this, _, window, cx| {
            if let Err(error) = this.annotation_action(&pane, &action, window, cx) { this.annotation_error(error, window, cx); }
        })
    }

    fn annotation_bar(&self, id: &str, a: &Annotate, cx: &mut Context<Self>) -> Stateful<Div> {
        let items = a.items();
        let can_send = !items.is_empty() && !a.live && !a.refreshing && a.draft.is_none() && !a.outgoing.as_ref().is_some_and(|o| o.sending);
        let gap = || div().w(px(1.)).h(px(16.)).flex_none().bg(c("border"));
        let mut bar = super::helper_bar::bar_shell(1.).id("annotation-bar").relative().max_w_full().flex_wrap().items_center().justify_center().occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        for (tool, action, label, icon) in TOOLS {
            bar = bar.child(annotation_button(action, icon, label, a.tool == tool).on_click(self.annotation_click(id, action, cx)));
        }
        // Colour and stroke appear only for a tool that draws with them.
        if a.tool != Tool::Select {
            bar = bar.child(gap());
            for (label, color) in COLORS {
                bar = bar.child(div().id(label).role(gpui::Role::Button).aria_label(label)
                    .size(px(24.)).flex_none().flex().items_center().justify_center().rounded(px(ui::RADIUS_SM))
                    .cursor_pointer().hover(|d| d.bg(c("hover"))).when(a.color == color, |d| d.bg(c("hover")))
                    .child(div().size(px(14.)).rounded_full().bg(swatch(color)))
                    .tooltip(strip_tooltip(label.into(), None))
                    .on_click(self.annotation_click(id, label, cx)));
            }
        }
        if matches!(a.tool, Tool::Pen | Tool::Box) {
            bar = bar.child(gap());
            for (label, width) in STROKES {
                bar = bar.child(div().id(label).role(gpui::Role::Button).aria_label(label)
                    .size(px(24.)).flex_none().flex().items_center().justify_center().rounded(px(ui::RADIUS_SM))
                    .cursor_pointer().hover(|d| d.bg(c("hover"))).when(a.width == width, |d| d.bg(c("hover")))
                    .child(div().size(px(width + 2.)).rounded_full().bg(c("foreground")))
                    .tooltip(strip_tooltip(label.into(), None))
                    .on_click(self.annotation_click(id, label, cx)));
            }
        }
        bar = bar.child(gap());
        if !items.is_empty() {
            bar = bar.child(div().id("annotation-send").role(gpui::Role::Button).aria_label("Send")
                .flex().items_center().flex_none().h(px(24.)).px(px(10.))
                .rounded(px(ui::RADIUS_SM)).bg(swatch(PIN)).text_color(gpui::white()).text_size(px(12.))
                .opacity(if can_send { 1. } else { 0.45 }).when(can_send, |d| d.cursor_pointer())
                .child("Send").on_click(self.annotation_click(id, "send", cx)));
        }
        bar = bar.child(annotation_button("close", "icons/x.svg", "Stop annotating", false).on_click(self.annotation_click(id, "close", cx)));
        if a.live && matches!(a.host, Host::Browser { .. }) {
            let bounds = self.bounds.clone();
            let cover = format!("cover:annotate:{id}");
            bar = bar.child(canvas(move |b, _, _| { bounds.borrow_mut().insert(cover, b); }, |_, _, _, _| {}).absolute().size_full());
        }
        bar
    }

    fn annotation_hint(&self, id: &str, a: &Annotate, width: f32, cx: &mut Context<Self>) -> Div {
        let target = id.to_owned();
        let surface = if matches!(a.host, Host::Browser { .. }) { "webpage" } else { "simulator" };
        let bounds = self.bounds.clone();
        let cover = format!("cover:annotate-hint:{id}");
        div().relative().w(px((width - 8.).clamp(1., 370.))).p(px(8.)).flex().flex_col().items_end().gap(px(5.))
            .bg(c("popover")).border_1().border_color(c("border")).rounded(px(ui::RADIUS)).shadow_md()
            .text_size(px(12.)).occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().w_full().child(format!("Click anywhere to comment, or drag to select an area. Hold Space to temporarily use the {surface}.")))
            .child(div().id("annotate-hint-ok").role(gpui::Role::Button).aria_label("Got it")
                .px(px(8.)).py(px(3.)).rounded(px(ui::RADIUS_SM)).bg(swatch(PIN)).text_color(gpui::white()).cursor_pointer()
                .child("Got it").on_click(cx.listener(move |this, _, window, cx| {
                    if let Err(error) = this.annotation_action(&target, "hint", window, cx) { this.annotation_error(error, window, cx); }
                })))
            .when(a.live && matches!(a.host, Host::Browser { .. }), |d| {
                d.child(canvas(move |b, _, _| { bounds.borrow_mut().insert(cover, b); }, |_, _, _, _| {}).absolute().size_full())
            })
    }

    pub(in crate::window) fn annotation_escape(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if key.key != "escape" || key.modifiers.modified() || self.overlays.iter().any(|o| o.kind.modal_keys()) { return false; }
        let Some(id) = self.canvas_state.annotations.iter()
            .find(|(_, a)| a.focus.contains_focused(window, cx)).map(|(id, _)| id.clone()) else { return false };
        if self.annotation_cancel_draft(&id, window, cx) { return true; }
        if let Err(error) = self.annotation_close(&id, window, cx) { self.annotation_error(error, window, cx); }
        true
    }

    pub(in crate::window) fn annotation_focus(&self, cx: &App) -> Option<FocusHandle> {
        let id = self.canvas_state.selected.last()?;
        let a = self.canvas_state.annotations.get(id)?;
        // A popped page's annotation takes the keyboard in its own window.
        if self.browser_popped(id) { return None; }
        if a.live || a.outgoing.as_ref().is_some_and(|o| o.sending) || !self.annotation_pane_visible(id) { return None; }
        Some(a.draft.as_ref().map_or_else(|| a.focus.clone(), |d| d.input.read(cx).focus_handle(cx)))
    }

    /// Whether the main window can give the annotation focus or show its live page again.
    fn annotation_pane_visible(&self, id: &str) -> bool {
        self.canvas_state.doc.panes.iter().any(|p| p.id == id && self.canvas_state.shows_pane(p))
    }

    /// Running agents an annotation of pane `id` can go to: wired ones first, then the pane's space.
    fn annotation_agents(&self, id: &str) -> Vec<(String, bool, Value)> {
        let home = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.space.as_deref());
        self.ranked_agents(Some(id), home)
    }

    /// The agent picker beside Annotate: its label, or None when Send needs no choice (exactly one
    /// agent wired, or none running).
    pub(super) fn annotation_picker(&self, id: &str) -> Option<String> {
        let a = self.canvas_state.annotations.get(id)?;
        let agents = self.annotation_agents(id);
        if agents.is_empty() || agents.iter().filter(|x| x.1).count() == 1 { return None; }
        Some(a.target.as_ref().filter(|t| agents.iter().any(|x| &x.0 == *t)).map_or_else(|| "Choose agent".to_owned(), |t| self.pill_name(t)))
    }

    /// Where Send goes: the sole wired agent, else the one picked beside Annotate.
    fn annotation_target(&self, id: &str) -> Result<String, String> {
        let agents = self.annotation_agents(id);
        let chosen = self.canvas_state.annotations.get(id).and_then(|a| a.target.as_deref());
        annotation_recipient(&agents, chosen).map(str::to_owned).map_err(str::to_owned)
    }

    /// The menu the picker opens: wired agents first, then the others.
    fn annotation_agent_menu(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let chosen = self.canvas_state.annotations.get(id).ok_or("Start annotating first")?.target.clone();
        let rows: Vec<_> = self.annotation_agents(id).into_iter().map(|(addr, wired, _)| {
            let label = format!("{}{}{}", if chosen.as_deref() == Some(&addr) { "✓ " } else { "" }, self.pill_name(&addr), if wired { " (wired)" } else { "" });
            json!([label, "canvas.annotate_action", {"pane": id, "action": format!("agent:{addr}")}])
        }).collect();
        if rows.is_empty() { return Err("No agent is running to send this to".into()); }
        self.show_browser_menu(id, self.browser_menu_host(id, window), rows, window, cx);
        Ok(())
    }
}

#[cfg(any(target_os = "macos", test))]
fn annotation_recipient<'a>(agents: &'a [(String, bool, Value)], chosen: Option<&str>) -> Result<&'a str, &'static str> {
    let mut wired = agents.iter().filter(|agent| agent.1);
    if let (Some((addr, ..)), None) = (wired.next(), wired.next()) { return Ok(addr); }
    agents.iter().find(|agent| chosen == Some(agent.0.as_str())).map(|agent| agent.0.as_str())
        .ok_or(if agents.is_empty() { "No agent is running to send this to" } else { "Choose an agent next to Annotate first" })
}

#[cfg(not(target_os = "macos"))]
impl AppWindow {
    pub(super) fn annotation_picker(&self, _: &str) -> Option<String> { None }
}

#[cfg(target_os = "macos")]
impl AppWindow {
    fn send_annotation(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let a = self.canvas_state.annotations.get(id).ok_or("There is no annotation to send")?;
        if a.live || a.refreshing || a.draft.is_some() { return Err("Finish the current annotation edit before sending".into()); }
        // Fail before the screenshot is saved when there is nowhere to send it.
        let addr = self.annotation_target(id)?;
        let a = self.canvas_state.annotations.get_mut(id).unwrap();
        if let Some(outgoing) = &a.outgoing {
            if outgoing.sending { return Err("The annotation is being sent".into()); }
            return self.offer_annotation(id, addr, window, cx);
        }
        let items = a.items();
        if items.is_empty() { return Err("Add a mark or comment before sending".into()); }
        let mut marks = Vec::new();
        let mut pins = Vec::new();
        for item in items {
            match item {
                Item::Mark(mark) => marks.push(mark.clone()),
                Item::Comment(_, comment) => {
                    if let Some(&[x, y, ..]) = a.anchor_rects(&comment.anchor).first() { pins.push([x, y]); }
                }
            }
        }
        let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let pane: String = a.pane.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let dir = std::env::temp_dir().join("canopy");
        let base = format!("annotate-{pane}-{stamp}");
        let path = dir.join(format!("{base}.png"));
        let images = vec![path.to_string_lossy().into_owned()];
        let text = message(a);
        let (still, page_size, session) = (a.still.clone(), a.page_size, a.session.clone());
        a.refreshing = true;
        a.drag = None;
        let id = id.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            let saved = cx.background_executor().spawn(async move {
                let write = || -> anyhow::Result<()> {
                    let shot = burn(still, page_size, &marks, &pins)?;
                    std::fs::create_dir_all(dir)?;
                    crate::control::save_png(&shot, &path)?;
                    Ok(())
                };
                let result = write();
                if result.is_err() { let _ = std::fs::remove_file(&path); }
                result
            }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.annotation_is(&id, &session) { return; }
                let a = this.canvas_state.annotations.get_mut(&id).unwrap();
                a.refreshing = false;
                match saved {
                    Ok(()) => {
                        a.outgoing = Some(Outgoing { text, images, sending: false });
                        if let Err(error) = this.offer_annotation(&id, addr, window, cx) {
                            this.annotation_error(error, window, cx);
                        }
                    }
                    Err(error) => this.annotation_error(format!("Cannot save the annotation: {error:#}"), window, cx),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
        Ok(())
    }

    fn offer_annotation(&mut self, id: &str, addr: String, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        // Saving takes time. A wire or agent change must never redirect a Send already pressed.
        if self.annotation_target(id).as_deref() != Ok(addr.as_str()) {
            return Err("The annotation recipient changed. Confirm the agent and press Send again".into());
        }
        let outgoing = self.canvas_state.annotations.get(id).and_then(|a| a.outgoing.as_ref())
            .ok_or("The annotation changed. Press Send again")?;
        let quote = json!({"kind":"annotation","pane":id,"text":outgoing.text,"path":outgoing.images[0],"images":outgoing.images});
        self.route_annotation(addr, &quote, window, cx)
    }

    pub(super) fn route_annotation(&mut self, addr: String, quote: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let id = quote["pane"].as_str().ok_or("The annotation pane is missing")?;
        let a = self.canvas_state.annotations.get_mut(id).ok_or("The annotation is closed")?;
        let outgoing = a.outgoing.as_mut().filter(|o| quote["path"].as_str() == o.images.first().map(String::as_str))
            .ok_or("The annotation changed. Choose Send again")?;
        if outgoing.sending { return Err("The annotation is being sent".into()); }
        if !self.canvas_state.annotation_targets.insert(addr.clone()) { return Err("This agent is already receiving an annotation".into()); }
        outgoing.sending = true;
        let mut outgoing = outgoing.clone();
        if let Some(message) = quote["message"].as_str().map(str::trim).filter(|m| !m.is_empty()) {
            outgoing.text.push_str("\n\n");
            outgoing.text.extend(message.chars().filter(|&c| c == '\n' || c == '\t' || !c.is_control()));
        }
        let session = a.session.clone();
        self.paste_annotation(addr, id.to_owned(), session, outgoing, window, cx);
        cx.notify();
        Ok(())
    }

    pub(super) fn annotation_sending(&self, id: &str, session: &Rc<()>) -> bool {
        self.annotation_is(id, session)
            && self.canvas_state.annotations[id].outgoing.as_ref().is_some_and(|o| o.sending)
    }

    pub(super) fn annotation_delivered(&mut self, addr: &str, id: &str, session: &Rc<()>, result: Result<(), String>, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state.annotation_targets.remove(addr);
        if result.is_ok() {
            if self.annotation_is(id, session) && let Err(error) = self.end_annotation(id, true, window, cx) {
                self.annotation_error(error, window, cx);
            }
            self.toast(Toast {
                title: format!("Sent to {}", self.pill_name(addr)), body: None, level: Level::Success,
                action: Some(("agent.focus".into(), json!({"terminal":addr}))),
            }, window, cx);
        } else if let Err(error) = result
            && self.annotation_is(id, session)
        {
            // Pasted but not submitted: ending here keeps a second Send from pasting it again.
            if error.ends_with(NOT_SUBMITTED) {
                if let Err(e) = self.end_annotation(id, true, window, cx) { self.annotation_error(e, window, cx); }
            } else if let Some(outgoing) = self.canvas_state.annotations.get_mut(id).and_then(|a| a.outgoing.as_mut()) {
                outgoing.sending = false;
            }
            self.annotation_error(error, window, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Anchor, Annotate, COLORS, Comment, Host, Item, Mark, Probe, STROKES, Tool, annotation_recipient, burn, fit, message, tidy_element};
    use gpui::{RenderImage, TestAppContext};
    use serde_json::json;
    use std::{collections::HashMap, rc::Rc, sync::Arc};

    fn example(host: Host, cx: &TestAppContext) -> Annotate {
        let still = image::RgbaImage::from_pixel(20, 20, image::Rgba([255; 4]));
        let image = Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(still.clone())]));
        Annotate {
            pane: "p1".into(), host, still, image, page_size: [1280., 800.],
            done: vec![], undone: vec![], tool: Tool::Select, color: COLORS[0].1, width: STROKES[1].1,
            live: false, drag: None, editing: None, draft: None, measured: HashMap::new(),
            focus: cx.update(|cx| cx.focus_handle()), session: Rc::new(()), was_live: false, refreshing: false,
            hint: false, target: None, hover: None, probe: Probe::Idle, confirming: None, outgoing: None,
            #[cfg(target_os = "macos")]
            held: None,
        }
    }

    fn comment(text: &str, anchor: Anchor) -> Action {
        Action::Comment(Comment { anchor, text: text.into() })
    }

    #[gpui::test]
    fn message_describes_element_area_and_text(cx: &mut TestAppContext) {
        let mut a = example(Host::Browser { url: "http://localhost:3000/".into(), viewport: Some([1280., 800.]) }, cx);
        a.push(Action::Comment(Comment {
            anchor: tidy_element(&json!({
                "selector": "#buy", "tag": "<button>", "text": "Buy now", "html": "<button>Buy now</button>",
                "rect": [12, 20, 100, 32], "component": "Pricing > Buy", "file": "Pricing.tsx", "line": 42,
                "styles": {"font-size":"14px","color":"rgb(10, 20, 30)","margin":"0px","padding":"auto","position":"normal","background-color":"none"}
            })).unwrap(),
            text: "Align the button".into(),
        }));
        a.push(comment("Check this area", Anchor::Area([12.2, 40.3, 300., 120.])));
        a.push(comment("Use sentence case", Anchor::Text { text: "BUY NOW".into(), selector: "#buy".into(), rects: vec![[12., 20., 100., 32.]] }));
        a.push(Action::Mark(Mark::Text { at: [11.2, 60.], text: "baseline".into(), color: COLORS[0].1 }));
        let actual = message(&a);
        assert_eq!(actual, concat!(
            "Annotated http://localhost:3000/ (viewport 1280x800)\n",
            "Image 1 is the annotated screenshot; numbered blue dots mark the comments.\n\n",
            "1. Align the button\n   Element: #buy <button> \"Buy now\"\n",
            "   Component: Pricing > Buy (Pricing.tsx:42)\n   Styles: font-size: 14px; color: rgb(10, 20, 30)\n",
            "   HTML: <button>Buy now</button>\n",
            "2. Check this area\n   Area: x 12, y 40, 300 x 120\n",
            "3. Use sentence case\n   Text: \"BUY NOW\" in #buy\n",
            "Notes on the screenshot: \"baseline\" at 11, 60",
        ));
    }

    #[gpui::test]
    fn message_describes_sim_point(cx: &mut TestAppContext) {
        let mut a = example(Host::Sim { device: "iPhone 17".into(), runtime: "iOS 26.1".into() }, cx);
        a.page_size = [402., 874.];
        a.push(comment("Increase the tap target", Anchor::Point([120., 300.])));
        assert_eq!(message(&a), concat!(
            "Annotated iPhone 17 (iOS 26.1), 402x874 points\n",
            "Image 1 is the annotated screenshot; numbered blue dots mark the comments.\n\n",
            "1. Increase the tap target\n   Point: x 120, y 300",
        ));
    }

    #[test]
    fn annotation_recipient_follows_the_current_wires_and_running_agents() {
        let mut agents = [("t1".into(), true, json!(null)), ("t2".into(), false, json!(null))];
        let sent_to = annotation_recipient(&agents, Some("t2")).unwrap().to_owned();
        assert_eq!(sent_to, "t1");
        agents[0].1 = false;
        agents[1].1 = true;
        assert_ne!(annotation_recipient(&agents, None), Ok(sent_to.as_str()));
        agents[0].1 = true;
        assert!(annotation_recipient(&agents, None).is_err());
        assert_eq!(annotation_recipient(&agents, Some("t2")), Ok("t2"));
        agents[1].1 = false;
        assert!(annotation_recipient(&agents[1..], Some("t1")).is_err());
        assert_eq!(annotation_recipient(&agents[1..], Some("t2")), Ok("t2"));
    }

    #[test]
    fn browser_hover_does_not_start_another_query_after_pointer_reentry() {
        let mut probe = Probe::Idle;
        assert!(probe.request([10., 20.]));
        probe.leave();
        assert!(!probe.request([30., 40.]));
        assert!(!probe.request([50., 60.]));
        assert!(matches!(probe, Probe::Queued([50., 60.])));
    }

    #[test]
    fn fit_centers_and_maps_page_units() {
        let (scale, origin) = fit([20., 10., 400., 250.], [100., 100.]);
        assert_eq!((scale, origin), (2.5, [95., 10.]));
        assert_eq!([(220. - origin[0]) / scale, (135. - origin[1]) / scale], [50., 50.]);
    }

    #[test]
    fn burn_strokes_box_without_filling_it() {
        let still = image::RgbaImage::from_pixel(20, 20, image::Rgba([255; 4]));
        let mark = Mark::Box { rect: [3., 3., 14., 14.], width: 2., color: COLORS[0].1 };
        let out = burn(still, [20., 20.], &[mark], &[]).unwrap();
        assert_eq!(out.get_pixel(3, 10).0, COLORS[0].1);
        assert_eq!(out.get_pixel(10, 10).0, [255; 4]);
    }

    #[gpui::test]
    fn undo_redo_keeps_marks_and_comments_in_order(cx: &mut TestAppContext) {
        let mut a = example(Host::Browser { url: "http://localhost/".into(), viewport: None }, cx);
        a.push(Action::Mark(Mark::Pen { points: vec![[2., 2.]], width: 4., color: COLORS[0].1 }));
        a.push(comment("Here", Anchor::Point([4., 5.])));
        a.undo();
        assert!(matches!(a.items().as_slice(), [Item::Mark(_)]));
        a.undo();
        assert!(a.items().is_empty());
        a.redo();
        a.redo();
        assert!(matches!(a.items().as_slice(), [Item::Mark(_), Item::Comment(1, _)]));
        a.push(Action::Edit { original: 1, comment: None });
        assert!(matches!(a.items().as_slice(), [Item::Mark(_)]));
        assert!(a.undone.is_empty());
        a.undo();
        assert!(matches!(a.items().as_slice(), [Item::Mark(_), Item::Comment(1, _)]));
        let body = message(&a);
        assert!(body.starts_with("Annotated http://localhost/ (responsive viewport 1280x800)"));
        assert!(body.ends_with("Marks drawn: 1 pen."));
    }
}
