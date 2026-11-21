//! Open a browser pane on the canvas and enter an address in its focused New tab field.
//! Click a loaded page to interact with it. WebKit follows pans, zoom and resizes while
//! live, with the canvas edge and foreground panes, helper bars and overlays masked out.
//! Click elsewhere to rest the page and draw its last snapshot.
//!
//! Responsive pages reflow with the pane. Fixed CSS viewport presets scale to fit the
//! pane body and sit centered inside it. See docs/wiki/Browser-Pane.md.
//!
//! The page loads only what the person or an agent opens: no homepage, a persistent
//! website store per Canopy home (see `store_id`), no downloads, and bundled react-grab with its
//! update check and font import removed.
use super::*;

/// react-grab 0.2.0 (MIT, vendor/react-grab), never fetched.
const REACT_GRAB: &str = include_str!("../../../vendor/react-grab/index.global.js");
/// Requests react-grab would make on its own; `page_script` removes them.
const GRAB_REQUESTS: [(&str, &str); 3] = [
    ("@import url(\"https://fonts.googleapis.com/css2?family=Geist:wght@500&display=swap\");", ""),
    ("https://www.react-grab.com/api/version", "data:,"),
    ("https://react-grab.com/open-file", "data:,"),
];

/// What runs at document start in every browser pane: react-grab with its auto start
/// off, then Canopy's picker (browser.js).
fn page_script() -> String {
    let mut grab = REACT_GRAB.to_owned();
    for (from, to) in GRAB_REQUESTS { grab = grab.replacen(from, to, 1); }
    format!("window.__REACT_GRAB_DISABLED__ = true;\n{grab}\n{}", include_str!("browser.js"))
}

/// How long a load may take before what waits for it is answered anyway.
const LOAD_LIMIT: std::time::Duration = std::time::Duration::from_secs(15);
/// How long a loaded page gets to render before reads, shots and picks.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(500);
/// Default size of a new browser pane, world px.
pub(super) const SIZE: [f32; 2] = [960., 680.];

/// Command name, menu label and fixed CSS size.
const PRESETS: [(&str, &str, Option<[f32; 2]>); 5] = [
    ("responsive", "Responsive", None),
    ("1280x800", "1280 × 800", Some([1280., 800.])),
    ("1024x768", "1024 × 768", Some([1024., 768.])),
    ("mobile", "Mobile 375 × 812", Some([375., 812.])),
    ("tablet", "Tablet 768 × 1024", Some([768., 1024.])),
];

fn parse_viewport(size: Option<&str>) -> Result<Option<[f32; 2]>, String> {
    let size = if size == Some("desktop") { Some("1280x800") } else { size };
    PRESETS.iter().find(|(name, _, _)| Some(*name) == size).map(|&(_, _, viewport)| viewport)
        .ok_or_else(|| format!("size must be responsive, 1280x800 (desktop), 1024x768, mobile or tablet, got {}", size.unwrap_or("missing")))
}

#[cfg(any(target_os = "macos", test))]
fn fixed_viewport(pane: &canvas::Pane) -> Option<[f32; 2]> {
    let canvas::Kind::Browser { viewport, .. } = pane.kind else { return None };
    viewport.filter(|size| size.iter().all(|px| px.is_finite() && *px > 0.))
}

#[cfg(any(target_os = "macos", test))]
fn page_zoom(pane: &canvas::Pane, camera: canvas::Camera, body: [f32; 2]) -> f64 {
    match fixed_viewport(pane) {
        Some([w, h]) => (f64::from(body[0]) / f64::from(w)).min(f64::from(body[1]) / f64::from(h)),
        _ if pane.pinned.is_some() => 1.,
        _ => f64::from(camera.zoom),
    }
}

const ENTER_URL: &str = "type a URL in the address field first";

/// Opening tags in a quote are cut to this many characters, visible text to `TEXT_CUT`.
const TAG_CUT: usize = 80;
const TEXT_CUT: usize = 200;

/// Browser state that is not saved: native views, snapshots and waiting replies.
#[derive(Default)]
pub(in crate::window) struct Browsers {
    pages: HashMap<String, Page>,
    #[cfg(target_os = "macos")]
    pub(super) last_masks: Rc<RefCell<HashMap<String, Option<Vec<super::browser_mask::R>>>>>,
    /// Page events into the window; made with the first web view.
    events: Option<async_channel::Sender<(String, Event)>>,
}

#[derive(Default)]
struct Page {
    /// The native view, kept (hidden) while the page rests so a page may play out of sight
    /// and waking it does not reload.
    #[cfg(target_os = "macos")]
    native: Option<Native>,
    /// Rested and not yet checked out of sight for media still playing (`watch_pages`).
    watch: bool,
    /// The "still playing" toast was raised and the page has not been woken since.
    told: bool,
    /// The last snapshot, BGRA for GPUI.
    shot: Option<Arc<RenderImage>>,
    title: String,
    /// While live: the last placement used to set WebKit zoom.
    live: Option<(canvas::Camera, canvas::Rect)>,
    loading: bool,
    /// Replies held until the page finishes loading.
    waiting: Vec<Ask>,
    /// Pick mode starts once the page is live and loaded.
    pick: bool,
    /// The person started a pick: only then is a `{pick}` or `{error}` from the page taken,
    /// since any script on the page can post to Canopy.
    picking: bool,
    /// Counts loads, so a load's time limit leaves later loads alone.
    loads: u64,
    /// The header's address field, made when the pane is first drawn.
    url_input: Option<(Entity<gpui_base::input::InputState>, Subscription)>,
    focus_url: bool,
    /// The window showing the page while it is popped out; the pane has no view meanwhile.
    popped: Option<AnyWindowHandle>,
}

/// A page's native view: the GPUI entity, and the same view for scripts run without a GPUI
/// context. Both or neither.
#[cfg(target_os = "macos")]
struct Native { view: Entity<gpui_wry::WebView>, raw: gpui_wry::WebViewHandle }

#[cfg(target_os = "macos")]
impl Drop for Native {
    fn drop(&mut self) {
        super::browser_mask::clear_mask(self.raw.raw());
    }
}

enum Ask {
    Read(std::sync::mpsc::Sender<Value>),
    Shot(std::path::PathBuf, std::sync::mpsc::Sender<Value>),
}

/// What a web view reports, on the main thread, through `Browsers::events`.
pub(in crate::window) enum Event {
    Started,
    Loaded,
    Title(String),
    /// A message from browser.js.
    Message(String),
    /// A link asked for a new window: it opens in this pane instead.
    Open(String),
    /// The snapshot taken when the page rested (None: it failed).
    Rested(Option<image::RgbaImage>),
    /// A resting page out of sight has media playing.
    Playing,
}

/// Where a pane's last snapshot is kept between runs.
fn shot_file(id: &str) -> std::path::PathBuf {
    crate::mods::app_dir().join("browser").join(format!("{id}.png"))
}

/// GPUI draws BGRA.
fn render_image(mut rgba: image::RgbaImage) -> Arc<RenderImage> {
    for px in rgba.pixels_mut() { px.0.swap(0, 2); }
    Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(rgba)]))
}

/// The URL a person or agent typed, as a web page to load: http and https only. A bare
/// local host (localhost:3000, 127.0.0.1, *.localhost, *.test) gets http://, any other bare
/// host https://.
pub(in crate::window) fn web_url(input: &str) -> Result<String, String> {
    let url = input.trim();
    if url.is_empty() || url.chars().any(char::is_whitespace) { return Err(format!("not a web address: {input:?}")); }
    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") { return Err(format!("only http and https pages open in a browser pane, not {scheme}:")); }
        if rest.is_empty() || rest.starts_with('/') { return Err(format!("{url} has no host")); }
        return Ok(url.to_owned());
    }
    let host = url.split(['/', '?', '#']).next().unwrap_or_default();
    // A colon not followed by a port is a scheme (javascript:, file:, data:).
    if let Some((_, port)) = host.rsplit_once(':') && !host.starts_with('[') && (port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit())) {
        return Err(format!("only http and https pages open in a browser pane: {url}"));
    }
    let name = host.rsplit_once(':').filter(|_| !host.starts_with('[')).map_or(host, |(n, _)| n).to_ascii_lowercase();
    let local = matches!(name.as_str(), "localhost" | "127.0.0.1" | "0.0.0.0") || host.starts_with("[::1]") || name.ends_with(".localhost") || name.ends_with(".test");
    Ok(format!("{}://{url}", if local { "http" } else { "https" }))
}

/// The file react-grab named (a path, a dev-server URL, a bundler URL or, from Vite's source
/// maps, a bare file name), found on disk: as an absolute path, else under each root,
/// dropping leading parts one at a time, else by searching each root for a file ending in it.
pub(in crate::window) fn resolve_source(file: &str, roots: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    let mut path = file.split(['?', '#']).next()?;
    for prefix in ["webpack-internal:///", "webpack:///", "webpack://", "file://"] { path = path.strip_prefix(prefix).unwrap_or(path); }
    if let Some((_, rest)) = path.split_once("://") { path = rest.find('/').map_or("", |i| &rest[i..]); }
    let path = std::path::Path::new(path);
    // The page names the file: an absolute path counts only inside a root.
    if path.is_absolute() && path.is_file() && roots.iter().any(|r| path.starts_with(r)) { return Some(path.to_path_buf()); }
    let parts: Vec<_> = path.components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect();
    let tail: std::path::PathBuf = parts.iter().collect();
    (0..parts.len()).find_map(|skip| {
        let tail: std::path::PathBuf = parts[skip..].iter().collect();
        roots.iter().map(|root| root.join(&tail)).find(|p| p.is_file())
    }).or_else(|| roots.iter().find_map(|root| find_suffix(root, &tail)))
}

/// The shallowest file under `root` whose path ends in `tail`, skipping dependency, VCS and
/// build folders and giving up after 50,000 entries. Runs once per pick, never while idle.
fn find_suffix(root: &std::path::Path, tail: &std::path::Path) -> Option<std::path::PathBuf> {
    const SKIP: [&str; 7] = ["node_modules", ".git", "target", "dist", "build", ".next", ".turbo"];
    if tail.as_os_str().is_empty() { return None; }
    let mut queue = std::collections::VecDeque::from([root.to_path_buf()]);
    let mut budget = 50_000;
    while let Some(dir) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            budget -= 1;
            if budget == 0 { return None; }
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            if kind.is_dir() && !SKIP.iter().any(|s| entry.file_name() == *s) { queue.push_back(path); }
            else if kind.is_file() && path.ends_with(tail) { return Some(path); }
        }
    }
    None
}

