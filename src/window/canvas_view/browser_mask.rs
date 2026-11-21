//! Clips live browser views to the canvas and lets foreground GPUI elements receive input.
#[cfg(target_os = "macos")]
use super::*;

/// A screen-space rectangle in logical pixels: x, y, width, height.
#[cfg(any(target_os = "macos", test))]
pub(super) type R = [f32; 4];

#[cfg(any(target_os = "macos", test))]
fn intersection([x, y, w, h]: R, [a, b, aw, bh]: R) -> Option<R> {
    let (left, top) = (x.max(a), y.max(b));
    let (right, bottom) = ((x + w).min(a + aw), (y + h).min(b + bh));
    (left < right && top < bottom).then_some([left, top, right - left, bottom - top])
}

#[cfg(any(target_os = "macos", test))]
fn subtract(rect: R, hole: R) -> impl Iterator<Item = R> {
    let [x, y, w, h] = rect;
    match intersection(rect, hole) {
        Some([a, b, aw, bh]) => [
            [x, y, w, b - y],
            [x, b + bh, w, y + h - (b + bh)],
            [x, b, a - x, bh],
            [a + aw, b, x + w - (a + aw), bh],
        ],
        None => [rect, [0.; 4], [0.; 4], [0.; 4]],
    }
    .into_iter()
    .filter(|r| r[2] > 0. && r[3] > 0.)
}

/// Foreground rectangles and the parts of the page outside the canvas, clipped to the page.
#[cfg(any(target_os = "macos", test))]
pub(super) fn covers(page: R, canvas: R, above: &[R]) -> Vec<R> {
    above
        .iter()
        .filter_map(|&r| intersection(page, r))
        .chain(subtract(page, canvas))
        .collect()
}

/// The disjoint parts of the page outside the union of the holes.
#[cfg(any(target_os = "macos", test))]
pub(super) fn visible(page: R, holes: &[R]) -> Vec<R> {
    let mut pieces: Vec<_> = [page]
        .into_iter()
        .filter(|r| r[2] >= 0.5 && r[3] >= 0.5)
        .collect();
    for &hole in holes {
        pieces = pieces
            .into_iter()
            .flat_map(|rect| subtract(rect, hole))
            .filter(|r| r[2] >= 0.5 && r[3] >= 0.5)
            .collect();
    }
    pieces
}

#[cfg(target_os = "macos")]
pub(super) fn screen_rect(pane: &canvas::Pane, rect: R, camera: canvas::Camera) -> R {
    // Document::pin puts the clamped screen origin into the world rect used to draw the pane.
    let [x, y] = camera.to_screen([rect[0], rect[1]]);
    let [w, h] = pane.pinned.map_or(
        [rect[2] * camera.zoom, rect[3] * camera.zoom],
        |[_, _, w, h]| [w, h],
    );
    [x, y, w, h]
}

#[cfg(target_os = "macos")]
fn rect(bounds: Bounds<Pixels>) -> R {
    [
        bounds.origin.x.into(),
        bounds.origin.y.into(),
        bounds.size.width.into(),
        bounds.size.height.into(),
    ]
}

#[cfg(target_os = "macos")]
impl AppWindow {
    /// Foreground pane rectangles relative to the canvas origin, which prepaint measures.
    fn page_covers(&self, id: &str) -> Vec<R> {
        let state = &self.canvas_state;
        let doc = state.shown.as_ref().unwrap_or(&state.doc);
        let Some(page) = doc.panes.iter().find(|p| p.id == id) else {
            return vec![];
        };
        let camera = state.doc.camera;

        // shortcut: GPUI tooltips lack cover bounds, measure them if they need to overlap live pages.
        // shortcut: rectangular holes leave square corners, use pane-shaped paths if the corners matter.
        let [x, y, w, h] = screen_rect(page, page.rect, camera);
        let corner = PANE_RESIZE_CORNER * camera.zoom;
        let grip = [x + w - 1. - corner, y + h - 1. - corner, corner, corner];
        std::iter::once(grip)
            .chain(
                doc.panes
                    .iter()
                    .filter(|p| doc.shows(p.space.as_ref()) && paint_order(p) > paint_order(page))
                    .filter_map(|p| {
                        match &p.kind {
                            canvas::Kind::File { path } if !state.previews.contains_key(path) => {
                                return None;
                            }
                            canvas::Kind::Pdf { .. } if !state.pdfs.contains_key(&p.id) => {
                                return None;
                            }
                            canvas::Kind::Sub { parent, sub }
                                if !state
                                    .subs
                                    .get(parent)
                                    .is_some_and(|subs| subs.iter().any(|s| &s.id == sub)) =>
                            {
                                return None;
                            }
                            _ => {}
                        }
                        let world = if p.boxed() {
                            p.rect
                        } else {
                            self.wire_cell(doc, &p.id)?
                        };
                        Some(screen_rect(p, world, camera))
                    }),
            )
            .collect()
    }

