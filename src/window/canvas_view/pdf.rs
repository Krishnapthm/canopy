//! PDF pane rendering, highlights and page text. See docs/wiki/Images-and-PDFs.md.
use super::*;
use gpui_pdf::{FitMode, Highlight, PdfEvent, PdfStyle, PdfView};
use std::{io::Read, path::Path, rc::Rc};

/// Checks the signature without opening the PDF for writing.
pub(super) fn has_pdf_header(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut header = [0; 5];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok()
        && &header == b"%PDF-"
}

pub(super) type ParsedPdf = (Arc<Vec<u8>>, Arc<gpui_pdf::Document>);

pub(super) fn parse_pdf(path: &str) -> Result<ParsedPdf, String> {
    if !Path::new(path).is_file() {
        return Err(format!("cannot read {path}: not a file"));
    }
    // shortcut: parse for sizing and reads on the canvas thread, move it off-thread if large PDFs stall the canvas.
    let bytes = Arc::new(std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?);
    let doc = gpui_pdf::parse(bytes.clone()).map_err(|e| match e {
        gpui_pdf::LoadError::Locked => format!("cannot read {path}: the PDF is password protected"),
        gpui_pdf::LoadError::Other(e) => format!("cannot read {path}: {e}"),
    })?;
    Ok((bytes, doc))
}

/// The first page's size at the viewer's base width, with fit-width chrome.
pub(super) fn natural_size(doc: &gpui_pdf::Document) -> Option<canophy::Natural> {
    let &(w, h) = gpui_pdf::page_dims(doc).first()?;
    let width = gpui_pdf::PAGE_WIDTH;
    let height = width * (h / w);
    if !(w.is_finite() && w > 0. && height.is_finite() && height > 0.) {
        return None;
    }
    let (chrome_w, chrome_h) = gpui_pdf::FIT_CHROME;
    Some(canophy::Natural {
        content: canophy::WorldSize::new(width, height),
        chrome: canophy::WorldSize::new(chrome_w, chrome_h),
    })
}

/// Returns the PDF's page text in a pane read envelope.
pub(super) fn read_pdf(id: &str, path: &str) -> Result<String, String> {
    let (_, doc) = parse_pdf(path)?;
    let pages = doc.pages().len();
    let mut text = String::new();
    for page in 0..pages {
        if page > 0 {
            text.push_str("\n\n");
        }
        let page_text =
            gpui_pdf::extract_page_text(&doc, page).map_or_else(String::new, |p| p.text());
        text.push_str(&format!("[page {}]\n{page_text}", page + 1));
        if text.len() > canvas::READ_LIMIT {
            break;
        }
    }
    Ok(canvas::envelope(
        id,
        "pdf",
        &[("path", path), ("pages", &pages.to_string())],
        &text,
    ))
}

fn view_marks(marks: &[canvas::PdfMark]) -> Vec<Highlight> {
    marks
        .iter()
        .enumerate()
        .filter_map(|(id, mark)| {
            Some(Highlight {
                id: id as u64,
                page: mark.page.checked_sub(1)?,
                quote: mark.quote.clone(),
                occurrence: mark.occurrence,
                color: hsla(50. / 360., 0.95, 0.55, 1.),
                region: None,
            })
        })
        .collect()
}

impl AppWindow {
    /// Keeps one viewer per saved PDF pane.
    pub(super) fn sync_pdf_views(
        &mut self,
        mut parsed: Option<(&str, ParsedPdf)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prune_pdf_views();
        for pane in &self.canvas_state.doc.panes {
            let canvas::Kind::Pdf { path, highlights } = &pane.kind else {
                continue;
            };
            if self.canvas_state.pdfs.contains_key(&pane.id) {
                continue;
            }

            let style = Rc::new(|| PdfStyle {
                bg: c("surface"),
                border: c("border"),
                placeholder_bg: c("hover"),
                placeholder_fg: c("muted-foreground"),
                header_fg: c("foreground"),
                header_muted: c("muted-foreground"),
            });
            let parsed = if parsed.as_ref().is_some_and(|(id, _)| *id == pane.id) {
                parsed.take().map(|(_, pdf)| pdf)
            } else {
                None
            };
            let path = std::path::PathBuf::from(path);
            let view = cx.new(|cx| {
                PdfView::with_parsed_document(path.clone(), style, Rc::new(|| 1.), parsed, cx)
            });
            let weak = cx.weak_entity();
            let id = pane.id.clone();
            view.update(cx, |view, cx| {
                view.set_show_header(false, cx);
                // Panes rest fit to width: the helper bar's reset zoom and Cmd+0 come back to it.
                view.set_home_fit(FitMode::Width, cx);
                view.set_highlights(view_marks(highlights), cx);
                let (owner, pane) = (weak.clone(), id.clone());
                view.set_on_create_highlight(Rc::new(
                    move |page, quote, occurrence, _, window, app| {
                        let (owner, pane, at) =
                            (owner.clone(), pane.clone(), window.mouse_position());
                        // The callback runs while PdfView is borrowed.
                        app.defer(move |app| {
                            let _ = owner.update(app, |this, cx| {
                                this.add_pdf_mark(
                                    &pane,
                                    canvas::PdfMark {
                                        page: page + 1,
                                        quote,
                                        occurrence,
                                    },
                                    at,
                                    cx,
                                );
                            });
                        });
                    },
                ));
                view.set_on_highlight(Rc::new(move |mark, window, app| {
                    let (owner, pane, at) = (weak.clone(), id.clone(), window.mouse_position());
                    app.defer(move |app| {
                        let _ = owner.update(app, |this, cx| {
                            let Some(card) =
                                this.canvas_state.doc.panes.iter().find(|p| p.id == pane)
                            else {
                                return;
                            };
                            let canvas::Kind::Pdf { path, highlights } = &card.kind else {
                                return;
                            };
                            let Some(mark) = highlights.get(mark as usize) else {
                                return;
                            };
                            this.canvas_state.quote_chip =
                                Some((pdf_quote(path, &pane, mark, at), None));
                            cx.notify();
                        });
                    });
                }));
                view.set_on_open_external(Rc::new(move |_, _| {
                    crate::links::open(&crate::links::Target::File(path.clone()));
                }));
            });
            cx.subscribe_in(&view, window, |this, view, event: &PdfEvent, window, cx| {
                let view = view.read(cx);
                let error = match event {
                    PdfEvent::LoadFailed => view.load_error().map(|e| e.to_string()),
                    PdfEvent::LockChanged if view.is_locked() => {
                        Some("The PDF is password protected".into())
                    }
                    _ => None,
                };
                if let Some(title) = error {
                    this.open_overlay(
                        mods::new_overlay_id(),
                        json!({"kind":"toast","notification":{"title":title,"level":"error"}}),
                        window,
                        cx,
                    );
                }
            })
            .detach();
            self.canvas_state.pdfs.insert(pane.id.clone(), view);
        }
    }