/// The lines (1-based, inclusive) of the JSX element at `line`:`col`: the first element
/// that starts there or after it, which for a component's definition (where React's stack
/// points) is the root element it returns. None when none is found, the position is past
/// the end, or the element runs past 400 lines. A hand scanner: strings, comments,
/// template literals and `{}` expressions are skipped; regex literals are not understood.
pub(in crate::window) fn jsx_lines(src: &str, line: usize, col: usize) -> Option<(usize, usize)> {
    let start_of_line = src.split_inclusive('\n').take(line.checked_sub(1)?).map(str::len).sum::<usize>();
    let row_len = src.get(start_of_line..)?.split('\n').next()?.len();
    let b = src.as_bytes();
    let line_at = |i: usize| src[..i].matches('\n').count() + 1;
    // An element starts at `<` + a name or `>`, after `(`, `=`, `?`, `:`, `,`, `{`, `[`, `&`,
    // `|`, `>` or `return`: not a comparison like `a < b`.
    let starts_jsx = |i: usize| {
        let before = src[..i].trim_end();
        before.is_empty() || before.ends_with("return") || before.ends_with(['(', '=', '?', ':', ',', '{', '[', '&', '|', '>'])
    };
    // Byte offset where line `n` (1-based) starts, or the end: the 400-line limits.
    let line_start = |n: usize| if n <= 1 { 0 } else { src.match_indices('\n').nth(n - 2).map_or(src.len(), |(at, _)| at + 1) };
    let from = start_of_line + col.saturating_sub(1).min(row_len);
    let tag = |i: usize| b[i] == b'<' && b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'>');
    // A tag on the start line is the element; past it, only one in an element position.
    let mut i = (from..start_of_line + row_len).find(|&i| tag(i))
        .or_else(|| (from..line_start(line + 401)).find(|&i| tag(i) && starts_jsx(i)))?;
    let first = line_at(i);
    let limit = line_start(first + 401);
    let mut depth = 0usize;
    loop {
        if i >= limit { return None; }
        match *b.get(i)? {
            b'{' => i = skip_braces(b, i)?,
            b'<' if b.get(i + 1) == Some(&b'/') => {
                i = i + b[i..].iter().position(|&c| c == b'>')? + 1;
                depth = depth.checked_sub(1)?;
                if depth == 0 { return Some((first, line_at(i - 1))); }
            }
            b'<' => {
                // An opening tag: its attributes, to `>` or `/>`.
                i += 1;
                loop {
                    match *b.get(i)? {
                        b'{' => i = skip_braces(b, i)?,
                        q @ (b'"' | b'\'') => i = i + 1 + b[i + 1..].iter().position(|&c| c == q)? + 1,
                        b'/' if b.get(i + 1) == Some(&b'>') => {
                            if depth == 0 { return Some((first, line_at(i))); }
                            i += 2;
                            break;
                        }
                        b'>' => { depth += 1; i += 1; break; }
                        _ => i += 1,
                    }
                }
            }
            _ => i += 1,
        }
    }
}

/// From a `{` to just past its `}`, skipping strings, template literals and comments.
fn skip_braces(b: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    loop {
        match *b.get(i)? {
            b'{' => depth += 1,
            b'}' => { depth -= 1; if depth == 0 { return Some(i + 1); } }
            q @ (b'"' | b'\'') => {
                i += 1;
                while *b.get(i)? != q { if b[i] == b'\\' { i += 1; } i += 1; }
            }
            b'`' => {
                i += 1;
                while *b.get(i)? != b'`' {
                    match b[i] {
                        b'\\' => i += 2,
                        b'$' if b.get(i + 1) == Some(&b'{') => i = skip_braces(b, i + 1)?,
                        _ => i += 1,
                    }
                }
            }
            b'/' if b.get(i + 1) == Some(&b'/') => i += b[i..].iter().position(|&c| c == b'\n')?,
            b'/' if b.get(i + 1) == Some(&b'*') => i += 2 + b[i + 2..].windows(2).position(|w| w == b"*/")? + 1,
            _ => {}
        }
        i += 1;
    }
}

/// Cuts text to `max` characters, marking the cut with `…`.
fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) { Some((at, _)) => format!("{}…", &text[..at]), None => text.to_owned() }
}

/// What a browser pick pastes into an agent: a pointer to the element, never its source
/// (docs/canopy/canvas-items-plan.md section 8). With React source:
/// `@<url> <Component> src/File.tsx:42-61 "<div class=card>"`; otherwise
/// `@<url> <css selector> "<visible text>"`. `cwd` shortens the file path. The quote:
/// `{kind: "browser", start?, end?, text, message?, source: {url, component?, file?, selector, tag}}`.
pub(in crate::window) fn format_quote(quote: &Value, cwd: Option<&str>) -> String {
    let source = &quote["source"];
    let s = |k: &str| source[k].as_str().or(quote[k].as_str()).unwrap_or_default();
    let mut out = format!("@{}", s("url"));
    if let Some(file) = source["file"].as_str() {
        let path = std::path::Path::new(file);
        let file = cwd.and_then(|cwd| path.strip_prefix(cwd).ok()).unwrap_or(path).display().to_string();
        if !s("component").is_empty() { out.push_str(&format!(" <{}>", s("component"))); }
        out.push_str(&format!(" {file}"));
        if let Some(start) = quote["start"].as_u64() {
            out.push_str(&format!(":{start}"));
            if let Some(end) = quote["end"].as_u64().filter(|e| *e != start) { out.push_str(&format!("-{end}")); }
        }
        out.push_str(&format!(" {}", serde_json::to_string(&cut(s("tag"), TAG_CUT)).unwrap()));
    } else {
        out.push_str(&format!(" {}", s("selector")));
        if !s("text").is_empty() { out.push_str(&format!(" {}", serde_json::to_string(&cut(s("text"), TEXT_CUT)).unwrap())); }
    }
    match quote["message"].as_str().map(str::trim).filter(|m| !m.is_empty()) {
        Some(message) => format!("{out}\n\n{message}"),
        None => out,
    }
}

