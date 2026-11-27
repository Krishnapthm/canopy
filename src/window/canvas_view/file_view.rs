use super::*;
use helper_bar::BarAction;

/// What a file can show besides its source. Decided once from the extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Previewable {
    Markdown,
    Jsonl,
    Html,
    Svg,
}

pub(super) fn previewable(path: &Path) -> Option<Previewable> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => Some(Previewable::Markdown),
        "jsonl" => Some(Previewable::Jsonl),
        "html" | "htm" => Some(Previewable::Html),
        "svg" => Some(Previewable::Svg),
        _ => None,
    }
}

/// Whether the file's rendered view is available on this platform.
pub(super) fn can_preview(path: &Path) -> bool {
    previewable(path).is_some_and(|kind| kind != Previewable::Html || browser::LOCAL_FILES_SUPPORTED)
}

/// How an open file is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum FileMode {
    Source,
    Preview,
}

impl FileMode {
    /// Files with a preview on this platform open as Preview, everything else as Source.
    pub(super) fn default_for(path: &Path) -> Self {
        if can_preview(path) {
            Self::Preview
        } else {
            Self::Source
        }
    }
}

/// One editor instance of a file: the Files panel, or a canvas file pane. Each keeps its own
/// mode and draft, so a panel tab and a card of the same file never change each other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum FileHost {
    Panel,
    Card { pane: String },
}

impl FileHost {
    pub(super) fn slot(&self) -> String {
        match self {
            Self::Panel => "canvas-file".into(),
            Self::Card { pane } => format!("canvas-file:{pane}"),
        }
    }
}

/// HTML has its live preview only after the file host has become a browser pane.
pub(super) fn file_host_mode(path: &str, mode: FileMode) -> FileMode {
    if previewable(Path::new(path)) == Some(Previewable::Html) {
        FileMode::Source
    } else {
        mode
    }
}

/// A card viewer gets its pane id, so its dirty reports name the editor they come from.
pub(super) fn viewer_slot(path: &str, host: FileHost) -> (String, Instance) {
    let path = Path::new(path);
    let pane = match &host {
        FileHost::Panel => None,
        FileHost::Card { pane } => Some(pane.clone()),
    };
    (
        host.slot(),
        Instance {
            name: "files".into(),
            props: json!({"canvas":"viewer","editing":previewable(path) != Some(Previewable::Jsonl),
            "root":path.parent(),"file":path.file_name().map(|name| name.to_string_lossy()),"pane":pane}),
        },
    )
}

pub(super) fn mode_action(path: &str, current: FileMode, pane: Option<&str>) -> BarAction {
    let (mode, icon, label) = match current {
        FileMode::Source => (FileMode::Preview, "icons/eye.svg", "Preview"),
        FileMode::Preview => (FileMode::Source, "icons/code.svg", "Source"),
    };
    let mut args = json!({"path":path,"mode":mode});
    if let Some(pane) = pane {
        args["pane"] = json!(pane);
    }
    BarAction {
        icon: icon.into(),
        label: label.into(),
        command: "file.mode".into(),
        args,
    }
}

#[derive(serde::Deserialize)]
struct ModeArgs {
    path: String,
    mode: FileMode,
    pane: Option<String>,
}

impl AppWindow {
    pub(in crate::window) fn file_mode_command(
        &mut self,
        args: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(ModeArgs { path, mode, pane }) = serde_json::from_value(args.clone()) else {
            return self.fail("file.mode requires a path and a mode of source or preview");
        };
        // A toast, not just an answer: the person clicked a button and must see why nothing changed.
        if let Err(error) = self.set_file_mode(&path, mode, pane.as_deref(), window, cx) {
            self.command_error("file.mode", &error, window, cx);
            return self.fail(error);
        }
    }

    pub(super) fn html_panel_tab(&self, path: &str) -> Option<canvas::PanelTab> {
        let doc = &self.canvas_state.doc;
        doc.panel.tabs.iter().find(|tab| tab.pane().is_some_and(|id| {
            doc.panes.iter().any(|pane| pane.id == *id
                && matches!(&pane.kind, canvas::Kind::Browser { source: Some(source), .. } if source == path))
        })).cloned()
    }

