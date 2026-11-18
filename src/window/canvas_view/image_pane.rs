//! Image pane rendering and region quotes. See docs/panes.md.
use super::*;
use gpui::{ObjectFit, StyledImage, img};
use std::path::Path;

fn pane_size((w, h): (u32, u32)) -> canvas::Point {
    let (w, h) = (w as f32, h as f32);
    let scale = (560. / w).min(420. / h).min(1.);
    [
        (w * scale).max(160.),
        (h * scale).max(160.) + canvas::HEADER,
    ]
}

fn image_rect(
    body: Bounds<Pixels>,
    (iw, ih): (u32, u32),
    a: Point<Pixels>,
    b: Point<Pixels>,
) -> Option<[u32; 4]> {
    let (bw, bh) = (f32::from(body.size.width), f32::from(body.size.height));
    let scale = (bw / iw as f32).min(bh / ih as f32);
    if !scale.is_finite() || scale <= 0. || iw == 0 || ih == 0 {
        return None;
    }
    let axis = |a: Pixels, b: Pixels, origin: Pixels, displayed: f32, pixels: u32| {
        let start = f32::from(origin) + (displayed - pixels as f32 * scale) / 2.;
        let at = |p: Pixels| ((f32::from(p) - start) / scale).clamp(0., pixels as f32);
        let (lo, hi) = (at(a.min(b)), at(a.max(b)));
        (
            (lo.round() as u32).min(pixels),
            (hi - lo).round() as u32,
            hi - lo >= 4.,
        )
    };
    let (x, w, wide) = axis(a.x, b.x, body.origin.x, bw, iw);
    let (y, h, tall) = axis(a.y, b.y, body.origin.y, bh, ih);
    (wide && tall).then_some([x, y, w.min(iw - x), h.min(ih - y)])
}

fn save_region(source: &Path, [x, y, w, h]: [u32; 4], destination: &Path) -> anyhow::Result<()> {
    let image = image::ImageReader::open(source)?
        .with_guessed_format()?
        .decode()?;
    anyhow::ensure!(
        x + w <= image.width() && y + h <= image.height(),
        "the image size changed"
    );
    std::fs::create_dir_all(destination.parent().unwrap())?;
    crate::control::save_png(&image.crop_imm(x, y, w, h).into_rgba8(), destination)
}

/// An image's pixel size, read from its bytes (the extension may lie).
fn image_size(path: &Path) -> image::ImageResult<(u32, u32)> {
    image::ImageReader::open(path)?.with_guessed_format()?.into_dimensions()
}

fn bounds_key(id: &str) -> String {
    format!("pane-image-{id}")
}

impl AppWindow {
    /// Caches the source dimensions and returns the initial image pane size.
    pub(super) fn open_image(
        &mut self,
        path: &Path,
        canonical: &str,
    ) -> image::ImageResult<canvas::Point> {
        if let Some(&size) = self.canvas_state.image_sizes.get(canonical) {
            return Ok(pane_size(size));
        }
        let size = image_size(path)?;
        self.canvas_state.image_sizes.insert(canonical.into(), size);
        self.canvas_state.image_errors.remove(canonical);
        Ok(pane_size(size))
    }

    /// Keeps image dimensions and body bounds only while their panes are open.
    pub(super) fn prune_images(&mut self, ids: &[String]) {
        let state = &mut self.canvas_state;
        let open =
            |path: &String| {
                state.doc.panes.iter().any(
                    |p| matches!(&p.kind, canvas::Kind::Image { path: source } if source == path),
                )
            };
        state.image_sizes.retain(|path, _| open(path));
        state.image_errors.retain(open);
        if let Some((id, _, _)) = &state.region
            && ids.contains(id)
        {
            state.region = None;
        }
        let mut bounds = self.bounds.borrow_mut();
        for id in ids {
            bounds.remove(&bounds_key(id));
        }
    }