impl AppWindow {
    /// Adds a browser pane at `point` in `space`. An empty URL focuses its New tab field.
    pub(super) fn add_browser_pane(&mut self, url: String, point: canvas::Point, space: Option<String>, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        let empty = url.is_empty();
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Browser { home: url.clone(), url, viewport: None }, rect: [point[0], point[1], SIZE[0], SIZE[1]], z, space, pinned: None });
        if empty { self.canvas_state.browsers.pages.entry(id.clone()).or_default().focus_url = true; }
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        cx.notify();
        id
    }

    /// Tears down the pages (and pop-out windows) of closed browser panes, drops their
    /// snapshots, and loads saved snapshots for panes not seen yet this run.
    pub(super) fn sync_browsers(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<_> = self.canvas_state.doc.panes.iter().filter(|p| p.url().is_some()).map(|p| p.id.clone()).collect();
        if self.canvas_state.browser_tab.as_ref().is_some_and(|id| !ids.contains(id)) { self.canvas_state.browser_tab = None; }
        let browsers = &mut self.canvas_state.browsers;
        #[cfg(target_os = "macos")]
        browsers.last_masks.borrow_mut().retain(|id, _| ids.contains(id));
        let pages = &mut browsers.pages;
        pages.retain(|id, page| {
            let keep = ids.contains(id);
            if !keep {
                let _ = std::fs::remove_file(shot_file(id));
                #[cfg(target_os = "macos")]
                if let Some(native) = page.native.take() { destroy(native); }
                if let Some(window) = page.popped.take() { let _ = window.update(cx, |_, window, _| window.remove_window()); }
            }
            keep
        });
        for id in ids {
            pages.entry(id.clone()).or_insert_with(|| Page {
                // shortcut: decodes saved snapshots on the main thread, move to a background task if many panes make loading slow.
                shot: image::open(shot_file(&id)).ok().map(|i| render_image(i.to_rgba8())),
                ..Default::default()
            });
        }
    }

    /// Draws New tab, the live web view, or the resting snapshot. Click a resting page to wake it.
    pub(super) fn browser_body(&self, pane: &canvas::Pane, camera: canvas::Camera, cx: &mut Context<Self>) -> AnyElement {
        let id = pane.id.clone();
        let page = self.canvas_state.browsers.pages.get(&id);
        if page.is_some_and(|p| p.popped.is_some()) {
            let id = id.clone();
            return div().size_full().flex().flex_col().items_center().justify_center().gap(px(12.)).p(px(16.)).bg(c("canvas")).text_color(c("muted-foreground"))
                .child("Popped out")
                .child(div().id(SharedString::from(format!("browser-{id}-back"))).role(gpui::Role::Button).aria_label("Bring back")
                    .px(px(12.)).py(px(4.)).rounded(px(ui::RADIUS_SM)).border_1().border_color(c("border")).text_color(c("foreground")).cursor_pointer()
                    .hover(|d| d.bg(c("hover"))).child("Bring back")
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| this.run_key("canvas.browser_pop", json!({"pane": id}), window, cx))))
                .into_any_element();
        }
        if pane.url() == Some("") && !page.is_some_and(Page::has_view) {
            return div().id(SharedString::from(format!("browser-{id}"))).size_full()
                .flex().flex_col().items_center().justify_center().gap(px(8.)).bg(c("canvas")).cursor_pointer()
                .child(svg().path("icons/globe.svg").size(px(32.)).text_color(c("muted-foreground")))
                .child(div().text_size(px(15.)).text_color(c("foreground")).child("Browse the web"))
                .child(div().text_size(px(13.)).text_color(c("muted-foreground")).child("Type a URL to open a site."))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    if let Err(error) = this.focus_browser_url(&id, cx) {
                        this.command_error("Open URL", &error, window, cx);
                    }
                })).into_any_element();
        }
        #[cfg(target_os = "macos")]
        if let Some(page) = page && let (Some(Native { view, .. }), Some(at)) = (&page.native, page.live) {
            let next = placement(pane, camera);
            let preset = fixed_viewport(pane).is_some();
            if at != next {
                let (this, id) = (cx.entity(), id.clone());
                let zoom_changed = at.0.zoom != next.0.zoom || (preset && at.1[2..] != next.1[2..]);
                let zoom = zoom_changed.then(|| {
                    let [_, _, width, height] = pane_body(pane, camera);
                    page_zoom(pane, camera, [width, height])
                });
                cx.defer(move |cx| this.update(cx, |this, _| {
                    let Some(page) = this.canvas_state.browsers.pages.get_mut(&id) else { return };
                    if page.live == Some(at) && let Some(native) = &page.native {
                        page.live = Some(next);
                        if let Some(zoom) = zoom.filter(|z| *z > 0.) { let _ = native.raw.raw().zoom(zoom); }
                    }
                }));
            }
            let body = if preset {
                let [_, _, width, height] = viewport_rect(pane, camera);
                div().size_full().bg(c("canvas")).flex().items_center().justify_center()
                    .child(div().flex_none().w(px(width)).h(px(height)).child(view.clone()))
            } else {
                div().size_full().bg(gpui::white()).child(view.clone())
            };
            return body.into_any_element();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = camera;
        let body = match page.and_then(|p| p.shot.clone()) {
            Some(shot) => div().size_full().bg(gpui::white()).child(img(ImageSource::Render(shot)).size_full().object_fit(gpui::ObjectFit::Cover)),
            None => div().size_full().flex().items_center().justify_center().p(px(16.)).text_color(c("muted-foreground"))
                .child(if page.is_some_and(|p| p.loading) { "Loading…".to_owned() } else { "Click to load the page".to_owned() }),
        };
        body.id(SharedString::from(format!("browser-{id}"))).cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.wake_browser(&id, window, cx);
            })).into_any_element()
    }

    /// The pane's title row text: the page title, else the URL.
    pub(super) fn browser_title(&self, pane: &canvas::Pane) -> (String, String) {
        let title = self.canvas_state.browsers.pages.get(&pane.id).map(|p| p.title.clone()).filter(|t| !t.is_empty());
        let url = pane.url().unwrap_or_default().to_owned();
        if url.is_empty() { return ("New tab".into(), url); }
        match title { Some(title) => (title, url), None => (url, String::new()) }
    }

    /// A browser pane's header controls: back, forward, reload, the address field, and "⋯".
    pub(super) fn browser_header(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let button = |slot: &'static str, icon: &'static str, label: &'static str, command: &'static str, cx: &mut Context<Self>| {
            let id = id.to_owned();
            panel_button(slot, icon, c("muted-foreground"), c("hover")).aria_label(label).tooltip(strip_tooltip(label.into(), Some(command)))
                .on_click(cx.listener(move |this, _, window, cx| this.run_key(command, json!({"pane": id}), window, cx)))
        };
        let (reset, more) = (id.to_owned(), id.to_owned());
        let field = self.canvas_state.browsers.pages.get(id).and_then(|p| p.url_input.as_ref()).map(|(input, _)| input.clone());
        div().flex().flex_1().min_w_0().items_center().gap(px(4.))
            .child(button("browser-back", "icons/arrow-left.svg", "Back", "canvas.browser_back", cx))
            .child(button("browser-forward", "icons/arrow-right.svg", "Forward", "canvas.browser_forward", cx))
            .child(button("browser-reload", "icons/rotate-cw.svg", "Reload", "canvas.browser_reload", cx))
            .child(div().flex().flex_1().min_w_0().h(px(24.)).px(px(8.)).items_center().overflow_hidden()
                .rounded(px(ui::RADIUS_SM)).bg(c("canvas")).border_1().border_color(c("border"))
                // Typing in the field is not a pane drag.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                    if e.keystroke.key == "escape" { this.reset_url(&reset, window, cx); cx.stop_propagation(); }
                }))
                .children(field.map(|input| gpui_base::Input::new(&input))))
            .child(panel_button("browser-more", "icons/ellipsis.svg", c("muted-foreground"), c("hover")).aria_label("More").tooltip(strip_tooltip("More".into(), None))
                .on_click(cx.listener(move |this, _, window, cx| this.browser_menu(&more, window, cx))))
            .into_any_element()
    }

    pub(in crate::window) fn selected_browser(&self) -> Option<String> {
        let [id] = self.canvas_state.selected.as_slice() else { return None };
        self.browser_url(id).ok().map(|_| id.clone())
    }

    pub(in crate::window) fn focus_browser_url(&mut self, id: &str, cx: &mut Context<Self>) -> Result<(), String> {
        self.browser_url(id)?;
        self.canvas_state.browsers.pages.entry(id.to_owned()).or_default().focus_url = true;
        cx.notify();
        Ok(())
    }

    fn browser_url(&self, id: &str) -> Result<&str, String> {
        self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.url())
            .ok_or_else(|| format!("no browser pane {id}"))
    }

    fn require_browser_url(&self, id: &str) -> Result<(), String> {
        if self.browser_url(id)?.is_empty() { return Err(ENTER_URL.into()); }
        Ok(())
    }

    /// Makes each browser pane's address field, and keeps its text at the pane's URL unless the
    /// person is typing in it. Enter opens what was typed (`canvas.pane_open`, which makes
    /// `localhost:3000` a URL and refuses non-web schemes). Runs from `render`, which has a window.
    pub(in crate::window) fn sync_browser_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let doc = &self.canvas_state.doc;
        let urls: Vec<_> = doc.panes.iter().filter_map(|p| Some((p.id.clone(), p.url()?.to_owned(), doc.shows(p.space.as_ref())))).collect();
        for (id, url, shown) in urls {
            let Some(page) = self.canvas_state.browsers.pages.get_mut(&id) else { continue };
            match &page.url_input {
                Some((input, _)) => {
                    let stale = input.read(cx).value().to_string() != url && !input.read(cx).focus_handle(cx).is_focused(window);
                    if stale { input.update(cx, |input, cx| input.set_value(url, window, cx)); }
                }
                None => {
                    let input = cx.new(|cx| {
                        let mut input = gpui_base::input::InputState::new(window, cx).placeholder("Search or enter address");
                        input.set_value(url, window, cx);
                        input
                    });
                    let pane = id.clone();
                    let sub = cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                        if !matches!(event, gpui_base::input::InputEvent::PressEnter { .. }) { return; }
                        let target = input.read(cx).value().to_string();
                        this.focus.focus(window, cx);
                        this.run_key("canvas.pane_open", json!({"pane": pane, "target": target}), window, cx);
                    });
                    page.url_input = Some((input, sub));
                }
            }
            if shown && std::mem::take(&mut page.focus_url) && let Some((input, _)) = &page.url_input {
                input.update(cx, |i, cx| i.focus(window, cx));
            }
        }
    }

    /// Escape in the address field: the pane's URL back in it, and the keyboard back to the canvas.
    fn reset_url(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.url()).unwrap_or_default().to_owned();
        if let Some(input) = self.canvas_state.browsers.pages.get(id).and_then(|p| p.url_input.as_ref().map(|(input, _)| input.clone())) {
            input.update(cx, |input, cx| input.set_value(url, window, cx));
        }
        self.focus.focus(window, cx);
    }

    /// The header's "⋯" menu, at the pointer.
    fn browser_menu(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return };
        let canvas::Kind::Browser { viewport, .. } = &pane.kind else { return };
        let popped = self.canvas_state.browsers.pages.get(id).is_some_and(|p| p.popped.is_some());
        let at = json!({"pane": id});
        let mut items = vec![
            json!(["Open in default browser", "canvas.browser_external", at]),
            json!(["Copy link", "canvas.browser_copy", at]),
            json!(["Save screenshot", "canvas.browser_shot", at]),
        ];
        for (name, label, size) in PRESETS {
            let mark = if *viewport == size { "✓ " } else { "" };
            items.push(json!([format!("{mark}Viewport: {label}"), "canvas.browser_viewport", {"pane": id, "size": name}]));
        }
        items.extend([
            // An expanded page hides its pane, menu included: its strip tab collapses it.
            json!(["Expand", "canvas.browser_expand", at]),
            json!([if popped { "Bring back" } else { "Pop out" }, "canvas.browser_pop", at]),
        ]);
        for (keep, value, label) in [
            (settings::KeepCookies::UntilQuit, "until_quit", "Until quit"),
            (settings::KeepCookies::Forever, "forever", "Forever"),
        ] {
            let mark = if self.person.browser_keep_cookies == keep { "✓ " } else { "" };
            items.push(json!([format!("{mark}Keep cookies: {label}"), "canvas.browser_cookies", {"keep": value}]));
        }
        items.push(json!(["Clear browsing data", "canvas.browser_clear", at]));
        self.open_menu("primary", json!({"items": items}), Some(window.mouse_position()), window, cx);
    }

    /// The expanded browser's tab for the top strip (None when none is expanded): globe, the
    /// page title, and a close button that collapses it.
    pub(super) fn browser_tab_strip(&self, fill: Hsla, strip: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pane = self.canvas_state.browser_tab.as_ref().and_then(|id| self.canvas_state.doc.panes.iter().find(|p| &p.id == id))?;
        let (title, _) = self.browser_title(pane);
        Some(strip_tab("browser-tab", true, fill, strip).role(gpui::Role::Tab).aria_selected(true).aria_label(title.clone())
            .flex().items_center().h_full().w(px(180.)).min_w(px(96.)).gap(px(6.)).px(px(12.))
            .text_size(px(12.)).text_color(c("foreground"))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(svg().path("icons/globe.svg").flex_none().size(px(14.)).text_color(c("foreground")))
            .child(div().flex_1().min_w_0().truncate().child(title))
            .child(panel_button("browser-tab-close", "mods/pane/close.svg", c("muted-foreground"), c("hover")).aria_label("Collapse")
                .tooltip(strip_tooltip("Collapse".into(), Some("canvas.browser_expand")))
                .on_click(cx.listener(|this, _, window, cx| this.collapse_browser(window, cx))))
            .into_any_element())
    }

    /// Shows the expanded live page over the measured canvas area.
    pub(super) fn expanded_browser(&self, bg: Hsla) -> Option<AnyElement> {
        let page = self.canvas_state.browsers.pages.get(self.canvas_state.browser_tab.as_ref()?)?;
        let bounds = self.bounds.clone();
        Some(div().id("canopy-browser-tab").relative().flex_1().h_full().overflow_hidden().bg(bg)
            .child(canvas(move |b, _, _| {
                let mut bounds = bounds.borrow_mut();
                bounds.insert("canopy-canvas".into(), b);
                #[cfg(target_os = "macos")]
                bounds.insert("browser-canvas".into(), b);
            }, |_, _, _, _| {}).absolute().size_full())
            .children(page.view_element()).into_any_element())
    }

    /// Answers page events from the web views.
    fn browser_event(&mut self, id: String, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(&id) else { return };
        match event {
            // A navigation inside the page (a link) committed.
            Event::Started => return self.begin_load(&id, cx),
            Event::Title(title) => page.title = title,
            Event::Loaded => {
                // Scripts often render after the load event (React does): let the page settle
                // before answering. The timer runs only while a load is pending.
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(SETTLE).await;
                    let _ = this.update(cx, |this, cx| this.page_ready(id, cx));
                }).detach();
                return;
            }
            Event::Open(url) => {
                if let Ok(url) = web_url(&url) { self.load_browser(&id, &url, false, window, cx); }
            }
            Event::Rested(shot) => {
                if let Some(rgba) = shot {
                    let file = shot_file(&id);
                    if let Some(dir) = file.parent() { let _ = std::fs::create_dir_all(dir); }
                    let _ = crate::control::save_png(&rgba, &file);
                    page.shot = Some(render_image(rgba));
                }
                #[cfg(target_os = "macos")]
                if page.live.is_none() && let Some(native) = &page.native {
                    native.view.update(cx, |v, _| v.hide());
                    super::browser_mask::clear_mask(native.raw.raw());
                }
                cx.notify();
            }
            Event::Playing => {
                if std::mem::replace(&mut page.told, true) { return; }
                let title = Some(page.title.clone()).filter(|t| !t.is_empty())
                    .or_else(|| self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.url()).map(|u| u.split_once("://").map_or(u, |(_, r)| r).split(['/', '?', '#']).next().unwrap_or(u).to_owned()))
                    .unwrap_or_else(|| id.clone());
                self.toast(Toast { title: format!("{title} is still playing"), body: None, level: Level::Info, action: Some(("pane.focus".into(), json!({"pane": id}))) }, window, cx);
            }
            Event::Message(body) => {
                if !std::mem::take(&mut page.picking) { return; }
                let Ok(message) = serde_json::from_str::<Value>(&body) else { return };
                if let Some(error) = message["error"].as_str() {
                    tracing::warn!("browser pane {id}: {error}");
                    self.toast(Toast { title: "Pick failed".into(), body: Some(error.into()), level: Level::Error, action: None }, window, cx);
                } else if message["pick"].is_object() {
                    self.browser_picked(&id, &message["pick"], window, cx);
                }
            }
        }
        cx.notify();
    }

    /// Marks the page loading. wry reports no failed load (a refused connection sends no
    /// event at all), so a page still loading after LOAD_LIMIT is answered anyway; the timer
    /// runs once per load.
    fn begin_load(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(id) else { return };
        page.loading = true;
        page.loads += 1;
        let (load, id) = (page.loads, id.to_owned());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LOAD_LIMIT).await;
            let _ = this.update(cx, |this, cx| {
                if this.canvas_state.browsers.pages.get(&id).is_some_and(|p| p.loading && p.loads == load) { this.page_ready(id, cx); }
            });
        }).detach();
    }

    /// A loaded page has settled: answer what waited for it, start a pending pick, and
    /// refresh the snapshot of a resting page.
    fn page_ready(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(&id) else { return };
        page.loading = false;
        let waiting = std::mem::take(&mut page.waiting);
        let pick = std::mem::take(&mut page.pick) && page.live.is_some();
        let resting = page.live.is_none();
        self.note_browser_url(&id, cx);
        for ask in waiting { self.answer_browser(&id, ask); }
        if pick { self.eval_browser(&id, "window.__canopy && window.__canopy.pick()"); }
        if resting { self.rest_browser(&id, cx); }
        cx.notify();
    }

    /// Saves the URL the page navigated to.
    fn note_browser_url(&mut self, id: &str, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            let Some(url) = self.raw_browser(id).and_then(|raw| page_url(raw.raw())) else { return };
            if let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id)
                && let canvas::Kind::Browser { url: saved, .. } = &mut pane.kind
                && *saved != url && web_url(&url).is_ok()
            {
                *saved = url;
                self.canvas_changed(cx);
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (id, cx);
    }

    /// A picked element becomes a quote. The page rests and the quote popover opens.
    fn browser_picked(&mut self, id: &str, pick: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id).cloned() else { return };
        // Where the source may live: the folders of agents wired to this pane, its space's
        // folder, then every space's folder.
        let wires = crate::wire::all(&self.canvas_state.doc, |k| self.live.get(k).and_then(|r| r["parent"].as_str()).map(str::to_owned));
        let mut roots: Vec<std::path::PathBuf> = crate::wire::neighbors(&wires, id).iter().filter_map(|t| self.cwds.get(t)).map(std::path::PathBuf::from).collect();
        if let Some(space) = pane.space.as_deref().and_then(|s| self.model.space(s)) { roots.push(space.folder.clone().into()); }
        roots.extend(self.model.spaces.iter().map(|s| std::path::PathBuf::from(&s.folder)));
        let text = pick["text"].as_str().unwrap_or_default();
        let mut quote = json!({"kind":"browser","pane":id,"text":text,"path":pick["url"],
            "source":{"url":pick["url"],"selector":pick["selector"],"tag":pick["tag"]}});
        if let (Some(file), Some(line)) = (pick["file"].as_str(), pick["line"].as_u64()) {
            quote["source"]["component"] = pick["component"].clone();
            quote["start"] = json!(line);
            match resolve_source(file, &roots) {
                Some(path) => {
                    let col = pick["column"].as_u64().unwrap_or(1) as usize;
                    // A file over 512 KiB is not scanned: the quote keeps the start line only.
                    let range = std::fs::metadata(&path).ok().filter(|m| m.len() <= 512 * 1024)
                        .and_then(|_| std::fs::read_to_string(&path).ok())
                        .and_then(|src| jsx_lines(&src, line as usize, col));
                    if let Some((start, end)) = range { quote["start"] = json!(start); quote["end"] = json!(end); }
                    quote["source"]["file"] = json!(path);
                    quote["path"] = json!(path);
                }
                None => quote["source"]["file"] = json!(file),
            }
        }
        // Return to the pane before placing the quote popover.
        if self.canvas_state.browser_tab.as_deref() == Some(id) { self.collapse_browser(window, cx); } else { self.rest_browser(id, cx); }
        let camera = self.canvas_state.doc.camera;
        let origin = self.bounds.borrow().get("canopy-canvas").map(|b| b.origin).unwrap_or_default();
        let [x, y] = camera.to_screen([pane.rect[0] + 16., pane.rect[1] + canvas::header_world(camera.zoom)]);
        quote["point"] = json!({"x": f32::from(origin.x) + x, "y": f32::from(origin.y) + y});
        tracing::debug!("browser pick in {id}: {}", format_quote(&quote, None));
        self.quote_menu(&quote, window, cx);
    }

    /// The browser pane a `canvas.browser_*` command acts on: `pane`, else the one selected.
    fn browser_target(&self, a: &Value) -> Result<String, String> {
        let doc = &self.canvas_state.doc;
        let id = a["pane"].as_str().map(str::to_owned).or_else(|| (self.canvas_state.selected.len() == 1).then(|| self.canvas_state.selected[0].clone()))
            .ok_or("select a browser pane first")?;
        match doc.panes.iter().find(|p| p.id == id) {
            Some(p) if p.url().is_some() => Ok(id),
            Some(p) => Err(format!("pane {id} is a {}, not a browser pane", p.kind_name())),
            None => Err(format!("no pane {id}")),
        }
    }

    /// canvas.browser_back | forward | reload | home | pick | shot | external | copy | viewport
    /// | expand | pop | cookies | clear | inspect.
    pub(in crate::window) fn browser_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        if name == "canvas.browser_cookies" {
            let Ok(keep) = serde_json::from_value::<settings::KeepCookies>(a["keep"].clone()) else {
                return self.fail("keep must be until_quit or forever");
            };
            return self.write_person(|person| person.browser_keep_cookies = keep, window, cx);
        }
        let id = match self.browser_target(a) { Ok(id) => id, Err(e) => return self.fail(e) };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { url, home, .. } = pane.kind.clone() else { return };
        if url.is_empty() && matches!(name, "canvas.browser_home" | "canvas.browser_reload" | "canvas.browser_back" | "canvas.browser_forward") { return; }
        match name {
            "canvas.browser_external" => {
                if url.is_empty() { return self.fail(ENTER_URL); }
                crate::links::open(&crate::links::Target::Url(url));
            }
            "canvas.browser_home" => self.load_browser(&id, &home, false, window, cx),
            "canvas.browser_reload" => self.eval_or_load(&id, "location.reload()", &url, window, cx),
            "canvas.browser_back" => self.eval_or_load(&id, "history.back()", &url, window, cx),
            "canvas.browser_forward" => self.eval_or_load(&id, "history.forward()", &url, window, cx),
            "canvas.browser_copy" => {
                if url.is_empty() { return self.fail(ENTER_URL); }
                cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
                self.toast(Toast { title: "Link copied".into(), body: Some(url), level: Level::Success, action: None }, window, cx);
            }
            "canvas.browser_viewport" => match parse_viewport(a["size"].as_str()) {
                Ok(viewport) => self.set_browser_viewport(&id, viewport, cx),
                Err(e) => self.fail(e),
            },
            "canvas.browser_expand" => {
                let expanded = self.canvas_state.browser_tab.as_deref() == Some(id.as_str());
                self.collapse_browser(window, cx);
                if !expanded && let Err(e) = self.expand_browser(&id, window, cx) { self.fail(e); }
            }
            "canvas.browser_pop" => {
                if let Err(e) = self.pop_browser(&id, window, cx) { self.fail(e); }
            }
            "canvas.browser_inspect" => {
                #[cfg(target_os = "macos")]
                if let Err(e) = self.inspect_browser(&id, window, cx) { self.fail(e); }
                #[cfg(not(target_os = "macos"))]
                self.fail("browser panes run on macOS only for now");
            }
            "canvas.browser_clear" => {
                if a["confirmed"].as_bool() != Some(true) {
                    return self.confirm("Clear browsing data?", "Cookies, storage and cache for every browser pane are deleted. Sites will ask you to sign in again.", "Clear", name, json!({"pane": id, "confirmed": true}), window, cx);
                }
                if let Err(e) = self.clear_browsing_data(window, cx) { self.fail(e); }
            }
            "canvas.browser_pick" => {
                // Another page expanded gives way; the expanded page itself is already live.
                if self.canvas_state.browser_tab.as_deref() != Some(id.as_str()) {
                    self.collapse_browser(window, cx);
                    if let Err(e) = self.wake_browser_result(&id, window, cx) { return self.fail(e); }
                }
                let page = self.canvas_state.browsers.pages.get_mut(&id).unwrap();
                page.picking = true;
                if page.loading { page.pick = true; } else { self.eval_browser(&id, "window.__canopy && window.__canopy.pick()"); }
            }
            _ => {
                // canvas.browser_shot: the person gets the path in a toast.
                let path = std::env::temp_dir().join("canopy").join(format!("{id}.png"));
                let (tx, rx) = std::sync::mpsc::channel();
                if let Err(e) = self.browser_ask(&id, Ask::Shot(path, tx), window, cx) { return self.fail(e); }
                cx.spawn_in(window, async move |this, cx| {
                    let reply = cx.background_executor().spawn(async move { rx.recv().unwrap_or_else(|_| json!({"error":"the page went away"})) }).await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        let note = match reply["path"].as_str() {
                            Some(path) => Toast { title: "Screenshot saved".into(), body: Some(path.into()), level: Level::Info, action: None },
                            None => Toast { title: "Screenshot failed".into(), body: reply["error"].as_str().map(str::to_owned), level: Level::Error, action: None },
                        };
                        this.toast(note, window, cx);
                    });
                }).detach();
            }
        }
    }

    /// A browser pane's helper bar labels for its state: bring back a popped page. (An expanded
    /// page hides its pane and bar, so no Collapse label is needed.)
    pub(super) fn browser_relabel(&self, id: &str, mut actions: Vec<super::helper_bar::BarAction>) -> Vec<super::helper_bar::BarAction> {
        let popped = self.canvas_state.browsers.pages.get(id).is_some_and(|p| p.popped.is_some());
        for a in actions.iter_mut().filter(|a| popped && a.command == "canvas.browser_pop") { a.label = "Bring back".into(); }
        actions
    }

    /// Saves the CSS viewport and updates a live pane's zoom at its current size.
    fn set_browser_viewport(&mut self, id: &str, size: Option<[f32; 2]>, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        let Some(pane) = state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
        let canvas::Kind::Browser { viewport, .. } = &mut pane.kind else { return };
        *viewport = size;
        if let Some(shown) = state.shown.as_mut().and_then(|doc| doc.panes.iter_mut().find(|p| p.id == id))
            && let canvas::Kind::Browser { viewport, .. } = &mut shown.kind
        {
            *viewport = size;
        }
        #[cfg(target_os = "macos")]
        if state.browser_tab.is_none()
            && let Some(page) = state.browsers.pages.get(id).filter(|p| p.live.is_some())
            && let Some(native) = &page.native
        {
            let camera = state.doc.camera;
            let pane = state.shown.as_ref().and_then(|doc| doc.panes.iter().find(|p| p.id == id)).unwrap_or(pane);
            let [_, _, w, h] = pane_body(pane, camera);
            let zoom = page_zoom(pane, camera, [w, h]);
            if zoom > 0. { let _ = native.raw.raw().zoom(zoom); }
        }
        self.canvas_changed(cx);
        cx.notify();
    }

    /// Runs script in a page that has a view; else loads `url` (a reload with no view is a load).
    fn eval_or_load(&mut self, id: &str, script: &str, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.eval_browser(id, script) { self.load_browser(id, url, false, window, cx); }
    }

    /// `canvas.pane_read` and `canvas.pane_shot` for a browser pane: answered once the page
    /// has loaded, through the held control reply. Err when there is no caller to answer.
    pub(super) fn browser_reply(&mut self, id: &str, shot: Option<std::path::PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.held_reply.is_none() { return Err("a browser pane answers this only to `canopy pane` (an agent or script)".into()); }
        if self.browser_url(id)?.is_empty() {
            if shot.is_some() { return Err(ENTER_URL.into()); }
            let tx = self.held_reply.take().unwrap();
            let _ = tx.send(json!({"text":canvas::envelope(id, "browser", &[("url", "")], "New tab")}));
            return Ok(());
        }
        // Make the view first: an error must still reach the held caller.
        self.ensure_view(id, window, cx)?;
        let tx = self.held_reply.take().unwrap();
        let ask = match shot { Some(path) => Ask::Shot(path, tx), None => Ask::Read(tx) };
        self.browser_ask(id, ask, window, cx)
    }

    /// Answers now when the page is loaded, else when it finishes; makes the view if needed.
    fn browser_ask(&mut self, id: &str, ask: Ask, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        self.ensure_view(id, window, cx)?;
        let page = self.canvas_state.browsers.pages.get_mut(id).unwrap();
        if page.loading { page.waiting.push(ask); } else { self.answer_browser(id, ask); }
        Ok(())
    }

    /// `canvas.pane_open`: loads `target` in the pane and makes it its Home.
    pub(super) fn browser_open(&mut self, id: &str, target: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let url = web_url(target)?;
        self.load_browser(id, &url, true, window, cx);
        Ok(json!({"pane":id,"url":url}))
    }

    /// Loads `url`, saving it (and as Home when `home`); toasts when the view cannot be made.
    fn load_browser(&mut self, id: &str, url: &str, home: bool, window: &mut Window, cx: &mut Context<Self>) {
        if url.is_empty() { return; }
        if let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id)
            && let canvas::Kind::Browser { url: saved, home: saved_home, .. } = &mut pane.kind
        {
            *saved = url.to_owned();
            if home { *saved_home = url.to_owned(); }
            self.canvas_changed(cx);
        }
        let made = self.canvas_state.browsers.pages.get(id).is_some_and(Page::has_view);
        if let Err(e) = self.ensure_view(id, window, cx) {
            self.toast(Toast { title: "Page not opened".into(), body: Some(e), level: Level::Error, action: None }, window, cx);
            return;
        }
        // A new view already loads the saved URL.
        if made { self.navigate_browser(id, url, cx); }
        cx.notify();
    }

    /// The click on a resting page.
    fn wake_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = self.wake_browser_result(id, window, cx) {
            self.toast(Toast { title: "Page not opened".into(), body: Some(e), level: Level::Error, action: None }, window, cx);
        }
    }
}

