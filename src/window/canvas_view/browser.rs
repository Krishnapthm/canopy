//! Open a browser pane on the canvas and enter an address in its focused New tab field.
//! Each tab has its own WebKit view, created when the page first loads.
//! Click a loaded page to interact with it. WebKit follows pans, zoom and resizes while
//! live, with the canvas edge and foreground panes, helper bars and overlays masked out.
//! Click elsewhere to rest the page and draw its last snapshot.
//!
//! Responsive pages reflow with the pane. Fixed CSS viewport presets scale to fit the
//! pane body and sit centered inside it. See docs/wiki/Browser-Pane.md.
//!
//! The browser header puts tabs and pane controls above navigation, the address field and
//! Annotate. A docked browser puts its upper row in the Files tab strip. A popped
//! pane keeps no views of its own: every tab's page is built in the pop-out window (`BrowserWindow`),
//! and bringing the pane back destroys them, to be rebuilt in the pane when next used.
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

/// The system browser's name (macOS: the app that opens https links), looked up once.
fn default_browser_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        static NAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        return NAME.get_or_init(|| {
            use objc2_app_kit::NSWorkspace;
            use objc2_foundation::{NSString, NSURL};
            let url = NSURL::URLWithString(&NSString::from_str("https://example.com"))?;
            let app = NSWorkspace::sharedWorkspace().URLForApplicationToOpenURL(&url)?;
            let file = app.lastPathComponent()?.to_string();
            Some(file.strip_suffix(".app").unwrap_or(&file).to_owned())
        }).clone();
    }
    #[cfg(not(target_os = "macos"))]
    None
}

fn open_in(name: Option<&str>) -> String {
    name.map_or_else(|| "Open in default browser".to_owned(), |name| format!("Open in {name}"))
}

/// The label of the "open in the system browser" button and menu row.
fn open_in_label() -> String {
    open_in(default_browser_name().as_deref())
}

/// The rows of the ⋯ menu as `[label, command, args]`; a null command is a section label.
/// The current viewport, cookie and link choices carry a "✓ ".
fn menu_items(id: &str, viewport: Option<[f32; 2]>, keep: settings::KeepCookies, links: settings::OpenLinks, open_in: &str) -> Vec<Value> {
    let at = json!({"pane": id});
    let label = |text: &str| json!([text, null, {"label": true}]);
    let mark = |on: bool, text: &str| format!("{}{text}", if on { "✓ " } else { "" });
    let mut items = vec![
        json!([open_in, "canvas.browser_external", at]),
        json!(["Copy link", "canvas.browser_copy", at]),
        json!(["Save screenshot", "canvas.browser_shot", at]),
        json!(["Open HTML file…", "canvas.browser_open_file", at]),
        label("Viewport"),
    ];
    for (name, text, size) in PRESETS {
        items.push(json!([mark(viewport == size, text), "canvas.browser_viewport", {"pane": id, "size": name}]));
    }
    let built_in = links == settings::OpenLinks::BuiltIn;
    items.push(label("Links"));
    items.push(json!([mark(built_in, "Open links in built-in browser"), "canvas.browser_links", {"open": if built_in { "default" } else { "built_in" }}]));
    items.push(label("Keep cookies"));
    for (value, text, choice) in [("until_quit", "Until quit", settings::KeepCookies::UntilQuit), ("forever", "Forever", settings::KeepCookies::Forever)] {
        items.push(json!([mark(keep == choice, text), "canvas.browser_cookies", {"keep": value}]));
    }
    items.push(json!(["Clear browsing data", "canvas.browser_clear", at]));
    items
}

/// Use the same file URL spelling for saved addresses and WebKit navigation decisions.
#[cfg(target_os = "macos")]
fn file_url(path: &std::path::Path) -> Result<String, String> {
    objc2_foundation::NSURL::from_file_path(path)
        .and_then(|url| url.absoluteString())
        .map(|url| url.to_string())
        .ok_or_else(|| format!("Invalid browser file path: {}", path.display()))
}

/// Classifies an explicit address. Web pages never call this entry point.
pub(super) fn browser_target_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    let file_scheme = input.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"));
    if !file_scheme && !std::path::Path::new(input).is_absolute() { return web_url(input); }
    #[cfg(target_os = "macos")]
    {
        let path = if file_scheme { file_path(input)? } else { input.into() };
        let path = checked_file(&path)?;
        file_url(&path)
    }
    #[cfg(not(target_os = "macos"))]
    Err("browser panes run on macOS only".into())
}

#[cfg(target_os = "macos")]
fn file_path(url: &str) -> Result<std::path::PathBuf, String> {
    use objc2_foundation::{NSString, NSURL};
    NSURL::URLWithString(&NSString::from_str(url))
        .filter(|url| url.isFileURL() && url.host().is_none_or(|host| {
            let host = host.to_string();
            host.is_empty() || host.eq_ignore_ascii_case("localhost")
        }))
        .and_then(|url| url.to_file_path())
        .filter(|path| path.is_absolute())
        .ok_or_else(|| "Enter an absolute path or a local file URL".into())
}

#[cfg(target_os = "macos")]
fn checked_file(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let path = path.canonicalize().map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    if !path.is_file() {
        return Err(format!("Choose a regular file: {}", path.display()));
    }
    Ok(path)
}

/// A view that can read a local file never loads a web page, and vice versa.
#[cfg(any(target_os = "macos", test))]
fn permits_navigation(local: Option<&str>, url: &str) -> bool {
    match local {
        Some(file) => url.split('#').next() == Some(file),
        None => url.get(..7).is_some_and(|s| s.eq_ignore_ascii_case("http://"))
            || url.get(..8).is_some_and(|s| s.eq_ignore_ascii_case("https://")),
    }
}

/// Pick HTML using the native panel so other files are disabled before selection.
#[cfg(target_os = "macos")]
fn pick_html(done: impl FnOnce(Option<std::path::PathBuf>) + 'static) {
    use objc2_app_kit::{NSModalResponse, NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{NSArray, NSString};
    let panel = NSOpenPanel::openPanel(objc2::MainThreadMarker::new().expect("browser command runs on the main thread"));
    panel.setCanChooseDirectories(false);
    panel.setCanChooseFiles(true);
    panel.setAllowsMultipleSelection(false);
    #[allow(deprecated)]
    panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&[NSString::from_str("html"), NSString::from_str("htm")])));
    let done = std::cell::Cell::new(Some(done));
    let handler = block2::RcBlock::new({
        let panel = panel.clone();
        move |response: NSModalResponse| {
            if let Some(done) = done.take() {
                done((response == NSModalResponseOK).then(|| panel.URL()).flatten().and_then(|url| url.to_file_path()));
            }
        }
    });
    panel.beginWithCompletionHandler(&handler);
}

#[cfg(any(target_os = "macos", test))]
fn fixed_viewport(pane: &canvas::Pane) -> Option<[f32; 2]> {
    let canvas::Kind::Browser { viewport, .. } = pane.kind else { return None };
    viewport.filter(|size| size.iter().all(|px| px.is_finite() && *px > 0.))
}

#[cfg(any(target_os = "macos", test))]
/// A fixed viewport scales to fit its body (screen px); a responsive page zooms with the
/// canvas (`scale`), so its CSS viewport keeps the pane's world size and it never reflows.
fn page_zoom(pane: &canvas::Pane, body: [f32; 2], scale: f32) -> f64 {
    match fixed_viewport(pane) {
        Some([w, h]) => (f64::from(body[0]) / f64::from(w)).min(f64::from(body[1]) / f64::from(h)),
        None => f64::from(scale),
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
    panes: HashMap<String, PaneUi>,
    #[cfg(target_os = "macos")]
    next_view: u64,
    #[cfg(target_os = "macos")]
    pub(super) last_masks: super::browser_mask::MaskCache,
    /// Page events into the window; made with the first web view.
    events: Option<async_channel::Sender<(String, u64, Event)>>,
}

#[derive(Default)]
struct Page {
    /// The native view, kept (hidden) while the page rests so a page may play out of sight
    /// and waking it does not reload.
    #[cfg(target_os = "macos")]
    native: Option<Native>,
    /// A folder chosen for this exact file and tab. Never saved to disk.
    #[cfg(target_os = "macos")]
    local_folder: Option<(String, std::path::PathBuf)>,
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
    /// Counts loads, so a load's time limit leaves later loads alone.
    loads: u64,
    /// Identifies callbacks from this view even if a closed tab's id is used again.
    view_id: u64,
}

/// An address field and the subscription that opens what is typed in it.
type UrlInput = (Entity<gpui_base::input::InputState>, Subscription);

/// Where a browser header is drawn: on its pane, or in the pop-out window.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::window) enum Host { Pane, Window }

#[derive(Default)]
struct PaneUi {
    /// The header's address field, made when the pane is first drawn.
    url_input: Option<UrlInput>,
    /// The pop-out window's own address field: a field belongs to one window.
    pop_input: Option<UrlInput>,
    focus_url: bool,
    /// The pop-out window while the pane is popped out. All its pages are built in it.
    popped: Option<AnyWindowHandle>,
    /// The pop-out's open menu (GPUI draws it: the menu mod draws only in the main window).
    // shortcut: the page hides while this menu is open (a native view covers GPUI), mask it like the canvas does if that bothers.
    pop_menu: Option<(Point<Pixels>, Vec<Value>)>,
}

/// A page's native view: the GPUI entity, and the same view for scripts run without a GPUI
/// context. Both or neither.
#[cfg(target_os = "macos")]
struct Native {
    view: Entity<gpui_wry::WebView>,
    raw: gpui_wry::WebViewHandle,
    clip: objc2::rc::Retained<objc2_app_kit::NSView>,
    // The pending local load may run only while this view is still mounted.
    _local_load: Option<std::rc::Rc<()>>,
}

#[cfg(target_os = "macos")]
impl Drop for Native {
    fn drop(&mut self) {
        super::browser_mask::clear_mask(self.raw.raw());
        self.clip.removeFromSuperview();
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
    /// A link asked for a new window: it opens in this pane instead.
    Open(String),
    Blocked(String),
    LoadError(String),
    /// The snapshot taken when the page rested (None: it failed).
    Rested(Option<image::RgbaImage>),
    /// A resting page out of sight has media playing.
    Playing,
}

fn tab_key(pane: &str, tab: &str) -> String {
    format!("{pane}/{tab}")
}

/// Where a tab's last snapshot is kept between runs.
fn shot_file(key: &str) -> std::path::PathBuf {
    crate::mods::app_dir().join("browser").join(format!("{}.png", key.replace('/', "-")))
}

fn link_target<'a>(last: Option<&str>, shown: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let last = last?;
    shown.into_iter().any(|id| id == last).then(|| last.to_owned())
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

/// Cuts text to `max` characters, marking the cut with `…`.
fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) { Some((at, _)) => format!("{}…", &text[..at]), None => text.to_owned() }
}

/// Formats a `quote.send` browser pointer as a URL and its element or source reference.
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

impl AppWindow {
    /// Adds a browser pane at `point` in `space`. An empty URL focuses its New tab field.
    pub(super) fn add_browser_pane(&mut self, url: String, point: canvas::Point, space: Option<String>, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        let empty = url.is_empty();
        let kind = canvas::Kind::Browser {
            home: url.clone(), tabs: vec![canvas::Tab { id: "t1".into(), url, title: String::new() }], active: 0, viewport: None,
        };
        doc.panes.push(canvas::Pane { id: id.clone(), kind, rect: [point[0], point[1], SIZE[0], SIZE[1]], z, space, color: None, pinned: None });
        self.canvas_state.last_browser = Some(id.clone());
        if empty { self.canvas_state.browsers.panes.entry(id.clone()).or_default().focus_url = true; }
        self.canvas_changed(cx);
        cx.notify();
        id
    }

    /// Tears down closed tabs and loads saved snapshots for tabs new to this run.
    pub(super) fn sync_browsers(&mut self, cx: &mut Context<Self>) {
        let mut ids = HashSet::new();
        let mut keys = HashSet::new();
        for pane in &self.canvas_state.doc.panes {
            if let canvas::Kind::Browser { tabs, .. } = &pane.kind {
                ids.insert(pane.id.clone());
                keys.extend(tabs.iter().map(|tab| tab_key(&pane.id, &tab.id)));
            }
        }
        let state = &mut self.canvas_state;
        if state.browser_tab.as_ref().is_some_and(|id| !ids.contains(id)) { state.browser_tab = None; }
        if state.last_browser.as_ref().is_some_and(|id| !ids.contains(id)) { state.last_browser = None; }
        let browsers = &mut state.browsers;
        browsers.panes.retain(|id, ui| {
            let keep = ids.contains(id);
            if !keep && let Some(window) = ui.popped.take() {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            }
            keep
        });
        #[cfg(target_os = "macos")]
        browsers.last_masks.borrow_mut().retain(|key, _| keys.contains(key));
        browsers.pages.retain(|key, page| {
            let keep = keys.contains(key);
            if !keep {
                let _ = std::fs::remove_file(shot_file(key));
                for ask in page.waiting.drain(..) {
                    let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
                    let _ = tx.send(json!({"error": "the browser tab closed"}));
                }
                #[cfg(target_os = "macos")]
                if let Some(native) = page.native.take() { destroy(native); }
            }
            keep
        });
        for key in keys {
            browsers.pages.entry(key.clone()).or_insert_with(|| Page {
                // shortcut: decodes snapshots on the main thread, move to a background task if many tabs make loading slow.
                shot: image::open(shot_file(&key)).ok().map(|i| render_image(i.to_rgba8())),
                ..Default::default()
            });
        }
    }

