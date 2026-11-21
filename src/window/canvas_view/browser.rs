//! Browser panes: a web page on the canvas. Resting, the pane draws the page's last
//! snapshot like any other GPUI content (it zooms, clips and sits under menus). Live, after
//! a click into it, a WKWebView sits exactly over the pane body and takes input; any click
//! elsewhere in the window, or a camera or pane move, snapshots it and steps it aside, since
//! a native view cannot zoom, clip or sit under GPUI. See docs/wiki/Browser-Pane.md.
//!
//! The page loads only what the person or an agent opens: no homepage, an in-memory
//! (incognito) website store, no downloads, and the bundled react-grab with its
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
/// Opening tags in a quote are cut to this many characters, visible text to `TEXT_CUT`.
const TAG_CUT: usize = 80;
const TEXT_CUT: usize = 200;

/// Browser state that is not saved: native views, snapshots and waiting replies.
#[derive(Default)]
pub(in crate::window) struct Browsers {
    pages: HashMap<String, Page>,
    /// Page events into the window; made with the first web view.
    events: Option<async_channel::Sender<(String, Event)>>,
}

#[derive(Default)]
struct Page {
    #[cfg(target_os = "macos")]
    view: Option<Entity<gpui_wry::WebView>>,
    /// The same native view, for scripts run without a GPUI context.
    #[cfg(target_os = "macos")]
    raw: Option<gpui_wry::WebViewHandle>,
    /// The last snapshot, BGRA for GPUI.
    shot: Option<Arc<RenderImage>>,
    title: String,
    /// While live: where the native view was placed (see `placement`); any change rests it.
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
    /// Opens a browser pane on `url` (already checked by `web_url`) at `point` in `space`.
    pub(super) fn add_browser_pane(&mut self, url: String, point: canvas::Point, space: Option<String>, cx: &mut Context<Self>) -> String {
        let doc = &mut self.canvas_state.doc;
        let id = doc.next_file_id();
        let z = doc.panes.iter().map(|p| p.z).max().unwrap_or(0) + 1;
        doc.panes.push(canvas::Pane { id: id.clone(), kind: canvas::Kind::Browser { home: url.clone(), url }, rect: [point[0], point[1], SIZE[0], SIZE[1]], z, space, pinned: None });
        self.solve_layout(Some(&id), None, cx);
        self.canvas_changed(cx);
        cx.notify();
        id
    }