impl Page {
    /// The live view as an element filling its parent.
    fn view_element(&self) -> Option<AnyElement> {
        #[cfg(target_os = "macos")]
        return self.native.as_ref().map(|n| div().size_full().child(n.view.clone()).into_any_element());
        #[cfg(not(target_os = "macos"))]
        None
    }

    fn has_view(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.native.is_some();
        #[cfg(not(target_os = "macos"))]
        false
    }
}

/// The page's URL; None before any page committed. (wry's own `url()` panics then.)
#[cfg(target_os = "macos")]
fn page_url(view: &wry::WebView) -> Option<String> {
    use wry::WebViewExtMacOS as _;
    // SAFETY: called on the main thread with a live WKWebView.
    unsafe { view.webview().URL()?.absoluteString() }.map(|s| s.to_string())
}

/// Closes a page for good: pauses its media, navigates it away, takes the WKWebView off the
/// window, then drops both handles. Dropping alone only hides the view (gpui-wry) and detaches
/// it (wry), so a page that was playing would keep playing.
#[cfg(target_os = "macos")]
fn destroy(native: Native) {
    use wry::WebViewExtMacOS as _;
    let wv = native.raw.raw();
    let _ = wv.evaluate_script("document.querySelectorAll('video,audio').forEach(m => m.pause())");
    let _ = wv.load_url("about:blank");
    wv.webview().removeFromSuperview();
}