    /// Resolves pane-level actions to the current tab.
    pub(super) fn page_key(&self, id: &str) -> Option<String> {
        let pane = self.canvas_state.doc.panes.iter().find(|pane| pane.id == id)?;
        Some(tab_key(id, &pane.active_tab()?.id))
    }

    fn active_page(&self, id: &str) -> Option<&Page> {
        self.canvas_state.browsers.pages.get(&self.page_key(id)?)
    }

    fn tab_by_key(&self, key: &str) -> Option<&canvas::Tab> {
        let (id, tab) = key.split_once('/')?;
        let pane = self.canvas_state.doc.panes.iter().find(|pane| pane.id == id)?;
        let canvas::Kind::Browser { tabs, .. } = &pane.kind else { return None };
        tabs.iter().find(|t| t.id == tab)
    }

    pub(super) fn browser_popped(&self, id: &str) -> bool {
        self.canvas_state.browsers.panes.get(id).is_some_and(|ui| ui.popped.is_some())
    }

    /// The keys of every tab's page in a browser pane.
    fn pane_page_keys(&self, id: &str) -> Vec<String> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id);
        let tabs = match pane.map(|p| &p.kind) { Some(canvas::Kind::Browser { tabs, .. }) => tabs.as_slice(), _ => &[] };
        tabs.iter().map(|t| tab_key(id, &t.id)).collect()
    }

    /// Draws New tab, the live web view, or the resting snapshot. Click a resting page to wake it.
    pub(super) fn browser_body(&self, pane: &canvas::Pane, camera: canvas::Camera, cx: &mut Context<Self>) -> AnyElement {
        let id = pane.id.clone();
        let page = self.active_page(&id);
        if self.browser_popped(&id) {
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
        if self.browser_url(&id) == Ok("") {
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
                let (this, key) = (cx.entity(), self.page_key(&id).unwrap());
                let zoom_changed = at.0.zoom != next.0.zoom || (preset && at.1[2..] != next.1[2..]);
                let zoom = zoom_changed.then(|| {
                    let [_, _, width, height] = pane_body(pane, camera);
                    page_zoom(pane, [width, height], next.0.zoom)
                });
                cx.defer(move |cx| this.update(cx, |this, _| {
                    let Some(page) = this.canvas_state.browsers.pages.get_mut(&key) else { return };
                    if page.live == Some(at) && let Some(native) = &page.native {
                        page.live = Some(next);
                        if let Some(zoom) = zoom.filter(|z| *z > 0.) { let _ = native.raw.raw().zoom(zoom); }
                    }
                }));
            }
            let frame = self.measured_webview(self.page_key(&id).unwrap(), view);
            let body = if preset {
                let [_, _, width, height] = viewport_rect(pane, camera);
                div().size_full().bg(c("canvas")).flex().items_center().justify_center()
                    .child(div().flex_none().w(px(width)).h(px(height)).child(frame))
            } else {
                div().size_full().bg(gpui::white()).child(frame)
            };
            let body = body.relative().child(self.browser_measure(format!("web-body:{id}")));
            return self.annotate_body(pane, body.into_any_element(), super::annotate::card_body(pane, camera.zoom), cx);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = camera;
        let body = match page.and_then(|p| p.shot.clone()) {
            Some(shot) => div().size_full().bg(gpui::white()).child(img(ImageSource::Render(shot)).size_full().object_fit(gpui::ObjectFit::Cover)),
            None => div().size_full().flex().items_center().justify_center().p(px(16.)).text_color(c("muted-foreground"))
                .child(if page.is_some_and(|p| p.loading) { "Loading…".to_owned() } else { "Click to load the page".to_owned() }),
        };
        let body = body.id(SharedString::from(format!("browser-{id}"))).cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.wake_browser(&id, window, cx);
            })).into_any_element();
        self.annotate_body(pane, body, super::annotate::card_body(pane, camera.zoom), cx)
    }

    /// The active tab's title, falling back to its address or New tab.
    pub(super) fn browser_title(&self, pane: &canvas::Pane) -> (String, String) {
        let tab = pane.active_tab().unwrap();
        if tab.url.is_empty() { return ("New tab".into(), String::new()); }
        let title = self.active_page(&pane.id).map(|page| page.title.as_str())
            .filter(|title| !title.is_empty()).unwrap_or(&tab.title);
        if title.is_empty() { (tab.url.clone(), String::new()) } else { (title.to_owned(), tab.url.clone()) }
    }

    fn browser_button(&self, id: &str, slot: &'static str, icon: &'static str, label: &str, command: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = id.to_owned();
        panel_button(slot, icon, c("muted-foreground"), c("hover")).aria_label(label.to_owned()).tooltip(strip_tooltip(label.to_owned(), Some(command)))
            .on_click(cx.listener(move |this, _, window, cx| this.run_key(command, json!({"pane": id}), window, cx)))
    }

    /// The browser's page tabs. In the Files strip they share its horizontal scroll area.
    pub(super) fn browser_tabs(&self, id: &str, host: Host, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id)?;
        let canvas::Kind::Browser { tabs, active, .. } = &pane.kind else { return None };
        let docked = host == Host::Pane && self.canvas_state.doc.docked(id);
        let tint = docked.then(|| self.docked_tab(id, cx).2).flatten();
        // The background behind the tabs: the Files strip, else the pane or pop-out header.
        let strip_color = if docked { self.person.canvas_color.as_deref().unwrap_or("canvas") } else { "surface" };
        let mut strip = div().id(SharedString::from(format!("{id}-tabs")))
            .flex().items_center().h_full()
            .when(docked, |d| d.flex_none())
            .when(!docked, |d| d.role(gpui::Role::TabList).min_w_0().flex_shrink_1().overflow_hidden());
        for (i, tab) in tabs.iter().enumerate() {
            let selected = i == *active;
            let title = if !tab.title.is_empty() { tab.title.clone() } else if !tab.url.is_empty() { tab.url.clone() } else { "New tab".to_owned() };
            let args = json!({"pane": id, "tab": tab.id});
            let (select, close, middle) = (args.clone(), args.clone(), args);
            let (menu_pane, menu_tab) = (id.to_owned(), tab.id.clone());
            let group = SharedString::from(format!("{id}-tab-{}", tab.id));
            // shortcut: a globe for every tab, no favicons: loading one would be a request outside WebKit.
            // The same tab as the Files strip, so pages and files read as one set of tabs.
            strip = strip.child(strip_tab(group.clone(), selected, c("hover"), c(strip_color)).group(group.clone())
                .flex().items_center().h_full().w(px(180.)).min_w(px(96.)).flex_shrink_1().pr(px(4.)).text_size(px(12.))
                .text_color(c(if selected { "foreground" } else { "muted-foreground" }))
                .hover(|d| d.text_color(c("foreground")))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.run_key("canvas.browser_tab_close", middle.clone(), window, cx);
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.browser_tab_menu(&menu_pane, &menu_tab, host, window, cx);
                }))
                .child(div().id("select").role(gpui::Role::Tab).aria_selected(selected).aria_label(title.clone())
                    .flex().items_center().gap(px(6.)).flex_1().min_w_0().h_full().pl(px(8.)).cursor_pointer()
                    .tooltip(strip_tooltip(title.clone(), None))
                    .on_click(cx.listener(move |this, _, window, cx| this.run_key("canvas.browser_tab_select", select.clone(), window, cx)))
                    .child(svg().path("icons/globe.svg").flex_none().size(px(14.)).text_color(c("muted-foreground")))
                    .child(div().flex_1().min_w_0().truncate().child(title))
                    .when_some(tint.as_deref().filter(|_| selected), |d, tint| d.child(div().flex_none().size(px(6.)).rounded_full().bg(c(tint)))))
                .child(div().id("close").role(gpui::Role::Button).aria_label("Close tab").flex_none().size(px(16.))
                    .flex().items_center().justify_center().rounded(px(ui::RADIUS_SM)).cursor_pointer().hover(|d| d.bg(c("hover")))
                    .when(!selected, |d| d.opacity(0.).group_hover(group.clone(), |d| d.opacity(1.)))
                    .tooltip(strip_tooltip("Close tab".into(), Some("canvas.browser_tab_close")))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| this.run_key("canvas.browser_tab_close", close.clone(), window, cx)))
                    .child(svg().path("icons/x.svg").size(px(12.)).text_color(c("muted-foreground")))));
        }
        Some(strip.into_any_element())
    }

    pub(super) fn browser_tab_add(&self, id: &str, cx: &mut Context<Self>) -> Stateful<Div> {
        self.browser_button(id, "browser-tab-new", "mods/workspace-chrome/plus.svg", "New tab", "canvas.browser_tab_new", cx)
    }

    /// Pane controls stay visible when the Files tabs scroll.
    pub(super) fn browser_tab_actions(&self, id: &str, host: Host, cx: &mut Context<Self>) -> Div {
        let more = id.to_owned();
        let connect = id.to_owned();
        let pop_label = if self.browser_popped(id) { "Bring back" } else { "Pop out" };
        let docked = self.canvas_state.doc.docked(id);
        div().flex().flex_none().items_center().gap(px(4.))
            .child(self.pane_color_swatch(id, cx))
            .when(host == Host::Pane, |d| d.child(panel_button("browser-connect", "icons/link.svg", c("muted-foreground"), c("hover"))
                .aria_label("Connect").tooltip(strip_tooltip("Connect".into(), Some("canvas.connect")))
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| this.wire_start(&connect, e.position(), window, cx)))))
            .child(panel_button("browser-more", "icons/ellipsis.svg", c("muted-foreground"), c("hover")).aria_label("More").tooltip(strip_tooltip("More".into(), None))
                .on_click(cx.listener(move |this, _, window, cx| this.browser_menu(&more, host, window, cx))))
            // In the pop-out, Close already brings the page back.
            .when(host == Host::Pane, |d| d.child(self.browser_button(id, "browser-pop", "icons/picture-in-picture-2.svg", pop_label, "canvas.browser_pop", cx)))
            .when(host == Host::Pane && !docked, |d| d.child(self.browser_button(id, "browser-expand", "icons/maximize-2.svg", "Expand", "canvas.browser_expand", cx)))
            .when(host == Host::Pane && !docked, |d| d.child(self.browser_button(id, "browser-close", "icons/x.svg", "Close", "canvas.close", cx)))
            .when(host == Host::Window, |d| d.child(self.browser_button(id, "browser-close", "icons/x.svg", "Bring back", "canvas.browser_pop", cx)))
    }

    /// Navigation, the address field and Annotate, below the browser tabs.
    pub(super) fn browser_navigation(&self, id: &str, host: Host, cx: &mut Context<Self>) -> AnyElement {
        let ui = self.canvas_state.browsers.panes.get(id);
        let field = ui.and_then(|ui| if host == Host::Pane { ui.url_input.as_ref() } else { ui.pop_input.as_ref() }).map(|(input, _)| input.clone());
        let reset = id.to_owned();
        let annotate = id.to_owned();
        let picker = self.annotation_picker(id);
        div().id("browser-navigation").flex().items_center().h(px(32.)).min_w_0().overflow_x_scroll().px(px(8.)).gap(px(4.))
            .child(self.browser_button(id, "browser-back", "icons/arrow-left.svg", "Back", "canvas.browser_back", cx))
            .child(self.browser_button(id, "browser-forward", "icons/arrow-right.svg", "Forward", "canvas.browser_forward", cx))
            .child(self.browser_button(id, "browser-reload", "icons/rotate-cw.svg", "Reload", "canvas.browser_reload", cx))
            .child(div().flex().flex_1().min_w_0().h(px(24.)).px(px(8.)).items_center().overflow_hidden()
                .rounded(px(ui::RADIUS_SM)).bg(c("canvas")).border_1().border_color(c("border"))
                // Typing in the field is not a pane drag.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                    if e.keystroke.key == "escape" { this.reset_url(&reset, host, window, cx); cx.stop_propagation(); }
                }))
                .children(field.map(|input| gpui_base::Input::new(&input))))
            .children(picker.map(|name| div().id("browser-annotate-agent").role(gpui::Role::Button).aria_label("Send annotation to")
                .flex().flex_none().items_center().gap(px(4.)).h(px(24.)).px(px(8.)).rounded(px(ui::RADIUS_SM)).border_1().border_color(c("border"))
                .text_size(px(12.)).text_color(c("foreground")).cursor_pointer().hover(|d| d.bg(c("hover")))
                .tooltip(strip_tooltip("Send annotation to".into(), Some("canvas.annotate_agent")))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener({ let pane = id.to_owned(); move |this, _, window, cx| this.run_key("canvas.annotate_agent", json!({"pane": pane}), window, cx) }))
                .child(div().max_w(px(128.)).truncate().child(name))))
            .child(div().id("browser-annotate").role(gpui::Role::Button).aria_label("Annotate")
                .flex().flex_none().items_center().gap(px(4.)).h(px(24.)).px(px(8.)).rounded(px(ui::RADIUS_SM)).border_1().border_color(c("border"))
                .text_size(px(12.)).text_color(c("muted-foreground")).cursor_pointer().hover(|d| d.bg(c("hover")).text_color(c("foreground")))
                .tooltip(strip_tooltip("Annotate".into(), Some("canvas.browser_annotate")))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| this.run_key("canvas.browser_annotate", json!({"pane": annotate}), window, cx)))
                .child("Annotate"))
            .into_any_element()
    }

    /// The two-row canvas or pop-out header. Buttons stop mouse-down so empty space
    /// in the top row can still drag the pane or window.
    pub(super) fn browser_chrome(&self, id: &str, host: Host, cx: &mut Context<Self>) -> AnyElement {
        let Some(tabs) = self.browser_tabs(id, host, cx) else { return div().into_any_element() };
        let row = div().flex().items_center().h(px(30.)).pr(px(8.)).pl(px(super::TAB_INSET)).gap(px(4.))
            .when(host == Host::Window, |d| d.pl(px(super::bar_inset(true, false))).on_mouse_down(MouseButton::Left, |e: &MouseDownEvent, window, _| {
                if e.click_count == 2 { window.titlebar_double_click(); } else { window.start_window_move(); }
            }))
            .child(tabs)
            .child(self.browser_tab_add(id, cx))
            .child(div().flex_1())
            .child(self.browser_tab_actions(id, host, cx));
        div().flex().flex_col().w_full().h(px(canvas::BROWSER_HEADER)).min_w_0().text_color(c("foreground"))
            .when(host == Host::Window, |d| d.bg(c("surface")).border_b_1().border_color(c("border")))
            .child(row).child(self.browser_navigation(id, host, cx)).into_any_element()
    }

    pub(in crate::window) fn selected_browser(&self) -> Option<String> {
        let [id] = self.canvas_state.selected.as_slice() else { return None };
        self.browser_url(id).ok().map(|_| id.clone())
    }

    pub(in crate::window) fn focus_browser_url(&mut self, id: &str, cx: &mut Context<Self>) -> Result<(), String> {
        self.browser_url(id)?;
        self.canvas_state.browsers.panes.entry(id.to_owned()).or_default().focus_url = true;
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
    /// `localhost:3000` a URL and handles explicit local files). Runs from `render`, which has a window.
    pub(in crate::window) fn sync_browser_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let doc = &self.canvas_state.doc;
        let urls: Vec<_> = doc.panes.iter().filter(|p| p.url().is_some()).map(|p| (p.id.clone(), self.canvas_state.shows_pane(p))).collect();
        // The page events task belongs to this window: a pop-out window must not be the one to start it.
        #[cfg(target_os = "macos")]
        if !urls.is_empty() { self.browser_events(window, cx); }
        for (id, shown) in urls {
            self.sync_url_input(&id, Host::Pane, window, cx);
            let ui = self.canvas_state.browsers.panes.entry(id).or_default();
            // A popped pane's new tab takes the keyboard in its window.
            if shown && ui.popped.is_none() && std::mem::take(&mut ui.focus_url) && let Some((input, _)) = &ui.url_input {
                input.update(cx, |i, cx| i.focus(window, cx));
            }
        }
    }

    /// Makes the address field of `host` (one per window) and keeps its text at the pane's URL
    /// unless the person is typing in it.
    fn sync_url_input(&mut self, id: &str, host: Host, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(url) = self.browser_url(id).map(str::to_owned) else { return };
        let ui = self.canvas_state.browsers.panes.entry(id.to_owned()).or_default();
        let slot = if host == Host::Pane { &mut ui.url_input } else { &mut ui.pop_input };
        match slot.as_ref() {
            Some((input, _)) => {
                let stale = input.read(cx).value().to_string() != url && !input.read(cx).focus_handle(cx).is_focused(window);
                if stale { input.update(cx, |input, cx| input.set_value(url, window, cx)); }
            }
            None => {
                let input = cx.new(|cx| {
                    let mut input = gpui_base::input::InputState::new(window, cx).placeholder("Enter an address or file path");
                    input.set_value(url, window, cx);
                    input
                });
                let pane = id.to_owned();
                let sub = cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                    if !matches!(event, gpui_base::input::InputEvent::PressEnter { .. }) { return; }
                    let target = input.read(cx).value().to_string();
                    // The pop-out has no canvas focus to return to.
                    if host == Host::Pane { this.focus.focus(window, cx); } else { window.blur(cx); }
                    this.run_key("canvas.pane_open", json!({"pane": pane, "target": target}), window, cx);
                });
                *slot = Some((input, sub));
            }
        }
    }

    /// Escape in the address field: the pane's URL back in it, and the keyboard back to the canvas.
    fn reset_url(&mut self, id: &str, host: Host, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.canvas_state.doc.panes.iter().find(|p| p.id == id).and_then(|p| p.url()).unwrap_or_default().to_owned();
        let ui = self.canvas_state.browsers.panes.get(id);
        if let Some(input) = ui.and_then(|ui| if host == Host::Pane { ui.url_input.as_ref() } else { ui.pop_input.as_ref() }).map(|(input, _)| input.clone()) {
            input.update(cx, |input, cx| input.set_value(url, window, cx));
        }
        if host == Host::Pane { self.focus.focus(window, cx); } else { window.blur(cx); }
    }

    /// The header's "⋯" menu, at the pointer.
    fn browser_menu(&mut self, id: &str, host: Host, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return };
        let canvas::Kind::Browser { viewport, .. } = &pane.kind else { return };
        let mut items = menu_items(id, *viewport, self.person.browser_keep_cookies, self.person.browser_open_links, &open_in_label());
        let at = json!({"pane": id});
        if pane.url().is_some_and(|url| url.starts_with("file:")) {
            items.insert(4, json!(["Allow folder assets…", "canvas.browser_allow_folder", at]));
        }
        items.insert(0, json!(["Home", "canvas.browser_home", at]));
        if host == Host::Pane {
            let docked = self.canvas_state.doc.docked(id);
            items.insert(1, json!([if docked { "Move to canvas" } else { "Open in panel" }, "canvas.pane_dock", {"pane": id, "docked": !docked}]));
            if docked {
                items.insert(2, json!(["Close browser pane", "canvas.close", at]));
            } else {
                let pinned = pane.pinned.is_some();
                items.insert(2, json!([if pinned { "Unpin" } else { "Pin to screen" }, "canvas.pane_pin", {"pane": id, "pinned": !pinned}]));
            }
        }
        // The pop-out shows the page at window size: viewport presets apply on the canvas only.
        if host == Host::Window { items.retain(|i| i[1] != "canvas.browser_viewport" && i[0] != "Viewport"); }
        self.show_browser_menu(id, host, items, window, cx);
    }

    /// A tab's right-click menu.
    fn browser_tab_menu(&mut self, id: &str, tab: &str, host: Host, window: &mut Window, cx: &mut Context<Self>) {
        let at = json!({"pane": id, "tab": tab});
        let items = vec![json!([open_in_label(), "canvas.browser_external", at]), json!(["Close tab", "canvas.browser_tab_close", at])];
        self.show_browser_menu(id, host, items, window, cx);
    }

    /// The pane popped out into this window, if it is one.
    pub(super) fn popped_pane(&self, window: &Window) -> Option<String> {
        self.canvas_state.browsers.panes.iter().find(|(_, ui)| ui.popped == Some(window.window_handle())).map(|(id, _)| id.clone())
    }

    /// Which window should render a menu opened by a pane command.
    #[cfg(target_os = "macos")]
    pub(super) fn browser_menu_host(&self, id: &str, window: &Window) -> Host {
        if self.canvas_state.browsers.panes.get(id).and_then(|pane| pane.popped) == Some(window.window_handle()) {
            Host::Window
        } else {
            Host::Pane
        }
    }

    /// Opens a menu of `[label, command, args]` rows at the pointer: the menu mod's popup in the
    /// main window, or in the pop-out the menu `pop_menu` draws.
    pub(super) fn show_browser_menu(&mut self, id: &str, host: Host, items: Vec<Value>, window: &mut Window, cx: &mut Context<Self>) {
        if host == Host::Window {
            self.canvas_state.browsers.panes.entry(id.to_owned()).or_default().pop_menu = Some((window.mouse_position(), items));
            return cx.notify();
        }
        self.open_menu("primary", json!({"items": items}), Some(window.mouse_position()), window, cx);
    }

    #[cfg(target_os = "macos")]
    fn dismiss_pop_menu(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(ui) = self.canvas_state.browsers.panes.get_mut(id) { ui.pop_menu = None; }
        cx.stop_propagation();
        cx.notify();
    }

    #[cfg(target_os = "macos")]
    /// The pop-out's open menu, over the window at `view` size. Choosing a row, or a click
    /// anywhere else, closes it.
    fn pop_menu(&self, id: &str, view: Size<Pixels>, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (at, items) = self.canvas_state.browsers.panes.get(id)?.pop_menu.clone()?;
        let rows = items.into_iter().enumerate().map(|(i, item)| {
            let label = item[0].as_str().unwrap_or_default().to_owned();
            let Some(command) = item[1].as_str().map(str::to_owned) else {
                return div().h(px(25.)).px(px(8.)).flex().items_center().text_size(px(11.5)).font_weight(FontWeight::SEMIBOLD)
                    .text_color(c("muted-foreground")).child(label).into_any_element();
            };
            let (args, pane) = (item[2].clone(), id.to_owned());
            div().id(SharedString::from(format!("pop-menu-{i}"))).role(gpui::Role::Button).aria_label(label.clone())
                .h(px(32.)).px(px(8.)).flex().items_center().rounded(px(ui::RADIUS_SM)).cursor_pointer().hover(|d| d.bg(c("hover")))
                .child(label)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(ui) = this.canvas_state.browsers.panes.get_mut(&pane) { ui.pop_menu = None; }
                    this.run_key(&command, args.clone(), window, cx);
                    cx.notify();
                })).into_any_element()
        }).collect::<Vec<_>>();
        let (width, height) = (220., rows.len() as f32 * 32. + 10.);
        let (x, y) = (f32::from(at.x).min(f32::from(view.width) - width - 4.).max(4.), f32::from(at.y).min(f32::from(view.height) - height - 4.).max(4.));
        let (left, right) = (id.to_owned(), id.to_owned());
        Some(div().absolute().inset_0()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _, cx| this.dismiss_pop_menu(&left, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, _: &MouseDownEvent, _, cx| this.dismiss_pop_menu(&right, cx)))
            .child(div().absolute().left(px(x)).top(px(y)).w(px(width)).p(px(5.)).flex().flex_col()
                .bg(c("popover")).border_1().border_color(c("border")).rounded(px(ui::RADIUS)).shadow_lg().text_size(px(12.8))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .children(rows))
            .into_any_element())
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

    fn expanded_webview(&self, id: &str, page: &Page) -> Option<Div> {
        #[cfg(target_os = "macos")]
        { Some(self.measured_webview(self.page_key(id)?, &page.native.as_ref()?.view)) }
        #[cfg(not(target_os = "macos"))]
        { let _ = (id, page); None }
    }

    /// Shows the expanded live page over the measured canvas area.
    pub(super) fn expanded_browser(&self, bg: Hsla) -> Option<AnyElement> {
        let id = self.canvas_state.browser_tab.as_ref()?;
        let page = self.active_page(id)?;
        let bounds = self.bounds.clone();
        Some(div().id("canopy-browser-tab").relative().flex_1().h_full().overflow_hidden().bg(bg)
            .child(canvas(move |b, _, _| {
                let mut bounds = bounds.borrow_mut();
                bounds.insert("canopy-canvas".into(), b);
                #[cfg(target_os = "macos")]
                bounds.insert("browser-canvas".into(), b);
            }, |_, _, _, _| {}).absolute().size_full())
            .children(self.expanded_webview(id, page)).into_any_element())
    }

    /// Answers page events from the web views.
    fn browser_event(&mut self, key: String, view: u64, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, tab)) = key.split_once('/') else { return };
        let Some(page) = self.canvas_state.browsers.pages.get_mut(&key).filter(|p| p.view_id == view && p.has_view()) else { return };
        match event {
            // A navigation inside the page (a link) committed.
            Event::Started => return self.begin_load(&key, cx),
            Event::Title(title) => {
                // Pages can set any title, as often as they like: keep a short one line.
                let title: String = title.chars().filter(|c| !c.is_control()).take(300).collect();
                page.title = title.clone();
                if let Some(saved) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id)
                    && let canvas::Kind::Browser { tabs, .. } = &mut saved.kind
                    && let Some(saved) = tabs.iter_mut().find(|t| t.id == tab)
                    && saved.title != title
                {
                    saved.title = title;
                    self.canvas_changed(cx);
                }
            }
            Event::Loaded => {
                // Scripts often render after the load event (React does): let the page settle
                // before answering. The timer runs only while a load is pending.
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(SETTLE).await;
                    let _ = this.update(cx, |this, cx| this.page_ready(&key, view, cx));
                }).detach();
                return;
            }
            Event::Open(url) => {
                let local = self.tab_by_key(&key).is_some_and(|t| t.url.starts_with("file:"));
                match web_url(&url) {
                    Ok(url) if !local => self.navigate_browser(&key, &url, cx),
                    _ => self.command_error("Page not opened", "Open this address explicitly in the browser's address field", window, cx),
                }
            }
            Event::Blocked(url) => {
                self.toast(Toast {
                    title: "Page not opened".into(),
                    body: Some(format!("This page cannot open {}", cut(&url, 180))),
                    level: Level::Error,
                    action: Some(("pane.focus".into(), json!({"pane": id}))),
                }, window, cx);
            }
            Event::LoadError(error) => {
                let waiting = std::mem::take(&mut page.waiting);
                page.loading = false;
                for ask in waiting {
                    let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
                    let _ = tx.send(json!({"error":error}));
                }
                self.toast(Toast { title: "Page not opened".into(), body: Some(error), level: Level::Error,
                    action: Some(("pane.focus".into(), json!({"pane": id}))) }, window, cx);
            }
            Event::Rested(shot) => {
                if let Some(rgba) = shot {
                    let file = shot_file(&key);
                    let saved = rgba.clone();
                    cx.background_executor().spawn(async move {
                        if let Some(dir) = file.parent() { let _ = std::fs::create_dir_all(dir); }
                        let _ = crate::control::save_png(&saved, &file);
                    }).detach();
                    // The old still's texture is freed after this frame, as GPUI wants.
                    if let Some(old) = page.shot.replace(render_image(rgba)) { cx.defer(move |cx| cx.drop_image(old, None)); }
                }
                #[cfg(target_os = "macos")]
                if page.live.is_none() && let Some(native) = &page.native {
                    native.view.update(cx, |v, _| { if v.visible() { v.hide(); } });
                    super::browser_mask::clear_mask(native.raw.raw());
                }
                cx.notify();
            }
            Event::Playing => {
                if page.live.is_some() || std::mem::replace(&mut page.told, true) { return; }
                let title = Some(page.title.clone()).filter(|t| !t.is_empty())
                    .or_else(|| self.tab_by_key(&key).map(|tab| tab.url.as_str()).map(|u| u.split_once("://").map_or(u, |(_, r)| r).split(['/', '?', '#']).next().unwrap_or(u).to_owned()))
                    .unwrap_or_else(|| id.to_owned());
                self.toast(Toast { title: format!("{title} is still playing"), body: None, level: Level::Info, action: Some(("pane.focus".into(), json!({"pane": id}))) }, window, cx);
            }
        }
        cx.notify();
    }

    /// Marks the page loading. wry reports no failed load (a refused connection sends no
    /// event at all), so a page still loading after LOAD_LIMIT is answered anyway; the timer
    /// runs once per load.
    fn begin_load(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(key) else { return };
        page.loading = true;
        page.loads += 1;
        let (load, view, key) = (page.loads, page.view_id, key.to_owned());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LOAD_LIMIT).await;
            let _ = this.update(cx, |this, cx| {
                if this.canvas_state.browsers.pages.get(&key).is_some_and(|p| p.loading && p.loads == load && p.view_id == view) {
                    this.page_ready(&key, view, cx);
                }
            });
        }).detach();
    }

    /// Answers the tab's waiting replies and refreshes its snapshot if it is resting.
    fn page_ready(&mut self, key: &str, view: u64, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(key).filter(|p| p.view_id == view && p.has_view()) else { return };
        page.loading = false;
        let waiting = std::mem::take(&mut page.waiting);
        let resting = page.live.is_none();
        self.note_browser_url(key, cx);
        // The host may have been measured since the view was made: read and snapshot at its size.
        if resting { self.fit_resting(key, cx); }
        for ask in waiting { self.answer_browser(key, ask); }
        if resting { self.rest_page(key, cx); }
        cx.notify();
    }

    /// Saves the URL the tab navigated to, including navigation while resting.
    fn note_browser_url(&mut self, key: &str, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            let Some((id, tab)) = key.split_once('/') else { return };
            let Some(url) = self.raw_page(key).and_then(|raw| page_url(raw.raw())) else { return };
            if web_url(&url).is_err() || self.tab_by_key(key).is_none_or(|t| t.url == url) { return; }
            let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
            if pane.active_tab().is_some_and(|t| t.id == tab) {
                pane.set_url(url);
            } else if let canvas::Kind::Browser { tabs, .. } = &mut pane.kind
                && let Some(saved) = tabs.iter_mut().find(|t| t.id == tab)
            {
                saved.url = url;
            }
            self.canvas_changed(cx);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (key, cx);
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

    pub(super) fn select_browser_pane(&mut self, id: &str) {
        self.canvas_state.last_browser = Some(id.to_owned());
        self.canvas_state.selected = vec![id.to_owned()];
    }

    fn select_tab(&mut self, id: &str, tab: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no browser pane {id}"))?;
        let canvas::Kind::Browser { tabs, active, .. } = &pane.kind else { return Err(format!("no browser pane {id}")); };
        let next = tabs.iter().position(|t| t.id == tab).ok_or_else(|| format!("no tab {tab} in browser pane {id}"))?;
        let shown = self.canvas_state.shows_pane(pane);
        if next == *active {
            if shown { self.select_browser_pane(id); }
            if self.browser_url(id)? == "" { self.focus_browser_url(id, cx)?; }
            cx.notify();
            return Ok(());
        }
        let was_live = self.active_page(id).is_some_and(|page| page.live.is_some());
        if self.canvas_state.browser_tab.as_deref() == Some(id) {
            self.collapse_browser(window, cx);
        } else if was_live {
            self.rest_browser(id, cx);
        }
        #[cfg(target_os = "macos")]
        if was_live && let Some(native) = self.active_page(id).and_then(|page| page.native.as_ref()) {
            native.view.update(cx, |v, _| v.hide());
        }
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { tabs, active, .. } = &mut pane.kind else { unreachable!() };
        *active = next;
        let empty = tabs[next].url.is_empty();
        let url = tabs[next].url.clone();
        let ui = self.canvas_state.browsers.panes.entry(id.to_owned()).or_default();
        ui.focus_url = empty;
        // The field of the window the click came from: an input belongs to one window.
        let here = ui.popped.is_some_and(|handle| handle == window.window_handle());
        if let Some((input, _)) = if here { &ui.pop_input } else { &ui.url_input } {
            input.update(cx, |input, cx| input.set_value(url, window, cx));
        }
        if shown { self.select_browser_pane(id); }
        self.canvas_changed(cx);
        cx.notify();
        if !empty && self.browser_popped(id) {
            self.ensure_view(id, window, cx)?;
        } else if was_live && shown && !empty {
            self.wake_browser_result(id, window, cx)?;
        }
        Ok(())
    }

    fn new_browser_tab(&mut self, id: &str, url: String, live: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<String, String> {
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).ok_or_else(|| format!("no browser pane {id}"))?;
        let canvas::Kind::Browser { tabs, .. } = &mut pane.kind else { return Err(format!("no browser pane {id}")); };
        let tab = canvas::next_tab_id(tabs);
        let has_url = !url.is_empty();
        tabs.push(canvas::Tab { id: tab.clone(), url, title: String::new() });
        self.canvas_state.last_browser = Some(id.to_owned());
        self.select_tab(id, &tab, window, cx)?;
        if has_url {
            if live && !self.browser_popped(id) && !self.active_page(id).is_some_and(|page| page.live.is_some()) {
                self.wake_browser_result(id, window, cx)?;
            } else if let Err(e) = self.ensure_view(id, window, cx) {
                self.command_error("Open tab", &e, window, cx);
            }
        }
        Ok(tab)
    }

    fn close_browser_tab(&mut self, id: &str, tab: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { tabs, active, .. } = &mut pane.kind else { unreachable!() };
        let index = match tab {
            Some(tab) => tabs.iter().position(|t| t.id == tab).ok_or_else(|| format!("no tab {tab} in browser pane {id}"))?,
            None => *active,
        };
        if tabs.len() == 1 {
            tabs.push(canvas::Tab { id: canvas::next_tab_id(tabs), url: String::new(), title: String::new() });
        }
        let selected = if index == *active {
            let next = if index == 0 { 1 } else { index - 1 };
            let tab = tabs[next].id.clone();
            self.select_tab(id, &tab, window, cx)
        } else {
            Ok(())
        };
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { tabs, active, .. } = &mut pane.kind else { unreachable!() };
        tabs.remove(index);
        if index < *active { *active -= 1; }
        self.sync_browsers(cx);
        self.canvas_changed(cx);
        cx.notify();
        selected
    }

    /// Grants a picked directory to the current local file for the lifetime of this tab.
    #[cfg(target_os = "macos")]
    fn allow_browser_folder(&mut self, id: &str, url: &str, path: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let file = checked_file(&file_path(url)?)?;
        let folder = path.canonicalize().map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
        if !folder.is_dir() || !file.starts_with(&folder) {
            return Err("Choose a folder containing the file and its assets".into());
        }
        let key = self.page_key(id).ok_or("the browser tab closed")?;
        self.canvas_state.browsers.pages.entry(key).or_default().local_folder = Some((url.to_owned(), folder));
        self.load_browser(id, url, false, window, cx);
        Ok(())
    }

    /// Opens an HTTP(S) link clicked in a terminal in the last browser here or in a pane beside it.
    pub(in crate::window) fn open_link(&mut self, terminal: &str, url: String, window: &mut Window, cx: &mut Context<Self>) {
        let url = match web_url(&url) {
            Ok(url) => url,
            Err(e) => return self.command_error("Open link", &e, window, cx),
        };
        let doc = &self.canvas_state.doc;
        let shown = doc.panes.iter().filter(|pane| pane.url().is_some() && self.canvas_state.shows_pane(pane)).map(|p| p.id.as_str());
        let last = link_target(self.canvas_state.last_browser.as_deref(), shown);
        let id = last.unwrap_or_else(|| {
            let doc = &self.canvas_state.doc;
            let (at, space) = match self.wire_cell(doc, terminal).zip(doc.panes.iter().find(|p| p.id == terminal)) {
                Some(([x, y, w, _], pane)) => ([x + w + settings::get().canvas_gap, y], pane.space.clone()),
                None => {
                    let [x, y] = self.view_center(window);
                    ([x - SIZE[0] / 2., y - SIZE[1] / 2.], doc.room.clone())
                }
            };
            let id = self.add_browser_pane(String::new(), at, space, cx);
            self.solve_layout(Some(&id), None, cx);
            id
        });
        if self.canvas_state.browser_tab.as_deref().is_some_and(|expanded| expanded != id) {
            self.collapse_browser(window, cx);
        }
        self.select_browser_pane(&id);
        if let Err(e) = self.new_browser_tab(&id, url, true, window, cx) {
            self.command_error("Open link", &e, window, cx);
        }
    }

    /// canvas.browser_back | forward | reload | home | pick | shot | external | copy | viewport
    /// | expand | pop | cookies | links | clear | inspect | open_file | tab_new | tab_select | tab_close
    /// (and annotate, which another module handles).
    pub(in crate::window) fn browser_command(&mut self, name: &str, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) {
        if name == "canvas.browser_cookies" {
            let Ok(keep) = serde_json::from_value::<settings::KeepCookies>(a["keep"].clone()) else {
                return self.fail("keep must be until_quit or forever");
            };
            return self.write_person(|person| person.browser_keep_cookies = keep, window, cx);
        }
        if name == "canvas.browser_links" {
            let Ok(open) = serde_json::from_value::<settings::OpenLinks>(a["open"].clone()) else {
                return self.fail("open must be built_in or default");
            };
            return self.write_person(|person| person.browser_open_links = open, window, cx);
        }
        if a["pane"].is_null() && self.selected_browser().is_none() {
            if name == "canvas.browser_tab_new" {
                return self.canvas_keyboard("canvas.new", &Value::Null, window, cx);
            }
            if name == "canvas.browser_tab_close" {
                if crate::keys::listing().iter().any(|(_, _, action)| {
                    action == "tab.close" || action.starts_with("tab.close:")
                        || serde_json::from_str::<Value>(action).is_ok_and(|binding| binding["command"] == "tab.close")
                }) {
                    let reply = self.run_as("tab.close", Value::Null, person, window, cx);
                    self.reply = Some(reply);
                }
                return;
            }
        }
        let id = match self.browser_target(a) { Ok(id) => id, Err(e) => return self.fail(e) };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { home, .. } = &pane.kind else { return };
        let home = home.clone();
        let url = pane.url().unwrap().to_owned();
        if url.is_empty() && matches!(name, "canvas.browser_home" | "canvas.browser_reload" | "canvas.browser_back" | "canvas.browser_forward") { return; }
        match name {
            "canvas.browser_tab_new" => {
                let url = match &a["url"] {
                    Value::Null => Ok(String::new()),
                    Value::String(url) => browser_target_url(url),
                    _ => Err("url must be a string".into()),
                };
                let result = url.and_then(|url| self.new_browser_tab(&id, url, false, window, cx))
                    .map(|tab| json!({"pane":id,"tab":tab}));
                self.answer(result);
            }
            "canvas.browser_tab_select" => {
                let Some(tab) = a["tab"].as_str() else { return self.fail("tab must be a tab id"); };
                if let Err(e) = self.select_tab(&id, tab, window, cx) { self.fail(e); }
            }
            "canvas.browser_tab_close" => {
                if !a["tab"].is_null() && !a["tab"].is_string() { return self.fail("tab must be a tab id"); }
                if let Err(e) = self.close_browser_tab(&id, a["tab"].as_str(), window, cx) { self.fail(e); }
            }
            "canvas.browser_external" => {
                // A tab's own menu names its tab; otherwise the active tab opens.
                let url = match a["tab"].as_str() {
                    Some(tab) => match self.tab_by_key(&tab_key(&id, tab)) {
                        Some(t) => t.url.clone(),
                        None => return self.fail(format!("no tab {tab} in browser pane {id}")),
                    },
                    None => url,
                };
                if url.is_empty() { return self.fail(ENTER_URL); }
                crate::links::open(&crate::links::Target::Url(url));
            }
            "canvas.browser_open_file" => {
                if !person { return self.fail("Use a path or file URL to open a file from a command"); }
                #[cfg(target_os = "macos")]
                {
                    let (send, picked) = async_channel::bounded(1);
                    pick_html(move |path| { let _ = send.try_send(path); });
                    cx.spawn_in(window, async move |this, cx| {
                        let Ok(Some(path)) = picked.recv().await else { return };
                        let _ = this.update_in(cx, |this, window, cx| {
                            let is_html = path.extension().and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm"));
                            let result = if is_html {
                                checked_file(&path).and_then(|path| file_url(&path))
                                    .and_then(|url| this.new_browser_tab(&id, url, true, window, cx))
                            } else { Err("Choose an .html or .htm file".into()) };
                            if let Err(e) = result { this.command_error("Open HTML file", &e, window, cx); }
                        });
                    }).detach();
                }
                #[cfg(not(target_os = "macos"))]
                self.fail("browser panes run on macOS only");
            }
            "canvas.browser_allow_folder" => {
                if !person || !url.starts_with("file:") { return self.fail("Choose a folder for an open local file"); }
                #[cfg(target_os = "macos")]
                {
                    let key = self.page_key(&id);
                    let picked = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Choose a folder containing the file and its assets".into()) });
                    cx.spawn_in(window, async move |this, cx| {
                        let Ok(Ok(Some(paths))) = picked.await else { return };
                        let Some(path) = paths.first() else { return };
                        let _ = this.update_in(cx, |this, window, cx| {
                            if this.page_key(&id) != key || this.browser_url(&id).ok() != Some(url.as_str()) { return; }
                            if let Err(e) = this.allow_browser_folder(&id, &url, path, window, cx) {
                                this.command_error("Allow folder assets", &e, window, cx);
                            }
                        });
                    }).detach();
                }
                #[cfg(not(target_os = "macos"))]
                self.fail("browser panes run on macOS only");
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
                #[cfg(target_os = "macos")]
                if let Err(e) = self.annotation_pick(&id, window, cx) { self.fail(e); }
                #[cfg(not(target_os = "macos"))]
                self.fail("browser panes run on macOS only for now");
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

    /// Saves the CSS viewport and updates a live pane's zoom at its current size.
    fn set_browser_viewport(&mut self, id: &str, size: Option<[f32; 2]>, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        let key = self.page_key(id);
        let state = &mut self.canvas_state;
        let Some(pane) = state.doc.panes.iter_mut().find(|p| p.id == id) else { return };
        let canvas::Kind::Browser { viewport, .. } = &mut pane.kind else { return };
        *viewport = size;
        if let Some(shown) = state.shown.as_mut().and_then(|doc| doc.panes.iter_mut().find(|p| p.id == id))
            && let canvas::Kind::Browser { viewport, .. } = &mut shown.kind
        {
            *viewport = size;
        }
        // A live page rescales in place; a resting one lays out again and retakes its snapshot.
        #[cfg(target_os = "macos")]
        if state.browser_tab.is_none() {
            let live = key.as_ref().and_then(|key| state.browsers.pages.get(key)).filter(|p| p.live.is_some())
                .and_then(|page| page.native.as_ref()).map(|native| native.raw.clone());
            match live {
                Some(raw) => {
                    let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
                    let _ = raw.raw().zoom(self.host_frame(pane).1);
                }
                None => self.resize_resting_browser(id, cx),
            }
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
        let key = self.page_key(id).unwrap();
        let page = self.canvas_state.browsers.pages.get_mut(&key).unwrap();
        if page.loading { page.waiting.push(ask); } else { self.answer_browser(&key, ask); }
        Ok(())
    }

    /// Loads a target in the active tab, or in a new tab when requested.
    pub(super) fn browser_open(&mut self, id: &str, target: &str, tab: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let url = browser_target_url(target)?;
        if tab == "new" {
            let tab = self.new_browser_tab(id, url.clone(), false, window, cx)?;
            return Ok(json!({"pane":id,"url":url,"tab":tab}));
        }
        if !tab.is_null() { return Err("tab must be new".into()); }
        self.load_browser(id, &url, true, window, cx);
        Ok(json!({"pane":id,"url":url}))
    }

    /// Loads `url`, saving it (and as Home when `home`); toasts when the view cannot be made.
    fn load_browser(&mut self, id: &str, url: &str, home: bool, window: &mut Window, cx: &mut Context<Self>) {
        if url.is_empty() { return; }
        let Some(key) = self.page_key(id) else { return };
        #[cfg(target_os = "macos")]
        let previous = self.browser_url(id).unwrap_or_default().to_owned();
        #[cfg(target_os = "macos")]
        let mut placement = None;
        #[cfg(target_os = "macos")]
        if let Some(page) = self.canvas_state.browsers.pages.get_mut(&key) {
            if previous.starts_with("file:") || url.starts_with("file:") {
                if previous != url { page.local_folder = None; }
                placement = page.live;
                self.release_page(&key, "the browser opened another address");
            }
        }
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).unwrap();
        pane.set_url(url.to_owned());
        if home && let canvas::Kind::Browser { home, .. } = &mut pane.kind { *home = url.to_owned(); }
        self.canvas_changed(cx);
        let made = self.canvas_state.browsers.pages.get(&key).is_some_and(Page::has_view);
        if let Err(e) = self.ensure_view(id, window, cx) {
            self.toast(Toast { title: "Page not opened".into(), body: Some(e), level: Level::Error, action: None }, window, cx);
            return;
        }
        // A new view already loads the saved URL.
        if made { self.navigate_browser(&key, url, cx); }
        #[cfg(target_os = "macos")]
        if let Some(placement) = placement {
            self.canvas_state.browsers.pages.get_mut(&key).unwrap().live = Some(placement);
        }
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
    fn has_view(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.native.is_some();
        #[cfg(not(target_os = "macos"))]
        false
    }
}

/// The local-file rule list, compiled once into the app's store and looked up after that.
/// Opens that arrive while it loads wait for the same lookup or compile. Bump the version
/// when the rules change, so a stale compiled list is never reused.
#[cfg(target_os = "macos")]
fn local_rules(
    store: objc2::rc::Retained<objc2_web_kit::WKContentRuleListStore>,
    done: impl FnOnce(Option<objc2::rc::Retained<objc2_web_kit::WKContentRuleList>>) + 'static,
) {
    use objc2::rc::Retained;
    use objc2_foundation::{NSError, NSString};
    use objc2_web_kit::WKContentRuleList;
    type Waiter = Box<dyn FnOnce(Option<Retained<WKContentRuleList>>)>;
    enum Rules { Unloaded, Loading(Vec<Waiter>), Ready(Retained<WKContentRuleList>) }
    thread_local! { static RULES: std::cell::RefCell<Rules> = const { std::cell::RefCell::new(Rules::Unloaded) }; }
    const ID: &str = "canopy-local-files-v1";
    const SOURCE: &str = r#"[{"trigger":{"url-filter":"^[a-z]+://"},"action":{"type":"block"}},{"trigger":{"url-filter":"^file://"},"action":{"type":"ignore-previous-rules"}}]"#;
    let mut done: Option<Waiter> = Some(Box::new(done));
    // Ok(true): this call starts the lookup; Ok(false): it waits for one already running.
    let start = RULES.with_borrow_mut(|rules| match rules {
        Rules::Ready(list) => Err(list.clone()),
        Rules::Loading(waiters) => { waiters.extend(done.take()); Ok(false) }
        Rules::Unloaded => { *rules = Rules::Loading(done.take().into_iter().collect()); Ok(true) }
    });
    match start {
        Err(list) => return done.take().unwrap()(Some(list)),
        Ok(false) => return,
        Ok(true) => {}
    }
    let finish = block2::RcBlock::new(|list: *mut WKContentRuleList, _: *mut NSError| {
        // SAFETY: WebKit passes a live list (or null) for the duration of its completion.
        let list = unsafe { Retained::retain(list) };
        // A failed compile is not cached, so the next open tries again.
        let next = list.clone().map_or(Rules::Unloaded, Rules::Ready);
        let waiters = RULES.with_borrow_mut(|rules| match std::mem::replace(rules, next) {
            Rules::Loading(waiters) => waiters,
            Rules::Unloaded | Rules::Ready(_) => Vec::new(),
        });
        for waiter in waiters { waiter(list.clone()); }
    });
    let lookup = block2::RcBlock::new({
        let (store, finish) = (store.clone(), finish.clone());
        move |list: *mut WKContentRuleList, error: *mut NSError| {
            if !list.is_null() { return finish.call((list, error)); }
            let (id, source) = (NSString::from_str(ID), NSString::from_str(SOURCE));
            // SAFETY: the store and its completion run on the main thread.
            unsafe { store.compileContentRuleListForIdentifier_encodedContentRuleList_completionHandler(Some(&id), Some(&source), Some(&finish)); }
        }
    });
    // SAFETY: the store and its completion run on the main thread.
    unsafe { store.lookUpContentRuleListForIdentifier_completionHandler(Some(&NSString::from_str(ID)), Some(&lookup)); }
}

/// Local HTML can run its own scripts. Block network resources before loading the file so a
/// script fetched from the network cannot run with the local page's read access.
#[cfg(target_os = "macos")]
fn load_local_file(
    view: &wry::WebView,
    path: &std::path::Path,
    folder: Option<&std::path::Path>,
    alive: std::rc::Weak<()>,
    failed: impl Fn(String) + 'static,
) -> Result<(), String> {
    use objc2_foundation::NSURL;
    use objc2_web_kit::WKContentRuleListStore;
    use wry::WebViewExtMacOS as _;
    let file = NSURL::from_file_path(path).ok_or("Invalid browser file path")?;
    let access = match folder {
        Some(folder) => NSURL::from_directory_path(folder).ok_or("Invalid browser folder path")?,
        None => file.clone(),
    };
    let rules_dir = crate::mods::app_dir().join("browser").join("content-rules");
    std::fs::create_dir_all(&rules_dir).map_err(|e| format!("Cannot create WebKit content rules store: {e}"))?;
    let rules_url = NSURL::from_directory_path(rules_dir).ok_or("Invalid WebKit content rules path")?;
    let mtm = objc2::MainThreadMarker::new().expect("WebKit loads on the main thread");
    // SAFETY: creation, completion and WebKit calls all run on the main thread.
    let store = unsafe { WKContentRuleListStore::storeWithURL(Some(&rules_url), mtm) }
        .ok_or("Cannot open WebKit content rules store")?;
    let view = view.webview();
    local_rules(store, move |rules| {
        if alive.upgrade().is_none() { return; }
        let Some(rules) = rules else { return failed("WebKit could not restrict local file access".into()) };
        // SAFETY: the rules apply before the file loads, on the main thread.
        unsafe {
            view.configuration().userContentController().addContentRuleList(&rules);
            if view.loadFileURL_allowingReadAccessToURL(&file, &access).is_none() {
                failed("WebKit could not open the local file".into());
            }
        }
    });
    Ok(())
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
    super::browser_mask::clear_mask(wv);
    wv.webview().removeFromSuperview();
}

/// Lays a resting view out at `host_frame`'s size and zoom. gpui-wry sizes a view only while
/// it is drawn; a resting page still lays out (read) and snapshots (shot) this way.
#[cfg(target_os = "macos")]
fn fit_view(view: &gpui_wry::WebView, ([width, height], zoom): ([f32; 2], f64)) {
    let _ = view.raw().set_bounds(wry::Rect {
        position: wry::dpi::LogicalPosition::new(0., 0.).into(),
        size: wry::dpi::LogicalSize::new(f64::from(width), f64::from(height)).into(),
    });
    let _ = view.raw().zoom(zoom);
}

/// A popped-out pane's window: the pane's header over the active tab's page. It holds no state
/// of its own; it draws what `AppWindow` keeps and redraws when that changes.
#[cfg(target_os = "macos")]
struct BrowserWindow {
    app: WeakEntity<AppWindow>,
    pane: String,
    title: String,
    _follow: Option<Subscription>,
}

#[cfg(target_os = "macos")]
impl Render for BrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = window.viewport_size();
        let parts = self.app.update(cx, |app, cx| {
            app.sync_url_input(&self.pane, Host::Window, window, cx);
            if let Some(ui) = app.canvas_state.browsers.panes.get_mut(&self.pane)
                && std::mem::take(&mut ui.focus_url)
                && let Some((input, _)) = &ui.pop_input
            {
                input.update(cx, |i, cx| i.focus(window, cx));
            }
            let title = app.canvas_state.doc.panes.iter().find(|p| p.id == self.pane).map(|p| app.browser_title(p).0);
            (title, app.browser_chrome(&self.pane, Host::Window, cx), app.popped_body(&self.pane, view, cx), app.pop_menu(&self.pane, view, cx))
        }).ok();
        let Some((title, chrome, body, menu)) = parts else { return div().into_any_element() };
        if let Some(title) = title.filter(|t| *t != self.title) {
            window.set_window_title(&title);
            self.title = title;
        }
        div().relative().size_full().flex().flex_col().bg(c("canvas")).text_color(c("foreground")).font_family(".SystemUIFont")
            .child(chrome)
            .child(div().flex_1().min_h_0().child(body))
            .children(menu)
            .into_any_element()
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
    /// Makes the tab's web view (hidden, loading the saved URL) unless it has one. A popped pane's
    /// views are built in its pop-out window, whichever window asked.
    // shortcut: hidden views stay allocated until their tab closes, drop views on rest if memory matters.
    pub(super) fn ensure_view(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        let key = self.page_key(id).unwrap();
        if self.canvas_state.browsers.pages.get(&key).is_some_and(Page::has_view) { return Ok(()); }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id && p.url().is_some()).ok_or_else(|| format!("no browser pane {id}"))?;
        let (url, frame) = (pane.url().unwrap().to_owned(), self.host_frame(pane));
        let local = if url.starts_with("file:") {
            let path = checked_file(&file_path(&url)?)?;
            // Recheck after rest, pop-out, or restore. A stale folder grant cannot follow a
            // symlink to a different file or directory.
            let folder = self.canvas_state.browsers.pages.get(&key).and_then(|page| page.local_folder.as_ref())
                .filter(|(granted_url, folder)| granted_url == &url && path.starts_with(folder))
                .and_then(|(_, folder)| folder.canonicalize().ok().filter(|current| current == folder));
            let address = file_url(&path)?;
            Some((address, path, folder))
        } else { None };
        let popped = self.canvas_state.browsers.panes.get(id).and_then(|ui| ui.popped);
        let tx = self.browser_events(window, cx);
        self.canvas_state.browsers.next_view += 1;
        let view_id = self.canvas_state.browsers.next_view;
        let build_key = key.clone();
        let build = move |window: &mut Window, cx: &mut App| -> Result<Native, String> {
            let key = build_key;
            let send = |event: fn(String) -> Event| {
                let (tx, key) = (tx.clone(), key.clone());
                move |s: String| { let _ = tx.try_send((key.clone(), view_id, event(s))); }
            };
            let (load_tx, load_key) = (tx.clone(), key.clone());
            let (open_tx, open_key) = (tx.clone(), key.clone());
            let (nav_tx, nav_key) = (tx.clone(), key.clone());
            let local_url = local.as_ref().map(|(address, _, _)| address.clone());
            let is_local = local.is_some();
            let download_error = send(Event::LoadError);
            let builder = if is_local {
                wry::WebViewBuilder::new().with_incognito(true)
            } else {
                browser_store(wry::WebViewBuilder::new()).with_url(url)
            };
            let webview = builder
                .with_navigation_handler(move |url| {
                    let permitted = permits_navigation(local_url.as_deref(), &url);
                    if !permitted && url != "about:blank" {
                        let _ = nav_tx.try_send((nav_key.clone(), view_id, Event::Blocked(url.clone())));
                    }
                    permitted || url == "about:blank"
                })
                .with_devtools(true)
                .with_visible(false)
                .with_accept_first_mouse(true)
                .with_initialization_script_for_main_only(page_script(), true)
                .with_document_title_changed_handler(send(Event::Title))
                .with_on_page_load_handler(move |event, _| {
                    let event = match event { wry::PageLoadEvent::Started => Event::Started, wry::PageLoadEvent::Finished => Event::Loaded };
                    let _ = load_tx.try_send((load_key.clone(), view_id, event));
                })
                .with_new_window_req_handler(move |url, _| {
                    let _ = open_tx.try_send((open_key.clone(), view_id, Event::Open(url)));
                    wry::NewWindowResponse::Deny
                })
                // No downloads: a pane shows pages, it does not save files.
                .with_download_started_handler(move |_, _| {
                    if is_local { download_error("WebKit cannot display this file format".into()); }
                    false
                })
                .build_as_child(window)
                .map_err(|e| format!("cannot make the web view: {e}"))?;
            let local_load = if let Some((_, path, folder)) = local {
                let token = std::rc::Rc::new(());
                load_local_file(&webview, &path, folder.as_deref(), std::rc::Rc::downgrade(&token), send(Event::LoadError))?;
                Some(token)
            } else { None };
            let clip = super::browser_mask::wrap(&webview)?;
            let view = cx.new(|cx| gpui_wry::WebView::new(webview, window, cx));
            let raw = view.update(cx, |v, _| {
                v.hide();
                // A popped view is sized by its window as it draws.
                if popped.is_none() { fit_view(v, frame); }
                v.handle()
            });
            Ok(Native { view, raw, clip, _local_load: local_load })
        };
        // The window that asked may itself be the pop-out, which cannot be updated from inside.
        let native = match popped {
            Some(handle) if handle != window.window_handle() => handle.update(cx, |_, window, cx| build(window, cx)).map_err(|e| e.to_string())??,
            _ => build(window, cx)?,
        };
        let page = self.canvas_state.browsers.pages.entry(key.clone()).or_default();
        page.native = Some(native);
        page.view_id = view_id;
        self.begin_load(&key, cx);
        Ok(())
    }

    /// Expands a page into the top strip's tab: live over the whole canvas area, at zoom 1. The
    /// canvas then draws nothing else (`render_canvas`). Refuses a popped page, or one in
    /// another space.
    fn expand_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).cloned().ok_or_else(|| format!("no pane {id}"))?;
        if self.canvas_state.doc.docked(id) { return Err("Use Expand panel for a browser in the Files panel".into()); }
        if !self.canvas_state.doc.shows(pane.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        if self.browser_popped(id) { return Err(format!("pane {id} is popped out: bring it back first")); }
        self.require_browser_url(id)?;
        let frame = self.bounds.borrow().get("canopy-canvas").copied().ok_or("the canvas is not shown")?;
        self.ensure_view(id, window, cx)?;
        // The expanded page replaces all panes, so the other live pages step aside.
        let key = self.page_key(id).unwrap();
        let others: Vec<String> = self.canvas_state.browsers.pages.iter().filter(|(k, p)| k.as_str() != key && p.live.is_some()).map(|(k, _)| k.clone()).collect();
        for other in others { self.rest_page(&other, cx); }
        let camera = self.canvas_state.doc.camera;
        let page = self.canvas_state.browsers.pages.get_mut(&key).unwrap();
        page.live = Some((camera, pane.rect));
        (page.told, page.watch) = (false, false);
        page.native.as_ref().unwrap().view.update(cx, |v, _| {
            show_view(v, frame, 1.);
        });
        self.canvas_state.browser_tab = Some(id.to_owned());
        self.select_browser_pane(id);
        cx.notify();
        Ok(())
    }

    /// Puts the expanded page (if any) back in its pane: laid out at the pane's size again, then
    /// rested the usual way. Called by every way out: the command, Escape, any space tab, close.
    pub(super) fn collapse_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.canvas_state.browser_tab.take() else { return };
        let frame = self.canvas_state.doc.panes.iter().find(|p| p.id == id).map(|p| self.host_frame(p));
        if let (Some(frame), Some(native)) = (frame, self.active_page(&id).and_then(|p| p.native.as_ref())) {
            native.view.update(cx, |v, _| fit_view(v, frame));
        }
        // shortcut: the snapshot is taken right after the resize, so it may predate WebKit's relayout; a rested page re-snapshots when it next loads.
        self.rest_browser(&id, cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Pops a pane out into its own window (the same header over the active tab's page), or
    /// brings it back. Every tab's page then lives in the window; the pane's own views are
    /// destroyed first and the pane says so. Website data stays in the shared store, so sign-ins
    /// carry over both ways.
    fn pop_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.browser_popped(id) {
            if let Some(popped) = self.unpop_browser(id, cx) {
                // The window may be the one asking: it cannot be updated from inside.
                if popped == window.window_handle() { window.remove_window(); } else { let _ = popped.update(cx, |_, window, _| window.remove_window()); }
            }
            return Ok(cx.notify());
        }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id && p.url().is_some()).cloned().ok_or_else(|| format!("no browser pane {id}"))?;
        if self.canvas_state.browser_tab.as_deref() == Some(id) { self.collapse_browser(window, cx); }
        for key in self.pane_page_keys(id) {
            self.note_browser_url(&key, cx);
            self.release_page(&key, "the page was popped out");
        }
        let (title, _) = self.browser_title(&pane);
        let owner = window.window_handle().downcast::<AppWindow>().ok_or("pop out from the app window")?;
        let app = cx.entity().downgrade();
        let (closed, shown, id) = (id.to_owned(), id.to_owned(), id.to_owned());
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(pane.rect[2].max(640.)), px(pane.rect[3].max(480.))), cx))),
            titlebar: Some(TitlebarOptions { title: Some(title.clone().into()), appears_transparent: true, traffic_light_position: Some(point(px(12.), px(11.))) }),
            // The header draws the title bar and moves the window itself (`start_window_move`).
            app_owns_titlebar_drag: true,
            ..Default::default()
        };
        // Opening draws the window at once, and its header reads this window: do it once this
        // update has ended.
        cx.defer(move |cx| {
            let opened = cx.open_window(options, move |window, cx| {
                // The red light brings the pane back; the window closes on its own.
                window.on_window_should_close(cx, move |_, cx| {
                    let _ = owner.update(cx, |this, _, cx| { this.unpop_browser(&closed, cx); cx.notify(); });
                    true
                });
                cx.new(|cx| {
                    // The header follows the pane's state.
                    let follow = app.upgrade().map(|app| cx.observe(&app, |_, _, cx| cx.notify()));
                    BrowserWindow { app, pane: shown, title, _follow: follow }
                })
            });
            let _ = owner.update(cx, |this, window, cx| match opened {
                Ok(handle) => {
                    if this.canvas_state.doc.panes.iter().any(|p| p.id == id) && !this.browser_popped(&id) {
                        this.canvas_state.browsers.panes.entry(id.clone()).or_default().popped = Some(handle.into());
                        // An empty tab has no page to build yet.
                        if this.browser_url(&id).is_ok_and(|url| !url.is_empty()) && let Err(e) = this.ensure_view(&id, window, cx) {
                            this.toast(Toast { title: "Page not opened".into(), body: Some(e), level: Level::Error, action: None }, window, cx);
                        }
                    } else {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    }
                    cx.notify();
                }
                Err(e) => this.toast(Toast { title: "Pop out failed".into(), body: Some(e.to_string()), level: Level::Error, action: None }, window, cx),
            });
        });
        cx.notify();
        Ok(())
    }

    /// Ends a pop-out: the pane stops being popped and its pages' views, built in the window,
    /// are destroyed (rebuilt in the pane when next used). Returns the window, which the caller closes.
    fn unpop_browser(&mut self, id: &str, cx: &mut Context<Self>) -> Option<AnyWindowHandle> {
        let ui = self.canvas_state.browsers.panes.get_mut(id)?;
        let popped = ui.popped.take()?;
        (ui.pop_input, ui.pop_menu) = (None, None);
        for key in self.pane_page_keys(id) { self.release_page(&key, "the page came back to its pane"); }
        cx.notify();
        Some(popped)
    }

    /// Destroys a page's view and forgets that it was live, answering what waited on it with `why`.
    fn release_page(&mut self, key: &str, why: &str) {
        self.canvas_state.browsers.last_masks.borrow_mut().remove(key);
        let Some(page) = self.canvas_state.browsers.pages.get_mut(key) else { return };
        if let Some(native) = page.native.take() { destroy(native); }
        (page.live, page.loading, page.watch) = (None, false, false);
        for ask in std::mem::take(&mut page.waiting) {
            let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
            let _ = tx.send(json!({"error": why}));
        }
    }

    /// The pop-out's page area at window size `view`: the active tab's view (shown, the others
    /// hidden, and all hidden while the menu is open or an annotation shows its still, since a
    /// native view draws over GPUI), or a note for an empty tab.
    fn popped_body(&self, id: &str, view: Size<Pixels>, cx: &mut Context<Self>) -> AnyElement {
        let menu = self.canvas_state.browsers.panes.get(id).is_some_and(|ui| ui.pop_menu.is_some())
            || self.canvas_state.annotations.get(id).is_some_and(|a| !a.live);
        let active = self.page_key(id);
        for key in self.pane_page_keys(id) {
            let Some(native) = self.canvas_state.browsers.pages.get(&key).and_then(|p| p.native.as_ref()) else { continue };
            let show = !menu && active.as_ref() == Some(&key);
            native.view.update(cx, |v, _| { if show && !v.visible() { v.show(); } else if !show && v.visible() { v.hide(); } });
        }
        match active.and_then(|key| self.canvas_state.browsers.pages.get(&key)).and_then(|p| p.native.as_ref()) {
            Some(native) => {
                let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id) else { return div().into_any_element() };
                let body = div().size_full().bg(gpui::white()).child(native.view.clone()).into_any_element();
                let size = [f32::from(view.width), (f32::from(view.height) - canvas::BROWSER_HEADER).max(1.)];
                self.annotate_body(pane, body, size, cx)
            }
            None => div().size_full().flex().flex_col().items_center().justify_center().gap(px(8.)).bg(c("canvas"))
                .child(svg().path("icons/globe.svg").size(px(32.)).text_color(c("muted-foreground")))
                .child(div().text_size(px(15.)).text_color(c("foreground")).child("Browse the web"))
                .child(div().text_size(px(13.)).text_color(c("muted-foreground")).child("Type a URL to open a site."))
                .into_any_element(),
        }
    }

    /// Clears data in open views (a pop-out's pages are among them) and asks WebKit to remove the named store.
    fn clear_browsing_data(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let mut views = Vec::new();
        for page in self.canvas_state.browsers.pages.values() {
            if let Some(native) = &page.native {
                views.push(native.raw.clone());
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
    fn browser_events(&mut self, window: &mut Window, cx: &mut Context<Self>) -> async_channel::Sender<(String, u64, Event)> {
        if let Some(tx) = &self.canvas_state.browsers.events { return tx.clone(); }
        let (tx, rx) = async_channel::unbounded::<(String, u64, Event)>();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok((key, view, event)) = rx.recv().await {
                if this.update_in(cx, |this, window, cx| this.browser_event(key, view, event, window, cx)).is_err() { break; }
            }
        }).detach();
        self.canvas_state.browsers.events = Some(tx.clone());
        tx
    }

    /// Makes the page live at the pane's current scale and window position.
    pub(super) fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        if self.browser_popped(id) { return Err(format!("pane {id} is popped out: bring it back first")); }
        let doc = &self.canvas_state.doc;
        let saved = doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        // A docked page is placed by the panel body, not the canvas camera.
        let docked = self.docked_view(id);
        let (pane, camera) = match docked {
            Some((view, _)) => (view, canvas::Camera::default()),
            None => (self.canvas_state.shown.as_ref().and_then(|shown| shown.panes.iter().find(|p| p.id == id)).unwrap_or(saved).clone(), self.canvas_state.doc.camera),
        };
        if !doc.docked(id) && !doc.shows(saved.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        let frame = self.body_rect(saved).ok_or("the canvas is not shown")?;
        let [_, _, width, height] = pane_body(&pane, camera);
        let zoom = page_zoom(&pane, [width, height], placement(&pane, camera).0.zoom);
        self.ensure_view(id, window, cx)?;
        let key = self.page_key(id).unwrap();
        let page = self.canvas_state.browsers.pages.get_mut(&key).unwrap();
        page.live = Some(placement(&pane, camera));
        (page.told, page.watch) = (false, false);
        page.native.as_ref().unwrap().view.update(cx, |v, _| show_view(v, frame, zoom));
        self.select_browser_pane(id);
        self.canvas_changed(cx);
        cx.notify();
        Ok(())
    }

    pub(super) fn rest_live_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.active_page(id).is_some_and(|page| page.live.is_some()) {
            self.rest_browser(id, cx);
        }
    }

    /// Steps a live page aside: snapshot, then hide (in `Event::Rested`), so the pane never
    /// shows blank. A resting page with a view only refreshes its snapshot.
    pub(super) fn rest_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        self.annotation_host_rested(id);
        if let Some(key) = self.page_key(id) { self.rest_page(&key, cx); }
    }

    fn rest_page(&mut self, key: &str, cx: &mut Context<Self>) {
        // A popped pane's pages live in its window: they never rest.
        if key.split_once('/').is_some_and(|(id, _)| self.browser_popped(id)) { return; }
        let Some(page) = self.canvas_state.browsers.pages.get_mut(key) else { return };
        let Some(view) = page.native.as_ref().map(|n| n.view.clone()) else { return };
        let view_id = page.view_id;
        let was_live = page.live.take().is_some();
        self.canvas_state.browsers.last_masks.borrow_mut().remove(key);
        page.watch = true;
        let Some(tx) = self.canvas_state.browsers.events.clone() else { return };
        let owned = key.to_owned();
        view.update(cx, |v, _| {
            if was_live { let _ = v.focus_parent(); }
            snapshot(v.raw(), move |shot| { let _ = tx.try_send((owned, view_id, Event::Rested(shot))); });
        });
        self.note_browser_url(key, cx);
        cx.notify();
    }

    /// Checks, once, each resting page that is no longer drawn (another place, or off the
    /// canvas view) for media still playing; a playing one is toasted (`Event::Playing`).
    pub(super) fn watch_pages(&self, drawn: &[String], cx: &mut Context<Self>) {
        let unseen: Vec<String> = self.canvas_state.browsers.pages.iter()
            .filter(|(key, page)| page.watch && !page.told && !drawn.contains(key)).map(|(key, _)| key.clone()).collect();
        if unseen.is_empty() { return; }
        let this = cx.entity();
        cx.defer(move |cx| this.update(cx, |this, _| for id in unseen {
            let Some(page) = this.canvas_state.browsers.pages.get_mut(&id).filter(|p| p.watch && !p.told) else { continue };
            page.watch = false;
            let (Some(raw), Some(tx)) = (page.native.as_ref().map(|n| n.raw.clone()), this.canvas_state.browsers.events.clone()) else { continue };
            let view = page.view_id;
            let script = "[...document.querySelectorAll('video,audio')].some(m => !m.paused && !m.ended && m.readyState > 2)";
            let _ = raw.raw().evaluate_script_with_callback(script, move |result| {
                if result.trim() == "true" { let _ = tx.try_send((id.clone(), view, Event::Playing)); }
            });
        }));
    }

    /// Runs a script in the page; false when the pane has no view.
    pub(super) fn eval_browser(&mut self, id: &str, script: &str) -> bool {
        let Some(raw) = self.raw_browser(id) else { return false };
        let _ = raw.raw().evaluate_script(script);
        true
    }

    pub(super) fn raw_browser(&self, id: &str) -> Option<gpui_wry::WebViewHandle> {
        self.raw_page(&self.page_key(id)?)
    }

    fn raw_page(&self, key: &str) -> Option<gpui_wry::WebViewHandle> {
        self.canvas_state.browsers.pages.get(key)?.native.as_ref().map(|n| n.raw.clone())
    }

    /// Runs a page-world function body and lets WebKit await its result.
    pub(super) fn eval_browser_json(&self, pane: &str, script: &str, done: impl FnOnce(Option<Value>) + 'static) {
        use objc2::{MainThreadMarker, runtime::AnyObject};
        use objc2_foundation::{NSError, NSString};
        use objc2_web_kit::WKContentWorld;
        use wry::WebViewExtMacOS as _;
        let Some(raw) = self.raw_browser(pane) else { return done(None) };
        let Some(mtm) = MainThreadMarker::new() else { return done(None) };
        let done = std::cell::Cell::new(Some(done));
        let callback = block2::RcBlock::new(move |value: *mut AnyObject, error: *mut NSError| {
            let Some(done) = done.take() else { return };
            // WebKit returns an NSString because the function body stringifies its result.
            let json = unsafe { value.as_ref() }
                .filter(|_| error.is_null())
                .and_then(|v| v.downcast_ref::<NSString>())
                .filter(|v| v.len_utf16() <= 1_000_000)
                .and_then(|v| serde_json::from_str(&v.to_string()).ok());
            done(json);
        });
        let body = NSString::from_str(&format!("return JSON.stringify(await (async () => {{ {script} }})())"));
        // Both the web view and page content world belong to the main thread.
        unsafe {
            raw.raw().webview().callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(
                &body, None, None, &WKContentWorld::pageWorld(mtm), Some(&callback),
            );
        }
    }

    /// Captures the same still used when a browser rests, with an RGBA copy for annotations.
    pub(super) fn freeze_browser(&mut self, pane: &str, done: impl FnOnce(&mut Self, Option<image::RgbaImage>, &mut Context<Self>) + 'static, cx: &mut Context<Self>) {
        let Some(key) = self.page_key(pane) else { return done(self, None, cx) };
        // A popped page stays in sight in its window: no "still playing" watch for it.
        let popped = self.browser_popped(pane);
        let Some(page) = self.canvas_state.browsers.pages.get_mut(&key) else { return done(self, None, cx) };
        let Some(view) = page.native.as_ref().map(|n| n.view.clone()) else { return done(self, None, cx) };
        let view_id = page.view_id;
        page.live = None;
        page.watch = !popped;
        self.canvas_state.browsers.last_masks.borrow_mut().remove(&key);
        let events = self.canvas_state.browsers.events.clone();
        let id = key.clone();
        let (tx, rx) = async_channel::bounded(1);
        view.update(cx, |v, _| {
            let _ = v.focus_parent();
            snapshot(v.raw(), move |shot| { let _ = tx.try_send(shot); });
        });
        self.note_browser_url(&key, cx);
        cx.spawn(async move |this, cx| {
            let shot = rx.recv().with_timeout(Duration::from_secs(2), cx.background_executor()).await.ok().and_then(Result::ok).flatten();
            if let Some(events) = events { let _ = events.try_send((id, view_id, Event::Rested(shot.clone()))); }
            let _ = this.update(cx, |this, cx| done(this, shot, cx));
        }).detach();
        cx.notify();
    }

    fn navigate_browser(&mut self, key: &str, url: &str, cx: &mut Context<Self>) {
        let Some(raw) = self.raw_page(key) else { return };
        self.begin_load(key, cx);
        let _ = raw.raw().load_url(url);
    }

    /// Answers one held reply from the loaded page.
    fn answer_browser(&mut self, key: &str, ask: Ask) {
        let Some(raw) = self.raw_page(key) else { return };
        // No page ever committed (the server refused or never answered): say so, since
        // WebKit would drop the script callback and the caller would hear nothing.
        if page_url(raw.raw()).is_none() {
            let url = self.tab_by_key(key).map(|tab| tab.url.as_str()).unwrap_or_default();
            let error = json!({"error": format!("{url} did not load: is its server running?")});
            let (Ask::Read(tx) | Ask::Shot(_, tx)) = ask;
            let _ = tx.send(error);
            return;
        }
        let id = key.split_once('/').unwrap().0.to_owned();
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
        if self.canvas_state.doc.docked(id) { return; }
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
        let popped = self.browser_popped(id);
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        if !popped && !self.canvas_state.shows_pane(pane) {
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
        // A popped page has its own window: no canvas to move.
        if popped {
            raw.raw().open_devtools();
            detach_inspector(raw.raw());
            return Ok(());
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
    // WebKit's store calls crash (WTF::RunLoop::dispatch, macOS 27.2) until something has set
    // WebKit up in this process; a configuration is the lightest thing that does.
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        // SAFETY: made on the main thread and dropped at once.
        let _ = unsafe { objc2_web_kit::WKWebViewConfiguration::new(mtm) };
    }
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
    pub(super) fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.require_browser_url(id)?;
        self.ensure_view(id, window, cx)
    }
    pub(super) fn rest_live_browser(&mut self, _: &str, _: &mut Context<Self>) {}
    pub(super) fn rest_browser(&mut self, _: &str, _: &mut Context<Self>) {}
    fn rest_page(&mut self, _: &str, _: &mut Context<Self>) {}
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
    pub(super) fn eval_browser(&mut self, _: &str, _: &str) -> bool { false }
    fn navigate_browser(&mut self, _: &str, _: &str, _: &mut Context<Self>) {}
    fn answer_browser(&mut self, _: &str, _: Ask) {}
}