    /// Queues removed viewers to release their page textures on the window.
    pub(super) fn prune_pdf_views(&mut self) {
        let state = &mut self.canvas_state;
        state.pdfs.retain(|id, view| {
            if state
                .doc
                .panes
                .iter()
                .any(|p| p.id == *id && matches!(p.kind, canvas::Kind::Pdf { .. }))
            {
                return true;
            }
            state.pdf_drops.push(view.clone());
            false
        });
    }

    /// Releases PDFs removed since the previous frame.
    pub(in crate::window) fn release_pdf_drops(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for view in self.canvas_state.pdf_drops.drain(..) {
            view.update(cx, |view, cx| view.release(window, cx));
        }
    }

    /// Releases every page texture before the window goes away.
    pub(in crate::window) fn release_pdfs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.canvas_state
            .pdf_drops
            .extend(self.canvas_state.pdfs.drain().map(|(_, view)| view));
        self.release_pdf_drops(window, cx);
    }

    /// Draws the PDF viewer inside its saved pane frame.
    pub(super) fn pdf_card(
        &self,
        card: &canvas::Pane,
        path: &str,
        camera: canophy::Camera,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let view = self.canvas_state.pdfs.get(&card.id)?;
        let path = Path::new(path);
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let detail = path.parent().unwrap_or(path).to_string_lossy().into_owned();
        // `sync_panes` set the page scale to the pane's.
        let body = div()
            .size_full()
            .child(view.clone())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        Some(self.pane_card(
            card,
            "icons/file-text.svg".into(),
            title,
            detail,
            None,
            body.into_any_element(),
            camera,
            cx,
        ))
    }

    /// Saves a selection as a highlight and offers it as a quote.
    pub(super) fn add_pdf_mark(
        &mut self,
        id: &str,
        mark: canvas::PdfMark,
        at: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        let canvas::Kind::Pdf { path, highlights } = &mut card.kind else {
            return;
        };
        self.canvas_state.quote_chip = Some((pdf_quote(path, id, &mark, at), None));
        highlights.push(mark);
        let marks = view_marks(highlights);
        self.canvas_changed(cx);
        if let Some(view) = self.canvas_state.pdfs.get(id) {
            view.update(cx, |view, cx| view.set_highlights(marks, cx));
        }
        cx.notify();
    }

    /// Runs a PDF helper action on the last selected pane.
    pub(in crate::window) fn pdf_command(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self
            .canvas_state
            .selected
            .last()
            .and_then(|id| self.canvas_state.doc.panes.iter_mut().find(|p| p.id == *id))
        else {
            return self.fail("Select a PDF pane first");
        };
        let canvas::Kind::Pdf { highlights, .. } = &mut card.kind else {
            return self.fail("Select a PDF pane first");
        };
        let view = self.canvas_state.pdfs.get(&card.id).cloned();
        if name == "canvas.pdf_clear" {
            let count = highlights.len();
            if count > 0 {
                highlights.clear();
                self.canvas_changed(cx);
                if let Some(view) = view {
                    view.update(cx, |view, cx| view.set_highlights(vec![], cx));
                }
                cx.notify();
            }
            self.open_overlay(mods::new_overlay_id(), json!({"kind":"toast","notification":{"title":format!("Cleared {count} highlight{}", if count == 1 { "" } else { "s" })}}), window, cx);
        } else if let Some(view) = view {
            view.update(cx, |view, cx| match name {
                "canvas.pdf_find" => view.toggle_search(cx),
                "canvas.pdf_highlight" => view.toggle_select_mode(cx),
                "canvas.pdf_prev" => view.prev_page(cx),
                "canvas.pdf_next" => view.next_page(cx),
                "canvas.pdf_zoom_out" => view.zoom_out(cx),
                "canvas.pdf_zoom_reset" => view.reset_zoom(cx),
                "canvas.pdf_zoom_in" => view.zoom_in(cx),
                _ => unreachable!(),
            });
        } else {
            self.fail("The PDF pane is still loading");
        }
    }
}

fn pdf_quote(path: &str, id: &str, mark: &canvas::PdfMark, at: Point<Pixels>) -> Value {
    json!({"kind":"pdf","path":path,"pane":id,"page":mark.page,"text":mark.quote,
        "point":{"x":f32::from(at.x),"y":f32::from(at.y)}})
}