/// Lays a resting view out at its pane's body size, zoom 1. gpui-wry sizes a view only while it
/// is drawn; a resting page still lays out (read) and snapshots (shot) this way.
#[cfg(target_os = "macos")]
fn fit_view(view: &gpui_wry::WebView, rect: canvas::Rect) {
    let _ = view.raw().set_bounds(wry::Rect {
        position: wry::dpi::LogicalPosition::new(0., 0.).into(),
        size: wry::dpi::LogicalSize::new(rect[2] as f64, (rect[3] - canvas::HEADER).max(1.) as f64).into(),
    });
    let _ = view.raw().zoom(1.);
}

/// A popped-out page's window: one web view, no pick mode and no link to the canvas.
#[cfg(target_os = "macos")]
struct BrowserWindow { native: Option<Native> }

#[cfg(target_os = "macos")]
impl Render for BrowserWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(gpui::white()).children(self.native.as_ref().map(|n| n.view.clone()))
    }
}

/// Snapshots a page into `done` on the main thread: RGBA at the screen's backing scale, None
/// when WebKit fails.
#[cfg(target_os = "macos")]
fn snapshot(view: &wry::WebView, done: impl FnOnce(Option<image::RgbaImage>) + 'static) {
    use objc2_app_kit::NSImage;
    use objc2_foundation::NSError;
    use wry::WebViewExtMacOS as _;
    let done = std::cell::Cell::new(Some(done));
    let block = block2::RcBlock::new(move |image: *mut NSImage, _: *mut NSError| {
        let Some(done) = done.take() else { return };
        // SAFETY: WebKit passes a valid NSImage or null.
        let tiff = unsafe { image.as_ref() }.and_then(|image| image.TIFFRepresentation());
        done(tiff.and_then(|data| image::load_from_memory_with_format(&data.to_vec(), image::ImageFormat::Tiff).ok()).map(|i| i.to_rgba8()));
    });
    // SAFETY: called on the main thread with a live WKWebView; WebKit copies the block.
    unsafe { view.webview().takeSnapshotWithConfiguration_completionHandler(None, &block) };
}