#[cfg(test)]
mod tests {
    // Explicit: a glob would bring gpui's own `test` attribute in.
    #[cfg(target_os = "macos")]
    use super::browser_target_url;
    use super::{GRAB_REQUESTS, PRESETS, REACT_GRAB, format_quote, menu_items, open_in, page_script, page_zoom, parse_viewport, permits_navigation, resolve_source, web_url};
    use crate::canvas::{Kind, Pane};
    use crate::settings::{KeepCookies, OpenLinks};
    use serde_json::json;

    #[test]
    fn menu_marks_the_current_choices() {
        let checked = |viewport, keep, links| menu_items("p1", viewport, keep, links, "Open in Safari").iter()
            .filter_map(|item| item[0].as_str().and_then(|l| l.strip_prefix("✓ ")).map(str::to_owned)).collect::<Vec<_>>();
        assert_eq!(checked(None, KeepCookies::UntilQuit, OpenLinks::BuiltIn), ["Responsive", "Open links in built-in browser", "Until quit"]);
        assert_eq!(checked(Some([1280., 800.]), KeepCookies::Forever, OpenLinks::Default), ["1280 × 800", "Forever"]);
        let items = menu_items("p1", None, KeepCookies::UntilQuit, OpenLinks::Default, "Open in Safari");
        assert_eq!(items[0], json!(["Open in Safari", "canvas.browser_external", {"pane": "p1"}]));
        // Labels are not commands, and choosing the links row flips the setting.
        assert_eq!(items[4], json!(["Viewport", null, {"label": true}]));
        let links = items.iter().find(|i| i[1] == "canvas.browser_links").unwrap();
        assert_eq!(links[2], json!({"open": "built_in"}));
    }