    /// Runs after the canvas, bars and overlay bodies have measured their current bounds.
    pub(in crate::window) fn browser_masks(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = &self.canvas_state;
        let doc = state.shown.as_ref().unwrap_or(&state.doc);
        let mut live = self.live_browser_views().peekable();
        live.peek()?;
        let origin = self
            .bounds
            .borrow()
            .get("canopy-canvas")
            .map(|b| b.origin)
            .unwrap_or_default();
        self.bounds.borrow_mut().remove("browser-canvas");
        let pages: Vec<_> = live
            .filter_map(|(key, view)| {
                let (id, _) = key.split_once('/')?;
                let expanded = state.browser_tab.as_deref() == Some(id);
                let body = doc
                    .panes
                    .iter()
                    .find(|p| p.id == id && doc.shows(p.space.as_ref()))
                    .filter(|_| state.browser_tab.is_none())
                    .and_then(|p| self.body_rect(p))
                    .map(|mut b| {
                        b.origin = b.origin - origin;
                        rect(b)
                    });
                let above = if body.is_some() {
                    self.page_covers(id)
                } else {
                    vec![]
                };
                Some((key.to_owned(), id.to_owned(), view, expanded, body, above))
            })
            .collect();
        let bounds = self.bounds.clone();
        let last = state.browsers.last_masks.clone();
        let this = cx.entity().downgrade();
        Some(
            canvas(
                move |_, window, cx| {
                    let bounds = bounds.borrow();
                    let canvas = bounds.get("browser-canvas").copied().map(rect);
                    let overlays: Vec<_> = bounds
                        .iter()
                        .filter(|(k, _)| k.starts_with("cover:"))
                        .map(|(_, b)| rect(*b))
                        .collect();
                    for (key, id, view, expanded, body, above) in pages {
                        let (frame, pieces) = match canvas {
                            Some(canvas @ [x, y, _, _]) if expanded || body.is_some() => {
                                let frame = if expanded {
                                    canvas
                                } else {
                                    let [a, b, w, h] = body.unwrap();
                                    [x + a, y + b, w, h]
                                };
                                let mut above: Vec<_> = above
                                    .into_iter()
                                    .map(|[a, b, w, h]| [x + a, y + b, w, h])
                                    .collect();
                                above.extend_from_slice(&overlays);
                                (frame, visible(frame, &covers(frame, canvas, &above)))
                            }
                            _ => ([0.; 4], vec![]),
                        };
                        // Compare local positions so a translating page can reuse its mask.
                        let local = (pieces != [frame]).then(|| local_rects(frame, &pieces));
                        if last.borrow().get(&key) == Some(&local) {
                            continue;
                        }
                        if set_mask(view.raw(), frame, &pieces) {
                            last.borrow_mut().insert(key, local);
                        } else {
                            let this = this.clone();
                            window.defer(cx, move |window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    if this.page_key(&id).as_deref() != Some(key.as_str()) { return; }
                                    if expanded
                                        && this.canvas_state.browser_tab.as_deref()
                                            == Some(id.as_str())
                                    {
                                        this.collapse_browser(window, cx);
                                    } else {
                                        this.rest_browser(&id, cx);
                                    }
                                });
                            });
                        }
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .w(px(0.))
            .h(px(0.))
            .into_any_element(),
        )
    }
}

#[cfg(not(target_os = "macos"))]
impl super::AppWindow {
    pub(in crate::window) fn browser_masks(
        &self,
        _: &mut gpui::Context<Self>,
    ) -> Option<gpui::AnyElement> {
        None
    }
}

#[cfg(target_os = "macos")]
pub(super) use native::clear_mask;
#[cfg(target_os = "macos")]
use native::{local_rects, set_mask};