#[cfg(target_os = "macos")]
impl AppWindow {
    /// Makes the pane's web view (hidden, loading the saved URL) unless it has one.
    // shortcut: a view lives until its pane closes (`destroy`), so each page opened this run keeps
    // a WebKit process and its hidden page's (throttled) scripts; drop views on rest if memory matters.
    fn ensure_view(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        if self.canvas_state.browsers.pages.get(id).is_some_and(Page::has_view) { return Ok(()); }
        if self.canvas_state.browsers.pages.get(id).is_some_and(|p| p.popped.is_some()) { return Err(format!("pane {id} is popped out: bring it back first")); }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id && p.url().is_some()).ok_or_else(|| format!("no browser pane {id}"))?;
        let (url, rect) = (pane.url().unwrap().to_owned(), pane.rect);
        let tx = self.browser_events(window, cx);
        let send = |event: fn(String) -> Event| { let (tx, id) = (tx.clone(), id.to_owned()); move |s: String| { let _ = tx.try_send((id.clone(), event(s))); } };
        let (load_tx, load_id) = (tx.clone(), id.to_owned());
        let (open_tx, open_id) = (tx.clone(), id.to_owned());
        let ipc = send(Event::Message);
        let webview = browser_store(wry::WebViewBuilder::new())
            .with_url(url)
            .with_devtools(true)
            .with_visible(false)
            .with_accept_first_mouse(true)
            .with_initialization_script_for_main_only(page_script(), true)
            .with_ipc_handler(move |request| ipc(request.body().clone()))
            .with_document_title_changed_handler(send(Event::Title))
            .with_on_page_load_handler(move |event, _| {
                let event = match event { wry::PageLoadEvent::Started => Event::Started, wry::PageLoadEvent::Finished => Event::Loaded };
                let _ = load_tx.try_send((load_id.clone(), event));
            })
            .with_new_window_req_handler(move |url, _| {
                let _ = open_tx.try_send((open_id.clone(), Event::Open(url)));
                wry::NewWindowResponse::Deny
            })
            // No downloads: a pane shows pages, it does not save files.
            .with_download_started_handler(|_, _| false)
            .build_as_child(window)
            .map_err(|e| format!("cannot make the web view: {e}"))?;
        let view = cx.new(|cx| gpui_wry::WebView::new(webview, window, cx));
        let raw = view.update(cx, |v, _| {
            v.hide();
            fit_view(v, rect);
            v.handle()
        });
        let page = self.canvas_state.browsers.pages.entry(id.to_owned()).or_default();
        page.native = Some(Native { view, raw });
        self.begin_load(id, cx);
        Ok(())
    }

    /// Expands a page into the top strip's tab: live over the whole canvas area, at zoom 1. The
    /// canvas then draws nothing else (`render_canvas`). Refuses a popped page, or one in
    /// another space.
    fn expand_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).cloned().ok_or_else(|| format!("no pane {id}"))?;
        if !self.canvas_state.doc.shows(pane.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        self.require_browser_url(id)?;
        let frame = self.bounds.borrow().get("canopy-canvas").copied().ok_or("the canvas is not shown")?;
        self.ensure_view(id, window, cx)?;
        // The expanded page replaces all panes, so the other live pages step aside.
        let others: Vec<String> = self.canvas_state.browsers.pages.iter().filter(|(k, p)| k.as_str() != id && p.live.is_some()).map(|(k, _)| k.clone()).collect();
        for other in others { self.rest_browser(&other, cx); }
        let camera = self.canvas_state.doc.camera;
        let page = self.canvas_state.browsers.pages.get_mut(id).unwrap();
        page.live = Some((camera, pane.rect));
        (page.told, page.watch) = (false, false);
        page.native.as_ref().unwrap().view.update(cx, |v, _| {
            show_view(v, frame, 1.);
        });
        self.canvas_state.browser_tab = Some(id.to_owned());
        self.canvas_state.selected = vec![id.to_owned()];
        cx.notify();
        Ok(())
    }

    /// Puts the expanded page (if any) back in its pane: laid out at the pane's size again, then
    /// rested the usual way. Called by every way out: the command, Escape, any space tab, close.
    pub(super) fn collapse_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.canvas_state.browser_tab.take() else { return };
        let rect = self.canvas_state.doc.panes.iter().find(|p| p.id == id).map(|p| p.rect);
        if let (Some(rect), Some(native)) = (rect, self.canvas_state.browsers.pages.get(&id).and_then(|p| p.native.as_ref())) {
            native.view.update(cx, |v, _| fit_view(v, rect));
        }
        // shortcut: the snapshot is taken right after the resize, so it may predate WebKit's relayout; a rested page re-snapshots when it next loads.
        self.rest_browser(&id, cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Pops a page out into its own window, or brings it back. The pane's own view is destroyed
    /// meanwhile and the pane says so; the window loads the pane's current URL in a fresh,
    /// private web view and is not tracked after that.
    fn pop_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if let Some(popped) = self.canvas_state.browsers.pages.get_mut(id).and_then(|p| p.popped.take()) {
            let _ = popped.update(cx, |_, window, _| window.remove_window());
            return Ok(cx.notify());
        }
        self.require_browser_url(id)?;
        if self.canvas_state.browser_tab.as_deref() == Some(id) { self.collapse_browser(window, cx); }
        self.note_browser_url(id, cx);
        self.canvas_state.browsers.last_masks.borrow_mut().remove(id);
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id && p.url().is_some()).cloned().ok_or_else(|| format!("no browser pane {id}"))?;
        let (title, _) = self.browser_title(&pane);
        let page = self.canvas_state.browsers.pages.get_mut(id).ok_or_else(|| format!("no browser pane {id}"))?;
        if let Some(native) = page.native.take() { destroy(native); }
        (page.live, page.picking, page.pick, page.loading, page.watch) = (None, false, false, false, false);
        for ask in std::mem::take(&mut page.waiting) {
            let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
            let _ = tx.send(json!({"error": "the page was popped out"}));
        }
        let url = pane.url().unwrap().to_owned();
        let owner = window.window_handle().downcast::<AppWindow>().unwrap();
        let (closed, id) = (id.to_owned(), id.to_owned());
        let failed = std::rc::Rc::new(std::cell::RefCell::new(None::<String>));
        let build_failed = failed.clone();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(pane.rect[2]), px(pane.rect[3])), cx))),
            ..Default::default()
        };
        // Opening draws the window at once: do it once this update has ended.
        cx.defer(move |cx| {
            let opened = cx.open_window(options, move |window, cx| {
                window.set_window_title(&title);
                window.on_window_should_close(cx, move |_, cx| {
                    let _ = owner.update(cx, |this, _, cx| {
                        if let Some(page) = this.canvas_state.browsers.pages.get_mut(&closed) { page.popped = None; }
                        cx.notify();
                    });
                    true
                });
                let built = browser_store(wry::WebViewBuilder::new())
                    .with_url(url)
                    .with_devtools(true)
                    .with_accept_first_mouse(true)
                    .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
                    .with_download_started_handler(|_, _| false)
                    .build_as_child(window);
                let native = match built {
                    Ok(webview) => {
                        let view = cx.new(|cx| gpui_wry::WebView::new(webview, window, cx));
                        let raw = view.update(cx, |v, _| v.handle());
                        Some(Native { view, raw })
                    }
                    Err(e) => { *build_failed.borrow_mut() = Some(e.to_string()); None }
                };
                cx.new(|cx| {
                    cx.on_release(|this: &mut BrowserWindow, _| { if let Some(native) = this.native.take() { destroy(native); } }).detach();
                    BrowserWindow { native }
                })
            });
            let _ = owner.update(cx, |this, window, cx| {
                let error = match (opened, failed.take()) {
                    (Ok(handle), None) => {
                        if let Some(page) = this.canvas_state.browsers.pages.get_mut(&id) { page.popped = Some(handle.into()); }
                        return cx.notify();
                    }
                    (Ok(handle), Some(e)) => { let _ = handle.update(cx, |_, window, _| window.remove_window()); e }
                    (Err(e), _) => e.to_string(),
                };
                this.toast(Toast { title: "Pop out failed".into(), body: Some(error), level: Level::Error, action: None }, window, cx);
            });
        });
        cx.notify();
        Ok(())
    }

    /// Clears data in open views, including pop-outs, and asks WebKit to remove the named store.
    fn clear_browsing_data(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let mut views = Vec::new();
        for page in self.canvas_state.browsers.pages.values() {
            if let Some(native) = &page.native {
                views.push(native.raw.clone());
            }
            if let Some(popped) = page.popped.and_then(|window| window.downcast::<BrowserWindow>())
                && let Ok(Some(view)) = popped.update(cx, |page, _, _| page.native.as_ref().map(|native| native.raw.clone()))
            {
                views.push(view);
            }
        }
        // shortcut: lb-wry does not expose completion for clearing live views, wait for it if the API adds a callback.
        let mut failure = None;
        for view in &views {
            if let Err(e) = view.raw().clear_all_browsing_data() { failure = Some(e); }
        }
        let result = persistent_browser_store().then(remove_browser_store);
        let has_views = !views.is_empty();
        let cleared = Toast { title: "Browsing data cleared".into(), body: None, level: Level::Success, action: None };
        if let Some(e) = failure { return Err(format!("cannot clear browsing data: {e}")); }
        if let Some(result) = result {
            cx.spawn_in(window, async move |this, cx| {
                let error = match result.recv().await {
                    Ok(Ok(())) => None,
                    Ok(Err(wry::Error::DataStoreInUse)) if has_views => None,
                    Ok(Err(e)) => Some(e.to_string()),
                    Err(e) => Some(e.to_string()),
                };
                let _ = this.update_in(cx, |this, window, cx| match error {
                    Some(e) => this.command_error("canvas.browser_clear", &e, window, cx),
                    None => this.toast(cleared, window, cx),
                });
            }).detach();
        } else {
            self.toast(cleared, window, cx);
        }
        Ok(())
    }

    /// The channel web views report into, and the task that hands its events to the window.
    /// The task waits on the channel: nothing runs while pages are quiet.
    fn browser_events(&mut self, window: &mut Window, cx: &mut Context<Self>) -> async_channel::Sender<(String, Event)> {
        if let Some(tx) = &self.canvas_state.browsers.events { return tx.clone(); }
        let (tx, rx) = async_channel::unbounded::<(String, Event)>();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok((id, event)) = rx.recv().await {
                if this.update_in(cx, |this, window, cx| this.browser_event(id, event, window, cx)).is_err() { break; }
            }
        }).detach();
        self.canvas_state.browsers.events = Some(tx.clone());
        tx
    }

    /// Makes the page live at the pane's current scale and window position.
    fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        let doc = self.canvas_state.shown.as_ref().unwrap_or(&self.canvas_state.doc);
        let pane = doc.panes.iter().find(|p| p.id == id).cloned().ok_or_else(|| format!("no pane {id}"))?;
        if !doc.shows(pane.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        let frame = self.body_rect(&pane).ok_or("the canvas is not shown")?;
        let camera = self.canvas_state.doc.camera;
        let [_, _, width, height] = pane_body(&pane, camera);
        let zoom = page_zoom(&pane, camera, [width, height]);
        self.ensure_view(id, window, cx)?;
        let page = self.canvas_state.browsers.pages.get_mut(id).unwrap();
        page.live = Some(placement(&pane, camera));
        (page.told, page.watch) = (false, false);
        page.native.as_ref().unwrap().view.update(cx, |v, _| show_view(v, frame, zoom));
        self.canvas_state.selected = vec![id.to_owned()];
        self.canvas_changed(cx);
        cx.notify();
        Ok(())
    }

    pub(super) fn rest_live_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.canvas_state.browsers.pages.get(id).is_some_and(|page| page.live.is_some()) {
            self.rest_browser(id, cx);
        }
    }

    /// Steps a live page aside: snapshot, then hide (in `Event::Rested`), so the pane never
    /// shows blank. A resting page with a view only refreshes its snapshot.
    pub(super) fn rest_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(id) else { return };
        let Some(view) = page.native.as_ref().map(|n| n.view.clone()) else { return };
        page.live = None;
        self.canvas_state.browsers.last_masks.borrow_mut().remove(id);
        // A pick not finished before the page rests is over.
        page.picking = false;
        page.watch = true;
        let Some(tx) = self.canvas_state.browsers.events.clone() else { return };
        let owned = id.to_owned();
        view.update(cx, |v, _| { let _ = v.focus_parent(); snapshot(v.raw(), move |shot| { let _ = tx.try_send((owned, Event::Rested(shot))); }); });
        self.note_browser_url(id, cx);
        cx.notify();
    }

    /// Checks, once, each resting page that is no longer drawn (another place, or off the
    /// canvas view) for media still playing; a playing one is toasted (`Event::Playing`).
    pub(super) fn watch_pages(&self, drawn: &[String], cx: &mut Context<Self>) {
        let unseen: Vec<String> = self.canvas_state.browsers.pages.iter()
            .filter(|(id, p)| p.watch && !p.told && !drawn.contains(id)).map(|(id, _)| id.clone()).collect();
        if unseen.is_empty() { return; }
        let this = cx.entity();
        cx.defer(move |cx| this.update(cx, |this, _| for id in unseen {
            let Some(page) = this.canvas_state.browsers.pages.get_mut(&id) else { continue };
            page.watch = false;
            let (Some(raw), Some(tx)) = (page.native.as_ref().map(|n| n.raw.clone()), this.canvas_state.browsers.events.clone()) else { continue };
            let script = "[...document.querySelectorAll('video,audio')].some(m => !m.paused && !m.ended && m.readyState > 2)";
            let _ = raw.raw().evaluate_script_with_callback(script, move |result| {
                if result.trim() == "true" { let _ = tx.try_send((id.clone(), Event::Playing)); }
            });
        }));
    }

    /// Runs a script in the page; false when the pane has no view.
    fn eval_browser(&mut self, id: &str, script: &str) -> bool {
        let Some(raw) = self.raw_browser(id) else { return false };
        let _ = raw.raw().evaluate_script(script);
        true
    }

    fn raw_browser(&self, id: &str) -> Option<gpui_wry::WebViewHandle> {
        self.canvas_state.browsers.pages.get(id)?.native.as_ref().map(|n| n.raw.clone())
    }

    fn navigate_browser(&mut self, id: &str, url: &str, cx: &mut Context<Self>) {
        let Some(raw) = self.raw_browser(id) else { return };
        self.begin_load(id, cx);
        let _ = raw.raw().load_url(url);
    }

    /// Answers one held reply from the loaded page.
    fn answer_browser(&mut self, id: &str, ask: Ask) {
        let Some(raw) = self.raw_browser(id) else { return };
        // No page ever committed (the server refused or never answered): say so, since
        // WebKit would drop the script callback and the caller would hear nothing.
        if page_url(raw.raw()).is_none() {
            let url = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.url()).unwrap_or_default();
            let error = json!({"error": format!("{url} did not load: is its server running?")});
            let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
            let _ = tx.send(error);
            return;
        }
        let id = id.to_owned();
        match ask {
            Ask::Read(tx) => {
                let tx = std::sync::Mutex::new(Some(tx));
                let script = "JSON.stringify({url: location.href, title: document.title, text: document.body ? document.body.innerText : ''})";
                let _ = raw.raw().evaluate_script_with_callback(script, move |result| {
                    let Some(tx) = tx.lock().unwrap().take() else { return };
                    // wry hands back the script's value as JSON: here a JSON string of JSON.
                    let page = serde_json::from_str::<String>(&result).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or_default();
                    let s = |k: &str| page[k].as_str().unwrap_or_default().to_owned();
                    let text = canvas::envelope(&id, "browser", &[("url", &s("url")), ("title", &s("title"))], &s("text"));
                    let _ = tx.send(json!({"text": text}));
                });
            }
            Ask::Shot(path, tx) => {
                snapshot(raw.raw(), move |shot| {
                    let reply = match shot {
                        None => json!({"error": "WebKit could not take the snapshot"}),
                        Some(rgba) => {
                            let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).map_err(anyhow::Error::from)
                                .and_then(|_| crate::control::save_png(&rgba, &path));
                            match saved { Ok(()) => json!({"path": path}), Err(e) => json!({"error": format!("cannot save the shot: {e:#}")}) }
                        }
                    };
                    let _ = tx.send(reply);
                });
            }
        }
    }

    /// Give an unpinned browser room to work at 100%, or fit it with a margin.
    pub(super) fn snap_usable(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(bounds) = self.bounds.borrow().get("canopy-canvas").copied() else { return };
        let [w, h] = [f32::from(bounds.size.width), f32::from(bounds.size.height)];
        if w <= 0. || h <= 0. { return; }
        let doc = &mut self.canvas_state.doc;
        let Some(pane) = doc.panes.iter_mut().find(|p| p.id == id && p.pinned.is_none()) else { return };
        let [x, y, pw, ph] = pane.rect;
        let (pw, ph) = (pw.max(SIZE[0]), ph.max(SIZE[1]));
        pane.rect = [x, y, pw, ph];
        let zoom = if pw <= w && ph <= h { 1. } else { ((w - 48.).max(1.) / pw).min((h - 48.).max(1.) / ph) };
        doc.camera = canvas::Camera { x: x + (pw - w / zoom) / 2., y: y + (ph - h / zoom) / 2., zoom };
        self.canvas_changed(cx);
        cx.notify();
    }

    /// Move the pane before making its native view live, since movement rests a live view.
    fn inspect_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.canvas_state.browsers.pages.get(id).is_some_and(|p| p.popped.is_some()) {
            return Err("the page is popped out: bring it back first".into());
        }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        if !self.canvas_state.doc.shows(pane.space.as_ref()) {
            return Err(format!("pane {id} is in another space: go there first"));
        }
        self.ensure_view(id, window, cx)?;
        let raw = self.raw_browser(id).ok_or("the browser view went away")?;
        {
            use objc2::{runtime::NSObjectProtocol as _, sel};
            use wry::WebViewExtMacOS as _;
            if !raw.raw().webview().respondsToSelector(sel!(_inspector)) {
                return Err("The Web Inspector is not available in this version of macOS".into());
            }
        }
        self.collapse_browser(window, cx);
        let from = self.canvas_state.doc.camera;
        self.snap_usable(id, cx);
        let to = self.canvas_state.doc.camera;
        if from == to { return self.open_browser_inspector(id, window, cx); }
        let bounds = self.bounds.borrow().get("canopy-canvas").copied().ok_or("the canvas is not shown")?;
        let anchor = [f32::from(bounds.size.width) / 2., f32::from(bounds.size.height) / 2.];
        self.canvas_state.doc.camera = from;
        let motion = (from, to, std::time::Instant::now(), anchor);
        self.canvas_state.motion.target = Some(motion);
        self.animate_camera(window, cx);
        self.inspect_after_camera(id.to_owned(), motion.2, to, window, cx);
        Ok(())
    }

    fn inspect_after_camera(&self, id: String, started: std::time::Instant, target: canvas::Camera, window: &Window, cx: &Context<Self>) {
        let entity = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = entity.update(cx, |this, cx| match this.canvas_state.motion.target {
                Some((_, _, time, _)) if time == started => this.inspect_after_camera(id, started, target, window, cx),
                None if this.canvas_state.doc.camera == target => {
                    if let Err(e) = this.open_browser_inspector(&id, window, cx) {
                        this.command_error("canvas.browser_inspect", &e, window, cx);
                    }
                }
                _ => {}
            });
        });
    }

    fn open_browser_inspector(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let camera = self.canvas_state.doc.camera;
        let pinned = self.canvas_state.doc.panes.iter().any(|p| p.id == id && p.pinned.is_some());
        let wake = self.wake_browser_result(id, window, cx);
        if pinned { self.canvas_state.doc.camera = camera; }
        wake?;
        let raw = self.raw_browser(id).ok_or("the browser view went away")?;
        raw.raw().open_devtools();
        detach_inspector(raw.raw());
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn detach_inspector(view: &wry::WebView) {
    use objc2::{msg_send, runtime::{AnyObject, Bool}, sel};
    use wry::WebViewExtMacOS as _;
    let wkwebview = view.webview();
    // Private WebKit API used by lb-wry too, for builds outside the App Store.
    // SAFETY: runs on the main thread with a live WKWebView checked in inspect_browser.
    unsafe {
        let inspector: *mut AnyObject = msg_send![&*wkwebview, _inspector];
        let detaches = !inspector.is_null() && { let r: Bool = msg_send![&*inspector, respondsToSelector: sel!(detach)]; r.as_bool() };
        if detaches && let Some(inspector) = inspector.as_ref() {
            let _: () = msg_send![inspector, detach];
        }
    }
}

#[cfg(target_os = "macos")]
fn persistent_browser_store() -> bool {
    static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SUPPORTED.get_or_init(|| objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion().majorVersion >= 14)
}

#[cfg(target_os = "macos")]
fn browser_store(builder: wry::WebViewBuilder<'_>) -> wry::WebViewBuilder<'_> {
    use wry::WebViewBuilderExtDarwin as _;
    if persistent_browser_store() { builder.with_data_store_identifier(store_id()) }
    else { builder.with_incognito(true) }
}

#[cfg(target_os = "macos")]
fn store_id() -> [u8; 16] {
    store_id_for(&crate::mods::app_dir())
}

#[cfg(any(target_os = "macos", test))]
fn store_id_for(path: &std::path::Path) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    Sha256::digest(format!("canopy-browser-store:{}", path.display()))[..16].try_into().unwrap()
}

#[cfg(target_os = "macos")]
fn remove_browser_store() -> async_channel::Receiver<wry::Result<()>> {
    use wry::WebViewExtDarwin as _;
    let (send, receive) = async_channel::bounded(1);
    wry::WebView::remove_data_store(&store_id(), move |result| { let _ = send.try_send(result); });
    receive
}

/// Starts removing the named store and returns a future for WebKit's result.
pub(crate) fn forget_browser_data_if_until_quit() -> Option<impl std::future::Future<Output = Result<(), String>>> {
    #[cfg(target_os = "macos")]
    {
        let until_quit = || settings::load_person().is_ok_and(|person| person.browser_keep_cookies == settings::KeepCookies::UntilQuit);
        if !persistent_browser_store() || !until_quit() { return None; }
        let removal = remove_browser_store();
        return Some(async move {
            removal.recv().await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            Ok(())
        });
    }
    #[cfg(not(target_os = "macos"))]
    None::<std::future::Ready<Result<(), String>>>
}

#[cfg(not(target_os = "macos"))]
impl AppWindow {
    fn ensure_view(&mut self, id: &str, _: &mut Window, _: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        Err("browser panes run on macOS only for now".into())
    }
    fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        self.ensure_view(id, window, cx)
    }
    pub(super) fn rest_browser(&mut self, _: &str, _: &mut Context<Self>) {}
    pub(super) fn collapse_browser(&mut self, _: &mut Window, _: &mut Context<Self>) {}
    fn expand_browser(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) -> Result<(), String> {
        Err("browser panes run on macOS only for now".into())
    }
    fn pop_browser(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) -> Result<(), String> {
        Err("browser panes run on macOS only for now".into())
    }
    fn clear_browsing_data(&mut self, _: &mut Window, _: &mut Context<Self>) -> Result<(), String> {
        Err("browser panes run on macOS only for now".into())
    }
    pub(super) fn watch_pages(&self, _: &[String], _: &mut Context<Self>) {}
    fn eval_browser(&mut self, _: &str, _: &str) -> bool { false }
    fn navigate_browser(&mut self, _: &str, _: &str, _: &mut Context<Self>) {}
    fn answer_browser(&mut self, _: &str, _: Ask) {}
}