    /// Drops the views and snapshots of closed browser panes, and loads saved snapshots for
    /// panes not seen yet this run.
    pub(super) fn sync_browsers(&mut self) {
        let ids: Vec<_> = self.canvas_state.doc.panes.iter().filter(|p| p.url().is_some()).map(|p| p.id.clone()).collect();
        let pages = &mut self.canvas_state.browsers.pages;
        pages.retain(|id, _| {
            let keep = ids.contains(id);
            if !keep { let _ = std::fs::remove_file(shot_file(id)); }
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

    /// The pane body: the live web view, else the last snapshot, else the URL. A press on a
    /// resting body makes it live.
    pub(super) fn browser_body(&self, pane: &canvas::Pane, camera: canvas::Camera, cx: &mut Context<Self>) -> AnyElement {
        let id = pane.id.clone();
        let page = self.canvas_state.browsers.pages.get(&id);
        #[cfg(target_os = "macos")]
        if let Some(page) = page && let (Some(view), Some(at)) = (&page.view, page.live) {
            if at != placement(pane, camera) || !self.browser_fits(pane, camera) {
                let (this, id) = (cx.entity(), id.clone());
                cx.defer(move |cx| this.update(cx, |this, cx| this.rest_browser(&id, cx)));
            }
            // Presses on the page go to WebKit, never to GPUI: any press GPUI sees is outside.
            return div().size_full().bg(gpui::white()).child(view.clone())
                .on_mouse_down_out(cx.listener(move |this, _, _, cx| this.rest_browser(&id, cx)))
                .into_any_element();
        }
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
        match title { Some(title) => (title, url), None => (url, String::new()) }
    }

    /// Whether the pane body lies wholly inside the canvas: a native view cannot be clipped.
    fn browser_fits(&self, pane: &canvas::Pane, camera: canvas::Camera) -> bool {
        let Some(bounds) = self.bounds.borrow().get("canopy-canvas").copied() else { return false };
        let [x, y] = camera.to_screen([pane.rect[0], pane.rect[1]]);
        let (w, h) = (pane.rect[2] * camera.zoom, pane.rect[3] * camera.zoom);
        x >= 0. && y >= 0. && x + w <= f32::from(bounds.size.width) && y + h <= f32::from(bounds.size.height)
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
                if page.live.is_none() && let Some(view) = &page.view { view.update(cx, |v, _| v.hide()); }
                cx.notify();
            }
            Event::Message(body) => {
                if !std::mem::take(&mut page.picking) { return; }
                let Ok(message) = serde_json::from_str::<Value>(&body) else { return };
                if let Some(error) = message["error"].as_str() {
                    tracing::warn!("browser pane {id}: {error}");
                    self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"Pick failed","body":error,"level":"error"}}), window, cx);
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

    /// A picked element becomes a quote: the pane rests (GPUI menus cannot sit over a live
    /// page) and the quote popover opens over it.
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
        self.rest_browser(id, cx);
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

    /// canvas.browser_back | reload | home | pick | shot | external.
    pub(in crate::window) fn browser_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let id = match self.browser_target(a) { Ok(id) => id, Err(e) => return self.fail(e) };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
        let canvas::Kind::Browser { url, home } = pane.kind.clone() else { return };
        match name {
            "canvas.browser_external" => crate::links::open(&crate::links::Target::Url(url)),
            "canvas.browser_home" => self.load_browser(&id, &home, false, window, cx),
            "canvas.browser_reload" => self.eval_or_load(&id, "location.reload()", &url, window, cx),
            "canvas.browser_back" => self.eval_or_load(&id, "history.back()", &url, window, cx),
            "canvas.browser_pick" => {
                if let Err(e) = self.wake_browser_result(&id, window, cx) { return self.fail(e); }
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
                            Some(path) => json!({"title":"Screenshot saved","body":path,"level":"info"}),
                            None => json!({"title":"Screenshot failed","body":reply["error"],"level":"error"}),
                        };
                        this.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":note}), window, cx);
                    });
                }).detach();
            }
        }
    }

    /// Runs script in a page that has a view; else loads `url` (a reload with no view is a load).
    fn eval_or_load(&mut self, id: &str, script: &str, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.eval_browser(id, script) { self.load_browser(id, url, false, window, cx); }
    }

    /// `canvas.pane_read` and `canvas.pane_shot` for a browser pane: answered once the page
    /// has loaded, through the held control reply. Err when there is no caller to answer.
    pub(super) fn browser_reply(&mut self, id: &str, shot: Option<std::path::PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.held_reply.is_none() { return Err("a browser pane answers this only to `canopy pane` (an agent or script)".into()); }
        // Make the view first: an error must still reach the held caller.
        self.ensure_view(id, window, cx)?;
        let tx = self.held_reply.take().unwrap();
        let ask = match shot { Some(path) => Ask::Shot(path, tx), None => Ask::Read(tx) };
        self.browser_ask(id, ask, window, cx)
    }

    /// Answers now when the page is loaded, else when it finishes; makes the view if needed.
    fn browser_ask(&mut self, id: &str, ask: Ask, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
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
        if let Some(pane) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id)
            && let canvas::Kind::Browser { url: saved, home: saved_home } = &mut pane.kind
        {
            *saved = url.to_owned();
            if home { *saved_home = url.to_owned(); }
            self.canvas_changed(cx);
        }
        let made = self.canvas_state.browsers.pages.get(id).is_some_and(Page::has_view);
        if let Err(e) = self.ensure_view(id, window, cx) {
            self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"Page not opened","body":e,"level":"error"}}), window, cx);
            return;
        }
        // A new view already loads the saved URL.
        if made { self.navigate_browser(id, url, cx); }
        cx.notify();
    }

    /// The click on a resting page.
    fn wake_browser(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = self.wake_browser_result(id, window, cx) {
            self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":"Page not opened","body":e,"level":"error"}}), window, cx);
        }
    }
}