    #[test]
    fn open_in_label_falls_back() {
        assert_eq!(open_in(Some("Arc")), "Open in Arc");
        assert_eq!(open_in(None), "Open in default browser");
    }

    #[test]
    fn web_and_local_views_cannot_navigate_into_each_other() {
        let file = "file:///tmp/example.html";
        assert!(permits_navigation(None, "https://localhost:3000/"));
        assert!(!permits_navigation(None, file));
        assert!(!permits_navigation(None, "FILE:///tmp/example.html"));
        assert!(permits_navigation(Some(file), file));
        assert!(permits_navigation(Some(file), "file:///tmp/example.html#details"));
        assert!(!permits_navigation(Some(file), "file:///tmp/other.html"));
        assert!(!permits_navigation(Some(file), "https://localhost:3000/"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn explicit_file_targets_use_canonical_files() {
        let root = std::env::temp_dir().join(format!("canopy-browser-target-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("web page #1.html");
        std::fs::write(&file, "").unwrap();
        let url = browser_target_url(file.to_str().unwrap()).unwrap();
        assert!(url.ends_with("/web%20page%20%231.html"));
        assert_eq!(browser_target_url(&url), Ok(url));
        assert!(browser_target_url("file://other-host/tmp/page.html").is_err());
        let extensionless = root.join("README");
        std::fs::write(&extensionless, "plain text").unwrap();
        assert!(browser_target_url(extensionless.to_str().unwrap()).is_ok());
        assert!(browser_target_url(root.to_str().unwrap()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

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
        let mut pane = Pane {
            id: "p1".into(), kind: Kind::Browser {
                tabs: vec![crate::canvas::Tab { id: "t1".into(), url: String::new(), title: String::new() }],
                active: 0, home: String::new(), viewport: None,
            },
            rect: [0., 0., 640., 838.], z: 1, space: None, color: None, pinned: None,
        };
        assert_eq!(page_zoom(&pane, [640., 800.], 0.5), 0.5, "a responsive page zooms with the canvas");
        let Kind::Browser { viewport, .. } = &mut pane.kind else { unreachable!() };
        *viewport = Some([1280., 800.]);
        assert_eq!(page_zoom(&pane, [640., 800.], 2.), 0.5);
        let Kind::Browser { viewport, .. } = &mut pane.kind else { unreachable!() };
        *viewport = Some([0., 0.]);
        assert_eq!(page_zoom(&pane, [640., 800.], 1.), 1.);
    }

    #[test]
    fn terminal_links_reuse_only_the_last_shown_browser() {
        assert_eq!(super::link_target(Some("p2"), ["p1", "p2"]), Some("p2".into()));
        assert_eq!(super::link_target(Some("p2"), ["p1"]), None);
        assert_eq!(super::link_target(None, ["p1"]), None);
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
    fn quotes_point_not_copy() {
        let react = json!({"start":42,"end":61,"source":{"url":"http://localhost:3000/pricing","component":"PricingCard","file":"/w/src/PricingCard.tsx","tag":format!("<div class=\"card {}\">", "x".repeat(100))}});
        let quote = format_quote(&react, Some("/w"));
        assert!(quote.starts_with("@http://localhost:3000/pricing <PricingCard> src/PricingCard.tsx:42-61 \"<div class=\\\"card xxx"), "{quote}");
        assert!(quote.ends_with("…\""), "the tag is cut: {quote}");
        let dom = json!({"text":"Buy now","message":" fix this ","source":{"url":"https://x.dev/","selector":"main > section:nth-of-type(2)"}});
        assert_eq!(format_quote(&dom, None), "@https://x.dev/ main > section:nth-of-type(2) \"Buy now\"\n\nfix this");
        let one = json!({"kind":"browser","path":"u","text":"","start":3,"end":3,"source":{"url":"u","component":"A","file":"/x/A.tsx","tag":"<a>"}});
        assert_eq!(super::super::format_quote(&one, Some("/w")), "@u <A> /x/A.tsx:3 \"<a>\"");
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
    let header = canvas::header_height(&pane.kind);
    [x + 1., y + header + 1., (w - 2.).max(0.), (h - header - 2.).max(0.)]
}

/// A fixed viewport fits inside the body at its original aspect ratio.
#[cfg(target_os = "macos")]
pub(super) fn viewport_rect(pane: &canvas::Pane, camera: canvas::Camera) -> canvas::Rect {
    let [x, y, w, h] = pane_body(pane, camera);
    let Some([css_w, css_h]) = fixed_viewport(pane) else { return [x, y, w, h] };
    let zoom = page_zoom(pane, [w, h], camera.zoom);
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
    /// Measure the same content slot GPUI passes to Wry, including a fixed viewport.
    fn measured_webview(&self, key: String, view: &Entity<gpui_wry::WebView>) -> Div {
        div().relative().size_full().child(self.browser_measure(format!("web-frame:{key}"))).child(view.clone())
    }

    fn browser_measure(&self, key: String) -> gpui::Canvas<()> {
        let bounds = self.bounds.clone();
        canvas(move |b, _, _| { bounds.borrow_mut().insert(key, b); }, |_, _, _, _| {})
            .absolute().size_full()
    }

    /// Window-space bounds of the live element, including the viewport's centering offset.
    pub(super) fn body_rect(&self, pane: &canvas::Pane) -> Option<Bounds<Pixels>> {
        if self.canvas_state.doc.docked(&pane.id) {
            let (view, panel) = self.docked_view(&pane.id)?;
            let [x, y, w, h] = viewport_rect(&view, canvas::Camera::default());
            return Some(Bounds::new(panel.origin + point(px(x), px(y)), size(px(w), px(h))));
        }
        let origin = self.bounds.borrow().get("canopy-canvas")?.origin;
        let [x, y, w, h] = viewport_rect(pane, self.canvas_state.doc.camera);
        Some(Bounds::new(origin + point(px(x), px(y)), size(px(w), px(h))))
    }

    /// The size and zoom a page lays out at in its measured host (the Files panel body, or the
    /// pane at the current camera, zoomed with it), so a resting snapshot matches the page when
    /// it wakes. Before the host is measured, the pane's world size at zoom 1.
    fn host_frame(&self, pane: &canvas::Pane) -> ([f32; 2], f64) {
        let docked = self.canvas_state.doc.docked(&pane.id);
        let scale = if docked { 1. } else { placement(pane, self.canvas_state.doc.camera).0.zoom };
        match self.body_rect(pane).map(|b| [f32::from(b.size.width), f32::from(b.size.height)]) {
            Some([w, h]) if w >= 1. && h >= 1. => ([w, h], fixed_viewport(pane).map_or(f64::from(scale), |[css_w, _]| f64::from(w / css_w))),
            _ => {
                let rect = pane.pinned.unwrap_or(pane.rect);
                ([rect[2], (rect[3] - canvas::BROWSER_HEADER).max(1.)], 1.)
            }
        }
    }

    pub(super) fn live_browser_views(&self) -> impl Iterator<Item = (&str, gpui_wry::WebViewHandle)> {
        self.canvas_state.browsers.pages.iter().filter_map(|(id, page)| {
            page.live?;
            Some((id.as_str(), page.native.as_ref()?.raw.clone()))
        })
    }
}

impl AppWindow {
    /// Retakes a resting browser's snapshot at its settled host size.
    pub(super) fn resize_resting_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(key) = self.page_key(id) && self.fit_resting(&key, cx) { self.rest_browser(id, cx); }
    }

    /// Lays a resting page out at `host_frame`; false when it has no view to fit.
    fn fit_resting(&self, key: &str, cx: &mut Context<Self>) -> bool {
        #[cfg(target_os = "macos")]
        // A popped page is sized by its own window.
        if let Some((id, _)) = key.split_once('/')
            && !self.browser_popped(id)
            && let Some(native) = self.canvas_state.browsers.pages.get(key).filter(|p| p.live.is_none()).and_then(|p| p.native.as_ref())
            && let Some(pane) = self.canvas_state.doc.panes.iter().find(|p| p.id == id)
        {
            let frame = self.host_frame(pane);
            native.view.update(cx, |v, _| fit_view(v, frame));
            return true;
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (key, cx);
        false
    }
}

#[cfg(all(test, target_os = "macos"))]
mod host_tests {
    use crate::canvas;
    use gpui::{Bounds, TestAppContext, point, px, size};
    use serde_json::json;
    use crate::window::tests::{in_own_process, open_window, running_alone};
    const TEST: &str = "window::canvas_view::browser::host_tests::resting_pages_fit_their_measured_host_headless";

    #[test]
    fn resting_pages_fit_their_measured_host() { in_own_process(TEST); }

    #[gpui::test]
    fn resting_pages_fit_their_measured_host_headless(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "browser-host", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, _, _| {
            this.canvas_state.doc.panes = serde_json::from_value(json!([{"id":"b1","kind":"browser",
                "props":{"tabs":[{"id":"t1","url":"https://example.com/"}],"active":0,"home":""},"rect":[0,0,964,740],"z":0}])).unwrap();
            this.bounds.borrow_mut().insert("canopy-canvas".into(), Bounds::new(point(px(0.), px(0.)), size(px(1200.), px(800.))));
            let pane = this.canvas_state.doc.panes[0].clone();
            // On the canvas: the camera scales the frame and the page zooms with it, so its
            // CSS viewport stays the pane's world width.
            this.canvas_state.doc.camera.zoom = 0.5;
            let ([w, _], zoom) = this.host_frame(&pane);
            assert_eq!((w, zoom), (480., 0.5));
            // Docked: the measured panel body, not the pane's world size.
            this.canvas_state.doc.dock("b1");
            this.canvas_state.file = Some(canvas::PanelTab::Pane("b1".into()));
            this.bounds.borrow_mut().insert("panel-body:b1".into(), Bounds::new(point(px(624.), px(40.)), size(px(576.), px(700.))));
            assert_eq!(this.host_frame(&pane), ([576., 700.], 1.));
            // A fixed viewport scales to fit the body at its own aspect ratio.
            let canvas::Kind::Browser { viewport, .. } = &mut this.canvas_state.doc.panes[0].kind else { unreachable!() };
            *viewport = Some([288., 350.]);
            let pane = this.canvas_state.doc.panes[0].clone();
            assert_eq!(this.host_frame(&pane), ([576., 700.], 2.));
        });
    }
}