#[cfg(test)]
mod tests {
    // Explicit: a glob would bring gpui's own `test` attribute in.
    use super::{GRAB_REQUESTS, PRESETS, REACT_GRAB, format_quote, jsx_lines, page_script, page_zoom, parse_viewport, resolve_source, web_url};
    use crate::canvas::{Camera, Kind, Pane};
    use serde_json::json;

    #[test]
    fn browser_store_id_is_stable_and_scoped_to_the_home() {
        use std::path::Path;
        let daily = Path::new("/canopy");
        assert_eq!(super::store_id_for(daily), super::store_id_for(daily));
        assert_ne!(super::store_id_for(daily), super::store_id_for(Path::new("/canopy-dev")));
    }

    #[test]
    fn page_script_drops_react_grab_requests() {
        for (from, _) in GRAB_REQUESTS { assert_eq!(REACT_GRAB.matches(from).count(), 1, "react-grab changed: recheck {from}"); }
        let script = page_script();
        // react-grab.com stays only in a console banner, which fetches nothing.
        for url in ["fonts.googleapis.com", "react-grab.com/api", "react-grab.com/open-file"] { assert!(!script.contains(url), "{url} left in the page script"); }
        assert_eq!(script.matches("react-grab.com").count(), 1);
        assert!(script.starts_with("window.__REACT_GRAB_DISABLED__ = true;"));
    }