    pub(super) fn set_file_mode(
        &mut self,
        path: &str,
        mode: FileMode,
        pane: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if !Path::new(path).is_absolute() {
            return Err("File path must be absolute".into());
        }
        let kind = previewable(Path::new(path));
        if mode == FileMode::Preview && !can_preview(Path::new(path)) {
            return Err(format!("No preview available for {path}"));
        }
        let file_tab = canvas::PanelTab::File(path.into());
        let panel_browser = if pane.is_none() {
            self.html_panel_tab(path)
        } else {
            None
        };
        let doc = &self.canvas_state.doc;
        let target = pane.or(panel_browser
            .as_ref()
            .and_then(canvas::PanelTab::pane)
            .map(String::as_str));
        if let Some(id) = target {
            if !doc.panes.iter().any(|p| p.id == id && (p.file() == Some(path)
                || matches!(&p.kind, canvas::Kind::Browser { source: Some(source), .. } if source == path))) {
                return Err(format!("No file pane {id} for {path}"));
            }
        } else if !doc.panel.tabs.contains(&file_tab) {
            return Err(format!("No file tab for {path}"));
        }
        // A docked HTML preview is the panel's editor, not a card's.
        let host = match target {
            Some(id) if !doc.panel.tabs.contains(&canvas::PanelTab::Pane(id.into())) => FileHost::Card { pane: id.into() },
            _ => FileHost::Panel,
        };
        let drafted = match &host {
            FileHost::Panel => self.canvas_state.dirty.as_deref() == Some(path),
            FileHost::Card { pane } => self.canvas_state.card_drafts.contains(pane),
        };
        if mode == FileMode::Preview && drafted {
            return Err("Save or discard your changes before opening Preview".into());
        }

        if kind == Some(Previewable::Html) {
            match mode {
                FileMode::Preview => {
                    let url = browser::browser_target_url(path)?;
                    if let Some(id) = target {
                        let browser = Self::browser_kind(url, Some(path.into()));
                        let pane = self
                            .canvas_state
                            .doc
                            .panes
                            .iter_mut()
                            .find(|p| p.id == id)
                            .unwrap();
                        if pane.file().is_some() {
                            pane.kind = browser;
                        }
                    } else {
                        let point = self.view_center(window);
                        let space = self.canvas_state.doc.room.clone();
                        // Allocate a regular browser pane and keep the file's position in the tab strip.
                        let id = self.add_browser_pane(url, point, space, Some(path.into()), cx);
                        let state = &mut self.canvas_state;
                        let tab = canvas::PanelTab::Pane(id);
                        *state
                            .doc
                            .panel
                            .tabs
                            .iter_mut()
                            .find(|tab| **tab == file_tab)
                            .unwrap() = tab.clone();
                        if state.file.as_ref() == Some(&file_tab) {
                            state.file = Some(tab);
                        }
                    }
                    self.canvas_changed(cx);
                }
                FileMode::Source => {
                    if let Some(id) = target {
                        if self.canvas_state.browser_tab.as_deref() == Some(id) {
                            self.collapse_browser(window, cx);
                        }
                        let tab = canvas::PanelTab::Pane(id.into());
                        let state = &mut self.canvas_state;
                        if let Some(slot) =
                            state.doc.panel.tabs.iter_mut().find(|slot| **slot == tab)
                        {
                            *slot = file_tab.clone();
                            if state.file.as_ref() == Some(&tab) {
                                state.file = Some(file_tab);
                            }
                            self.close_boxed_panes(&[id.into()], cx);
                        } else {
                            state
                                .doc
                                .panes
                                .iter_mut()
                                .find(|p| p.id == id)
                                .unwrap()
                                .kind = canvas::Kind::File { path: path.into() };
                        }
                        self.canvas_changed(cx);
                    }
                }
            }
        }
        self.canvas_state.modes.insert((host, path.into()), mode);
        self.load_previews(None, window, cx);
        self.sync(window, cx);
        cx.notify();
        Ok(())
    }