    /// Measures restored images once and reports unavailable sources.
    pub(super) fn sync_image_sizes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.canvas_state;
        let mut errors = Vec::new();
        for pane in &state.doc.panes {
            let canvas::Kind::Image { path } = &pane.kind else {
                continue;
            };
            if state.image_sizes.contains_key(path) || state.image_errors.contains(path) {
                continue;
            }
            match image_size(Path::new(path)) {
                Ok(size) => {
                    state.image_sizes.insert(path.clone(), size);
                }
                Err(_) => {
                    state.image_errors.insert(path.clone());
                    let name = Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    errors.push(format!("Can't open {name}: not a readable image"));
                }
            }
        }
        for title in errors {
            self.open_overlay(
                mods::new_overlay_id(),
                json!({"kind":"toast","notification":{"title":title,"level":"error"}}),
                window,
                cx,
            );
        }
    }

    /// Draws an image in its saved pane frame and measures the region-drag surface.
    pub(super) fn image_card(
        &self,
        card: &canvas::Pane,
        path: &str,
        camera: canvas::Camera,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ready = self.canvas_state.image_sizes.contains_key(path);
        let path = Path::new(path);
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let detail = path.parent().unwrap_or(path).to_string_lossy().into_owned();
        let id = card.id.clone();
        let key = bounds_key(&id);
        let maps = self.bounds.clone();
        let selection = self
            .canvas_state
            .region
            .as_ref()
            .filter(|(pane, _, _)| pane == &id)
            .zip(maps.borrow().get(&key).copied())
            .map(|((_, a, b), body)| {
                let bounds = Bounds::from_corners(
                    point(a.x.min(b.x), a.y.min(b.y)),
                    point(a.x.max(b.x), a.y.max(b.y)),
                )
                .intersect(&body);
                div()
                    .absolute()
                    .left(bounds.origin.x - body.origin.x)
                    .top(bounds.origin.y - body.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .border_1()
                    .border_color(c("foreground"))
                    .bg(c("hover"))
            });
        let body = div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(c("surface"))
            .when(ready, |body| {
                body.child(
                    img(path.to_owned())
                        .size_full()
                        .object_fit(ObjectFit::Contain)
                        .with_fallback(|| {
                            div()
                                .p_3()
                                .child("Can't display this image")
                                .into_any_element()
                        }),
                )
            })
            .when(!ready, |body| {
                body.child(div().p_3().child("Image unavailable"))
            })
            .child(
                canvas(
                    move |b, _, _| {
                        maps.borrow_mut().insert(key, b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .children(selection)
            .when(ready, |body| {
                body.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        this.focus.focus(window, cx);
                        this.canvas_state.selected = vec![id.clone()];
                        this.canvas_state.region =
                            Some((id.clone(), event.position, event.position));
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
            });
        self.pane_card(
            card,
            "icons/image.svg".into(),
            title,
            detail,
            body.into_any_element(),
            camera,
            cx,
        )
    }

    /// Tracks region drags through mouse-up, even when the pointer leaves the pane.
    pub(in crate::window) fn image_region_tracker(&self, cx: &Context<Self>) -> Option<AnyElement> {
        if self.canvas_state.image_sizes.is_empty() || !self.has_canvas() {
            return None;
        }
        let weak = cx.weak_entity();
        Some(
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    let view = weak.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                        if phase != DispatchPhase::Capture
                            || !view
                                .upgrade()
                                .is_some_and(|view| view.read(cx).canvas_state.region.is_some())
                        {
                            return;
                        }
                        let _ = view.update(cx, |this, cx| {
                            let Some((_, _, current)) = this.canvas_state.region.as_mut() else {
                                return;
                            };
                            if event.pressed_button == Some(MouseButton::Left) {
                                *current = event.position;
                            } else {
                                this.canvas_state.region = None;
                            }
                            cx.notify();
                        });
                    });
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            let _ = weak.update(cx, |this, cx| {
                                this.finish_image_region(event.position, window, cx)
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_full()
            .into_any_element(),
        )
    }

    fn finish_image_region(
        &mut self,
        end: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((id, start, _)) = self.canvas_state.region.take() else {
            return;
        };
        cx.notify();
        let Some(path) = self
            .canvas_state
            .doc
            .panes
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.path())
        else {
            return;
        };
        let Some(bounds) = self.bounds.borrow().get(&bounds_key(&id)).copied() else {
            return;
        };
        let Some(&dimensions) = self.canvas_state.image_sizes.get(path) else {
            return;
        };
        let Some(region @ [x, y, w, h]) = image_rect(bounds, dimensions, start, end) else {
            return;
        };
        let filename_id = id.replace(['/', '\\', ':'], "-");
        let crop = std::env::temp_dir()
            .join("canopy")
            .join(format!("{filename_id}-region-{x}-{y}-{w}x{h}.png"));
        // shortcut: crop and encode on mouse-up, move it off-thread if large crops stall the canvas.
        match save_region(Path::new(path), region, &crop) {
            Ok(()) => {
                let quote = json!({"kind":"image-region","path":path,"pane":id,"text":format!("region {x},{y} {w}x{h}"),
                    "region":region,"crop":crop,"point":{"x":f32::from(end.x),"y":f32::from(end.y)}});
                self.canvas_state.quote_chip = Some((quote, None));
            }
            Err(error) => self.open_overlay(
                mods::new_overlay_id(),
                json!({"kind":"toast","notification":{
                    "title":format!("Can't quote region: {error}"),"level":"error"
                }}),
                window,
                cx,
            ),
        }
    }

    /// Opens the quote menu for the last selected image pane.
    pub(in crate::window) fn quote_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(card) = self
            .canvas_state
            .selected
            .last()
            .and_then(|id| self.canvas_state.doc.panes.iter().find(|p| p.id == *id))
        else {
            return self.fail("Select an image pane first");
        };
        let canvas::Kind::Image { path } = &card.kind else {
            return self.fail("Select an image pane first");
        };
        let text = Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let at = window.mouse_position();
        let quote = json!({"kind":"image","path":path,"pane":card.id,"text":text,
            "point":{"x":f32::from(at.x),"y":f32::from(at.y)}});
        self.clear_quote_mark(cx);
        self.canvas_state.quote_chip = None;
        self.quote_menu(&quote, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{Bounds, image_rect, point, px, save_region, size};

    #[test]
    fn image_rect_clamps_contained_images() {
        let pt = |x, y| point(px(x), px(y));
        let bounds = Bounds::new(pt(10., 20.), size(px(100.), px(100.)));
        assert_eq!(
            image_rect(bounds, (100, 50), pt(0., 38.), pt(60., 70.)),
            Some([0, 0, 50, 25])
        );
        assert_eq!(
            image_rect(bounds, (50, 100), pt(80., 80.), pt(40., 30.)),
            Some([5, 10, 40, 50])
        );
        assert_eq!(
            image_rect(bounds, (50, 100), pt(40., 30.), pt(43., 50.)),
            None
        );
    }

    #[test]
    fn image_crop_writes_requested_pixels() {
        let dir = std::env::temp_dir().join(format!("canopy-image-crop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.png");
        let destination = dir.join("crop.png");
        image::RgbaImage::new(10, 6).save(&source).unwrap();
        save_region(&source, [2, 1, 4, 3], &destination).unwrap();
        assert_eq!(image::image_dimensions(destination).unwrap(), (4, 3));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