#[cfg(target_os = "macos")]
mod native {
    use super::R;
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
    use objc2::{msg_send, sel};
    use objc2_app_kit::NSView;
    use objc2_core_graphics::CGMutablePath;
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use objc2_quartz_core::CAShapeLayer;
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex, OnceLock};
    use wry::WebViewExtMacOS;

    static MASKED_CLASS: OnceLock<Option<&'static AnyClass>> = OnceLock::new();
    static VISIBLE: LazyLock<Mutex<HashMap<usize, Vec<R>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    extern "C-unwind" fn hit_test(view: &NSView, _: Sel, point: NSPoint) -> *mut NSView {
        // SAFETY: AppKit calls hitTest on the main thread with a live NSView.
        let parent = unsafe { view.superview() };
        let point_in_view = view.convertPoint_fromView(point, parent.as_deref());
        let key = view as *const NSView as usize;
        let is_visible = VISIBLE.lock().unwrap().get(&key).is_none_or(|rects| {
            rects.iter().any(|&[x, y, w, h]| {
                let (x, y, w, h) = (f64::from(x), f64::from(y), f64::from(w), f64::from(h));
                point_in_view.x >= x
                    && point_in_view.x < x + w
                    && point_in_view.y >= y
                    && point_in_view.y < y + h
            })
        });
        if !is_visible {
            return std::ptr::null_mut();
        }
        let superclass = MASKED_CLASS
            .get()
            .copied()
            .flatten()
            .and_then(AnyClass::superclass)
            .expect("the installed hitTest method has a superclass");
        // SAFETY: the superclass is the original view class and hitTest returns an unretained NSView.
        unsafe { msg_send![super(view, superclass), hitTest: point] }
    }

    fn install_hit_test(view: &AnyObject) -> bool {
        let actual = view.class();
        let Some(class) = *MASKED_CLASS.get_or_init(|| {
            let mut builder = ClassBuilder::new(c"CanopyMaskedWKWebView", actual)?;
            // SAFETY: hit_test matches NSView's hitTest: ABI, including its unretained return.
            unsafe {
                builder.add_method(
                    sel!(hitTest:),
                    hit_test as extern "C-unwind" fn(_, _, _) -> _,
                );
            }
            Some(builder.register())
        }) else {
            return false;
        };
        if std::ptr::eq(actual, class) {
            return true;
        }
        if !class
            .superclass()
            .is_some_and(|parent| std::ptr::eq(actual, parent))
        {
            return false;
        }
        // SAFETY: this main-thread subclass has the same instance size, no ivars, and one ABI-compatible override.
        let old = unsafe { AnyObject::set_class(view, class) };
        debug_assert!(std::ptr::eq(old, actual));
        true
    }

    pub(super) fn local_rects([frame_x, frame_y, _, _]: R, visible: &[R]) -> Vec<R> {
        // WKWebView is flipped, so top-left window coordinates only need the view origin subtracted.
        visible
            .iter()
            .map(|&[x, y, w, h]| [x - frame_x, y - frame_y, w, h])
            .collect()
    }

    pub(in crate::window::canvas_view) fn clear_mask(view: &wry::WebView) {
        let webview = view.webview();
        let native: &NSView = &webview;
        if VISIBLE
            .lock()
            .unwrap()
            .remove(&(native as *const NSView as usize))
            .is_some()
            && let Some(layer) = native.layer()
        {
            // SAFETY: this is the view's layer on the main thread.
            unsafe {
                layer.setMask(None);
            }
        }
    }

    pub(super) fn set_mask(view: &wry::WebView, frame: R, visible: &[R]) -> bool {
        if visible == [frame] {
            clear_mask(view);
            return true;
        }
        let webview = view.webview();
        let native: &NSView = &webview;
        native.setWantsLayer(true);
        let Some(layer) = native.layer() else {
            return false;
        };
        if !install_hit_test(native) {
            return false;
        }

        let rects = local_rects(frame, visible);
        let path = CGMutablePath::new();
        for &[x, y, w, h] in &rects {
            let rect = NSRect::new(
                NSPoint::new(x.into(), y.into()),
                NSSize::new(w.into(), h.into()),
            );
            // SAFETY: a null transform leaves the rectangle in view coordinates.
            unsafe {
                CGMutablePath::add_rect(Some(&path), std::ptr::null(), rect);
            }
        }
        let shape = CAShapeLayer::layer();
        shape.setFrame(native.bounds());
        shape.setPath(Some(&path));
        // SAFETY: the new mask layer belongs to this main-thread view and has no parent.
        unsafe {
            layer.setMask(Some(&shape));
        }
        VISIBLE
            .lock()
            .unwrap()
            .insert(native as *const NSView as usize, rects);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{R, covers, intersection, visible};

    const PAGE: R = [0., 0., 100., 100.];

    fn assert_area(parts: &[R], expected: f32) {
        for (i, &a) in parts.iter().enumerate() {
            assert!(parts[i + 1..].iter().all(|&b| intersection(a, b).is_none()));
        }
        assert_eq!(parts.iter().map(|r| r[2] * r[3]).sum::<f32>(), expected);
    }

    #[test]
    fn unobstructed_page() {
        let holes = covers(PAGE, PAGE, &[]);
        assert!(holes.is_empty());
        assert_eq!(visible(PAGE, &holes), [PAGE]);
    }

    #[test]
    fn foreground_pane_covers_a_quarter() {
        let quarter = [0., 0., 50., 50.];
        let holes = covers(PAGE, PAGE, &[[-10., -10., 60., 60.], [100., 0., 10., 10.]]);
        assert_eq!(holes, [quarter]);
        let parts = visible(PAGE, &holes);
        assert!(parts.iter().all(|&r| intersection(r, quarter).is_none()));
        assert_area(&parts, 7_500.);
    }

    #[test]
    fn overlapping_holes_are_subtracted_once() {
        let holes = [[0., 0., 50., 50.], [25., 25., 50., 50.]];
        let parts = visible(PAGE, &holes);
        assert!(
            parts
                .iter()
                .all(|&r| holes.iter().all(|&h| intersection(r, h).is_none()))
        );
        assert_area(&parts, 5_625.);
    }

    #[test]
    fn canvas_edge_covers_offscreen_half() {
        let holes = covers(PAGE, [50., 0., 100., 100.], &[]);
        assert_eq!(holes, [[0., 0., 50., 100.]]);
        assert_eq!(visible(PAGE, &holes), [[50., 0., 50., 100.]]);
    }
}