impl Page {
    fn has_view(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.view.is_some();
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
    // shortcut: a view lives until its pane closes, so each page opened this run keeps a WebKit
    // process and its hidden page's (throttled) scripts; drop views on rest if memory matters.
    fn ensure_view(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.canvas_state.browsers.pages.get(id).is_some_and(Page::has_view) { return Ok(()); }
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id && p.url().is_some()).ok_or_else(|| format!("no browser pane {id}"))?;
        let (url, rect) = (pane.url().unwrap().to_owned(), pane.rect);
        let tx = self.browser_events(window, cx);
        let send = |event: fn(String) -> Event| { let (tx, id) = (tx.clone(), id.to_owned()); move |s: String| { let _ = tx.try_send((id.clone(), event(s))); } };
        let (load_tx, load_id) = (tx.clone(), id.to_owned());
        let (open_tx, open_id) = (tx.clone(), id.to_owned());
        let ipc = send(Event::Message);
        let webview = wry::WebViewBuilder::new()
            .with_url(url)
            .with_incognito(true)
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
            // gpui-wry sizes a view only while it is drawn; a resting page still lays out
            // (read) and snapshots (shot) at the pane's body size, zoom 1.
            let _ = v.raw().set_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0., 0.).into(),
                size: wry::dpi::LogicalSize::new(rect[2] as f64, (rect[3] - canvas::HEADER).max(1.) as f64).into(),
            });
            v.handle()
        });
        let page = self.canvas_state.browsers.pages.entry(id.to_owned()).or_default();
        page.view = Some(view);
        page.raw = Some(raw);
        self.begin_load(id, cx);
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

    /// Makes the page live over its pane, first panning just enough to bring the whole pane
    /// into view; refuses when it cannot fit at this zoom.
    fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).cloned().ok_or_else(|| format!("no pane {id}"))?;
        if !self.canvas_state.doc.shows(pane.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        let bounds = self.bounds.borrow().get("canopy-canvas").copied().ok_or("the canvas is not shown")?;
        let camera = &mut self.canvas_state.doc.camera;
        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let [x, y] = camera.to_screen([pane.rect[0], pane.rect[1]]);
        let (pw, ph) = (pane.rect[2] * camera.zoom, pane.rect[3] * camera.zoom);
        if pw > w || ph > h { return Err("zoom out until the whole page fits the canvas, then click it".into()); }
        let shift = |at: f32, size: f32, limit: f32| if at < 0. { at } else if at + size > limit { at + size - limit } else { 0. };
        camera.x += shift(x, pw, w) / camera.zoom;
        camera.y += shift(y, ph, h) / camera.zoom;
        let camera = *camera;
        self.ensure_view(id, window, cx)?;
        let page = self.canvas_state.browsers.pages.get_mut(id).unwrap();
        page.live = Some(placement(&pane, camera));
        let view = page.view.clone().unwrap();
        // A pinned page keeps its screen size, so it reads at 100% whatever the canvas zoom.
        let zoom = if pane.pinned.is_some() { 1. } else { camera.zoom };
        view.update(cx, |v, _| {
            v.show();
            let _ = v.raw().zoom(zoom as f64);
            let _ = v.raw().focus();
        });
        self.canvas_state.selected = vec![id.to_owned()];
        self.canvas_changed(cx);
        cx.notify();
        Ok(())
    }

    /// Steps a live page aside: snapshot, then hide (in `Event::Rested`), so the pane never
    /// shows blank. A resting page with a view only refreshes its snapshot.
    pub(super) fn rest_browser(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.canvas_state.browsers.pages.get_mut(id) else { return };
        let Some(view) = page.view.clone() else { return };
        page.live = None;
        // A pick not finished before the page rests is over.
        page.picking = false;
        let Some(tx) = self.canvas_state.browsers.events.clone() else { return };
        let owned = id.to_owned();
        view.update(cx, |v, _| { let _ = v.focus_parent(); snapshot(v.raw(), move |shot| { let _ = tx.try_send((owned, Event::Rested(shot))); }); });
        self.note_browser_url(id, cx);
        cx.notify();
    }

    /// Runs a script in the page; false when the pane has no view.
    fn eval_browser(&mut self, id: &str, script: &str) -> bool {
        let Some(raw) = self.raw_browser(id) else { return false };
        let _ = raw.raw().evaluate_script(script);
        true
    }

    fn raw_browser(&self, id: &str) -> Option<gpui_wry::WebViewHandle> {
        self.canvas_state.browsers.pages.get(id)?.raw.clone()
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
}

#[cfg(not(target_os = "macos"))]
impl AppWindow {
    fn ensure_view(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) -> Result<(), String> {
        Err("browser panes run on macOS only for now".into())
    }
    fn wake_browser_result(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.ensure_view(id, window, cx)
    }
    pub(super) fn rest_browser(&mut self, _: &str, _: &mut Context<Self>) {}
    fn eval_browser(&mut self, _: &str, _: &str) -> bool { false }
    fn navigate_browser(&mut self, _: &str, _: &str, _: &mut Context<Self>) {}
    fn answer_browser(&mut self, _: &str, _: Ask) {}
}

#[cfg(test)]
mod tests {
    // Explicit: a glob would bring gpui's own `test` attribute in.
    use super::{GRAB_REQUESTS, REACT_GRAB, format_quote, jsx_lines, page_script, resolve_source, web_url};
    use serde_json::json;

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