    /// The body of an open file, the same in the Files panel and on a canvas file card.
    pub(super) fn file_body(
        &self,
        path: &str,
        host: FileHost,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel = host == FileHost::Panel;
        let body = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .overflow_hidden();
        let kind = previewable(Path::new(path));
        if kind.is_none() || self.canvas_state.shown_mode(&host, path) == FileMode::Source {
            return body
                .child(self.render_slot(&host.slot(), cx))
                .into_any_element();
        }
        let (x, y): (gpui::DefiniteLength, gpui::DefiniteLength) = if panel {
            (px(24.).into(), px(20.).into())
        } else {
            (wpx(canophy::traits::FILE_PAD_X).into(), wpx(12.).into())
        };
        if kind == Some(Previewable::Svg) {
            return body
                .px(x)
                .py(y)
                .child(
                    img(std::path::PathBuf::from(path))
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element();
        }
        let Some((view, _)) = self.canvas_state.previews.get(path) else {
            return body
                .px(x)
                .py(y)
                .text_color(c("muted-foreground"))
                .child("Loading file")
                .into_any_element();
        };
        let pane = match host {
            FileHost::Panel => None,
            FileHost::Card { pane } => Some(pane),
        };
        let app = cx.weak_entity();
        let path = path.to_owned();
        let link_path = path.clone();
        let quote_view = view.clone();
        body.child(
            gpui_base::TextView::new(view)
                .scrollable(true)
                .px(x)
                .py(y)
                .image_source(|_| {
                    static BLANK: std::sync::LazyLock<Arc<gpui::Image>> =
                        std::sync::LazyLock::new(|| {
                            Arc::new(gpui::Image::from_bytes(
                                gpui::ImageFormat::Svg,
                                b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'/>"
                                    .to_vec(),
                            ))
                        });
                    gpui::ImageSource::Image(BLANK.clone())
                })
                .on_link_click(move |url, _, window, cx| {
                    let _ = app.update(cx, |this, cx| {
                        if url.starts_with("https://") || url.starts_with("http://") {
                            this.run(
                                "canvas.pane_new",
                                json!({"kind":"browser","url":url.to_string()}),
                                window,
                                cx,
                            );
                            return;
                        }
                        let target = url.split('#').next().unwrap_or_default();
                        if target.is_empty() {
                            return;
                        }
                        let target = Path::new(&link_path)
                            .parent()
                            .unwrap_or(Path::new("/"))
                            .join(target);
                        if target.exists() {
                            this.run("preview.open", json!({"path":target}), window, cx);
                        } else {
                            this.command_error(
                                "preview.open",
                                &format!("File not found: {}", target.display()),
                                window,
                                cx,
                            );
                        }
                    });
                }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, e: &MouseUpEvent, window, cx| {
                this.quote_selection(&path, pane.as_deref(), &quote_view, e.position, window, cx);
            }),
        )
        .into_any_element()
    }

    pub(super) fn file_outline(&self, path: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        let view = &self.canvas_state.previews.get(path)?.0;
        let rendered = view.read(cx).rendered_text();
        let source = rendered.source();
        let mut rail = div()
            .id("file-outline")
            .w(px(150.))
            .flex_none()
            .overflow_y_scroll()
            .border_r_1()
            .border_color(c("border"))
            .py(px(8.));
        let mut offsets = source
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some(start..start + line.trim_end_matches(['\r', '\n']).len())
            })
            .enumerate();
        for (level, title, line) in headings(source) {
            let Some((_, source_range)) = offsets.find(|(index, _)| *index == line) else {
                break;
            };

            let Some(range) = rendered.range_for_source(source_range) else {
                continue;
            };
            let view = view.clone();
            rail = rail.child(
                div()
                    .id(("file-heading", line))
                    .pl(px(8. + 10. * f32::from(level - 1)))
                    .pr(px(8.))
                    .py(px(4.))
                    .text_size(px(12.))
                    .truncate()
                    .cursor_pointer()
                    .hover(|d| d.bg(c("hover")))
                    .child(title)
                    .on_click(move |_, _, cx| {
                        view.update(cx, |view, cx| {
                            let _ = view.reveal_range(range.clone(), cx);
                        });
                    }),
            );
        }
        Some(rail.into_any_element())
    }
}

fn headings(text: &str) -> Vec<(u8, String, usize)> {
    let mut result = Vec::new();
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() <= 3
            && let Some(marker @ (b'`' | b'~')) = trimmed.bytes().next()
        {
            let count = trimmed.bytes().take_while(|&b| b == marker).count();
            if let Some((open, width)) = fence {
                if open == marker && count >= width && trimmed[count..].trim().is_empty() {
                    fence = None;
                }
            } else if count >= 3 && (marker != b'`' || !trimmed[count..].contains('`')) {
                fence = Some((marker, count));
            }
        }
        if fence.is_some() {
            continue;
        }
        let level = line.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&level)
            && let Some(title) = line[level..].strip_prefix(' ')
        {
            result.push((level as u8, title.trim().to_owned(), index));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{FileMode, Path, Previewable, browser, headings, previewable};

    #[test]
    fn extensions_and_default_modes() {
        for (ext, expected) in [
            ("md", Previewable::Markdown),
            ("markdown", Previewable::Markdown),
            ("mdx", Previewable::Markdown),
            ("jsonl", Previewable::Jsonl),
            ("svg", Previewable::Svg),
        ] {
            for ext in [ext.to_owned(), ext.to_ascii_uppercase()] {
                let path = Path::new("doc").with_extension(ext);
                assert_eq!(previewable(&path), Some(expected));
                assert_eq!(FileMode::default_for(&path), FileMode::Preview);
            }
        }
        for ext in ["html", "HTML", "htm", "HTM"] {
            let path = Path::new("doc").with_extension(ext);
            assert_eq!(previewable(&path), Some(Previewable::Html));
            assert_eq!(FileMode::default_for(&path), if browser::LOCAL_FILES_SUPPORTED { FileMode::Preview } else { FileMode::Source });
        }
        for path in [Path::new("doc"), Path::new("doc.txt")] {
            assert_eq!(previewable(path), None);
            assert_eq!(FileMode::default_for(path), FileMode::Source);
        }
    }

    #[test]
    fn outline_skips_fences_and_hashtags() {
        let text = "# Top\n#hashtag\n````md\n# Hidden\n```\n# Still hidden\n````\n~~~\n# Hidden\n~~~\n###### Final";
        assert_eq!(
            headings(text),
            vec![(1, "Top".into(), 0), (6, "Final".into(), 10)]
        );
    }
}