    #[test]
    fn viewports_parse_and_zoom_the_page() {
        for (name, _, viewport) in PRESETS {
            assert_eq!(parse_viewport(Some(name)), Ok(viewport));
        }
        assert_eq!(parse_viewport(Some("desktop")), Ok(Some([1280., 800.])));
        assert!(parse_viewport(Some("Mobile")).is_err());
        let camera = Camera { zoom: 0.75, ..Default::default() };
        let mut pane = Pane {
            id: "p1".into(), kind: Kind::Browser { url: String::new(), home: String::new(), viewport: None },
            rect: [0., 0., 640., 838.], z: 1, space: None, pinned: None,
        };
        assert_eq!(page_zoom(&pane, camera, [640., 800.]), 0.75);
        pane.pinned = Some(pane.rect);
        assert_eq!(page_zoom(&pane, camera, [640., 800.]), 1.);
        pane.kind = Kind::Browser { url: String::new(), home: String::new(), viewport: Some([1280., 800.]) };
        assert_eq!(page_zoom(&pane, camera, [640., 800.]), 0.5);
        pane.kind = Kind::Browser { url: String::new(), home: String::new(), viewport: Some([0., 0.]) };
        assert_eq!(page_zoom(&pane, camera, [640., 800.]), 1.);
    }

    #[test]
    fn web_urls() {
        for (input, url) in [
            ("http://localhost:3000/a", "http://localhost:3000/a"), ("HTTPS://x.dev", "HTTPS://x.dev"),
            ("localhost:3000", "http://localhost:3000"), ("127.0.0.1:8080/p?q=1", "http://127.0.0.1:8080/p?q=1"),
            ("app.localhost", "http://app.localhost"), ("[::1]:5173", "http://[::1]:5173"), ("site.test/x", "http://site.test/x"),
            ("example.com", "https://example.com"), (" example.com/a ", "https://example.com/a"),
        ] { assert_eq!(web_url(input).unwrap(), url, "{input}"); }
        for bad in ["", "file:///etc/passwd", "javascript:alert(1)", "data:text/html,x", "http://", "a b", "chrome://settings", "localhost:"] {
            assert!(web_url(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn sources_resolve_under_roots() {
        let dir = std::env::temp_dir().join(format!("canopy-browser-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/App.tsx"), "").unwrap();
        let roots = [dir.clone()];
        let want = Some(dir.join("src/App.tsx"));
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::write(dir.join("node_modules/x/App.tsx"), "").unwrap();
        for file in ["/src/App.tsx", "src/App.tsx?t=1", "http://localhost:5173/src/App.tsx", "webpack://app/./src/App.tsx", "/@fs/elsewhere/proj/src/App.tsx", "App.tsx"] {
            assert_eq!(resolve_source(file, &roots), want, "{file}");
        }
        assert_eq!(resolve_source(dir.join("src/App.tsx").to_str().unwrap(), &roots), want);
        assert_eq!(resolve_source(dir.join("src/App.tsx").to_str().unwrap(), &[]), None, "an absolute path outside every root");
        assert_eq!(resolve_source("/src/Missing.tsx", &roots), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn jsx_ranges() {
        let src = "export function Card() {\n  return (\n    <div className=\"card\" onClick={() => a > b}>\n      {/* a } comment */}\n      <Card.Title title={`x ${y} }`}>hi</Card.Title>\n      <br />\n      <div>\n        <div>nested</div>\n      </div>\n    </div>\n  );\n}\n";
        assert_eq!(jsx_lines(src, 3, 5), Some((3, 10)), "the whole card, over braces, strings and a comment");
        assert_eq!(jsx_lines(src, 3, 1), Some((3, 10)), "a column before the tag finds it");
        assert_eq!(jsx_lines(src, 1, 22), Some((3, 10)), "a component's definition finds the element it returns");
        assert_eq!(jsx_lines("f();\nif (a < b) {}\nconst x = <p>hi</p>;\n", 1, 1), Some((3, 3)), "a comparison is not an element");
        assert_eq!(jsx_lines(src, 5, 7), Some((5, 5)));
        assert_eq!(jsx_lines(src, 6, 7), Some((6, 6)), "self-closing");
        assert_eq!(jsx_lines(src, 7, 7), Some((7, 9)), "same-name nesting");
        assert_eq!(jsx_lines("<>\n  <A />\n</>\n", 1, 1), Some((1, 3)), "fragments");
        assert_eq!(jsx_lines("const f = <T,>(x: T) => x;\n", 1, 11), None, "a generic is not an element");
        assert_eq!(jsx_lines(&format!("<div>\n{}</div>\n", "x\n".repeat(500)), 1, 1), None, "past 400 lines");
        assert_eq!(jsx_lines("no tag here\n", 1, 1), None);
        assert_eq!(jsx_lines("<div>\n", 1, 9), None, "an unclosed element");
        assert_eq!(jsx_lines("<div>\n", 9, 1), None, "line past the end");
    }

    #[test]
    fn quotes_point_not_copy() {
        let react = json!({"start":42,"end":61,"source":{"url":"http://localhost:3000/pricing","component":"PricingCard","file":"/w/src/PricingCard.tsx","tag":format!("<div class=\"card {}\">", "x".repeat(100))}});
        let quote = format_quote(&react, Some("/w"));
        assert!(quote.starts_with("@http://localhost:3000/pricing <PricingCard> src/PricingCard.tsx:42-61 \"<div class=\\\"card xxx"), "{quote}");
        assert!(quote.ends_with("…\""), "the tag is cut: {quote}");
        let dom = json!({"text":"Buy now","message":" fix this ","source":{"url":"https://x.dev/","selector":"main > section:nth-of-type(2)"}});
        assert_eq!(format_quote(&dom, None), "@https://x.dev/ main > section:nth-of-type(2) \"Buy now\"\n\nfix this");
        let one = json!({"start":3,"end":3,"source":{"url":"u","component":"A","file":"/x/A.tsx","tag":"<a>"}});
        assert_eq!(format_quote(&one, Some("/w")), "@u <A> /x/A.tsx:3 \"<a>\"");
    }
}

/// Where a live page sits: the camera and rect, or for a pinned pane just its screen rect, so
/// panning or zooming the canvas under a pinned page leaves it live.
#[cfg(target_os = "macos")]
fn placement(pane: &canvas::Pane, camera: canvas::Camera) -> (canvas::Camera, canvas::Rect) {
    match pane.pinned {
        Some(screen) => (canvas::Camera::default(), screen),
        None => (camera, pane.rect),
    }
}

/// The pane body in canvas screen pixels, excluding the border and fixed-height header.
#[cfg(target_os = "macos")]
fn pane_body(pane: &canvas::Pane, camera: canvas::Camera) -> canvas::Rect {
    let [x, y, w, h] = super::browser_mask::screen_rect(pane, pane.rect, camera);
    [x + 1., y + canvas::HEADER + 1., (w - 2.).max(0.), (h - canvas::HEADER - 2.).max(0.)]
}

/// A fixed viewport fits inside the body at its original aspect ratio.
#[cfg(target_os = "macos")]
fn viewport_rect(pane: &canvas::Pane, camera: canvas::Camera) -> canvas::Rect {
    let [x, y, w, h] = pane_body(pane, camera);
    let Some([css_w, css_h]) = fixed_viewport(pane) else { return [x, y, w, h] };
    let zoom = page_zoom(pane, camera, [w, h]);
    let (page_w, page_h) = ((f64::from(css_w) * zoom) as f32, (f64::from(css_h) * zoom) as f32);
    [x + (w - page_w) / 2., y + (h - page_h) / 2., page_w, page_h]
}

#[cfg(target_os = "macos")]
fn show_view(view: &mut gpui_wry::WebView, frame: Bounds<Pixels>, zoom: f64) {
    if zoom > 0. { let _ = view.raw().zoom(zoom); }
    let _ = view.raw().set_bounds(wry::Rect {
        position: wry::dpi::LogicalPosition::new(f64::from(frame.origin.x), f64::from(frame.origin.y)).into(),
        size: wry::dpi::LogicalSize::new(f64::from(frame.size.width), f64::from(frame.size.height)).into(),
    });
    view.show();
    let _ = view.raw().focus();
}

#[cfg(target_os = "macos")]
impl AppWindow {
    /// Window-space bounds of the live element, including the viewport's centering offset.
    pub(super) fn body_rect(&self, pane: &canvas::Pane) -> Option<Bounds<Pixels>> {
        let origin = self.bounds.borrow().get("canopy-canvas")?.origin;
        let [x, y, w, h] = viewport_rect(pane, self.canvas_state.doc.camera);
        Some(Bounds::new(origin + point(px(x), px(y)), size(px(w), px(h))))
    }

    pub(super) fn live_browser_views(&self) -> impl Iterator<Item = (&str, gpui_wry::WebViewHandle)> {
        self.canvas_state.browsers.pages.iter().filter_map(|(id, page)| {
            page.live?;
            Some((id.as_str(), page.native.as_ref()?.raw.clone()))
        })
    }
}

impl AppWindow {
    /// Retakes a resting browser's snapshot at its settled pane size.
    pub(super) fn resize_resting_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        if let Some(page) = self.canvas_state.browsers.pages.get(id).filter(|p| p.live.is_none())
            && let Some(native) = &page.native
            && let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id)
        {
            let rect = pane.pinned.unwrap_or(pane.rect);
            native.view.update(cx, |v, _| fit_view(v, rect));
            self.rest_browser(id, cx);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (id, cx);
    }
}
