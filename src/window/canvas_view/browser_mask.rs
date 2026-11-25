//! Clips live browser views to the canvas and routes clicks through covered areas to GPUI.
//! The container filters hit tests so WebKit keeps its original class and AppKit state.
#[cfg(target_os = "macos")]
use super::*;

#[cfg(target_os = "macos")]
type LocalMask = Option<([f32; 2], Vec<R>, Vec<R>)>;

#[cfg(target_os = "macos")]
pub(super) type MaskCache = Rc<RefCell<HashMap<String, LocalMask>>>;

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

/// A native page without a current host has no pixels or hit targets in this window.
#[cfg(any(target_os = "macos", test))]
fn visible_in_host(frame: R, clip: Option<R>, above: &[R]) -> Vec<R> {
    clip.map_or_else(Vec::new, |clip| visible(frame, &covers(frame, clip, above)))
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
    /// Rectangles over a page relative to the canvas origin, which prepaint measures: foreground
    /// panes (hidden and click-through), then the pane's resize grips (click-through only).
    fn page_covers(&self, id: &str) -> (Vec<R>, Vec<R>) {
        let state = &self.canvas_state;
        let doc = state.shown.as_ref().unwrap_or(&state.doc);
        let Some(page) = doc.panes.iter().find(|p| p.id == id) else {
            return (vec![], vec![]);
        };
        let camera = state.doc.camera;

        // shortcut: GPUI tooltips lack cover bounds, measure them if they need to overlap live pages.
        // shortcut: rectangular holes leave square corners, use pane-shaped paths if the corners matter.
        let [x, y, w, h] = screen_rect(page, page.rect, camera);
        let (edge, corner) = resize_zone_sizes(camera.zoom);
        let (x, y, w, h) = (x + 1., y + 1., w - 2., h - 2.);
        // WebKit's native hit test also needs to leave the GPUI resize zones exposed.
        let grips = [
            [x + corner, y, w - 2. * corner, edge],
            [x + corner, y + h - edge, w - 2. * corner, edge],
            [x, y + corner, edge, h - 2. * corner],
            [x + w - edge, y + corner, edge, h - 2. * corner],
            [x, y, corner, corner],
            [x + w - corner, y, corner, corner],
            [x, y + h - corner, corner, corner],
            [x + w - corner, y + h - corner, corner, corner],
        ];
        let panes = doc.panes.iter()
            .filter(|p| doc.shown(p) && paint_order(p) > paint_order(page))
            .filter_map(|p| self.pane_screen_bounds(p).map(rect))
            .collect();
        (panes, grips.to_vec())
    }

    /// Runs after the canvas, controls and the native content slots measure this frame.
    pub(in crate::window) fn browser_masks(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = &self.canvas_state;
        self.bounds.borrow_mut().retain(|key, _| key != "browser-canvas"
            && !key.starts_with("web-frame:") && !key.starts_with("web-body:")
            && !key.starts_with("cover:annotate"));
        let mut live = self.live_browser_views().peekable();
        live.peek()?;
        let pages: Vec<_> = live.filter_map(|(key, view)| {
            let (id, _) = key.split_once('/')?;
            let expanded = state.browser_tab.as_deref() == Some(id);
            let docked = self.has_canvas() && state.active_panel_pane() == Some(id);
            let (above, grips) = if expanded || docked { (vec![], vec![]) } else { self.page_covers(id) };
            Some((key.to_owned(), id.to_owned(), view, expanded, docked, above, grips))
        }).collect();
        let bounds = self.bounds.clone();
        // A menu's frame is wider than its painted boxes (room for a submenu). The page shows
        // through the empty part, but clicks there still reach the menu, which closes on them.
        let menu_frames: Vec<String> = self.overlays.iter()
            .filter(|o| self.copies.get(&o.slot).is_some_and(|copy| copy.inst.name == "menu"))
            .map(|o| format!("cover:overlay:{}", o.id)).collect();
        let controls = state.chrome.clone();
        let last = state.browsers.last_masks.clone();
        let this = cx.entity().downgrade();
        Some(
            canvas(
                move |_, window, cx| {
                    let bounds = bounds.borrow();
                    let canvas = bounds.get("browser-canvas").copied();
                    let mut overlays: Vec<_> = bounds.iter().filter(|(k, _)| k.starts_with("cover:")).collect();
                    overlays.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
                    let (frames, overlays): (Vec<_>, Vec<_>) = overlays.into_iter().partition(|(k, _)| menu_frames.contains(k));
                    let mut overlays: Vec<_> = overlays.into_iter().map(|(_, bounds)| rect(*bounds)).collect();
                    overlays.extend(gpui_shell::menu_covers().into_iter().map(rect));
                    let frames: Vec<_> = frames.into_iter().map(|(_, bounds)| rect(*bounds)).collect();
                    let controls = controls.borrow();
                    for (key, id, view, expanded, docked, above, grips) in pages {
                        let host = if docked && !expanded {
                            bounds.get(&format!("panel-body:{id}")).copied()
                        } else {
                            canvas
                        };
                        let clip = host.map(|host| {
                            bounds.get(&format!("web-body:{id}")).map_or(host, |body| host.intersect(body))
                        });
                        let frame = bounds.get(&format!("web-frame:{key}")).copied().map(rect);
                        let frame = frame.unwrap_or([0.; 4]);
                        let to_window = |rects: Vec<R>| -> Vec<R> { rects.into_iter().filter_map(|[x, y, w, h]| {
                            let canvas = canvas?;
                            Some([x + f32::from(canvas.origin.x), y + f32::from(canvas.origin.y), w, h])
                        }).collect() };
                        let mut above = to_window(above);
                        above.extend_from_slice(&overlays);
                        above.extend(controls.iter().map(|control| rect(control.bounds)));
                        let pieces = visible_in_host(frame, clip.map(rect), &above);
                        // Grips and menu frames take clicks from the page but never hide it.
                        let grips = to_window(grips);
                        above.extend_from_slice(&frames);
                        above.extend_from_slice(&grips);
                        let hits = if frames.is_empty() && grips.is_empty() { pieces.clone() } else { visible_in_host(frame, clip.map(rect), &above) };
                        // Translation alone can reuse the mask. Resizing also changes its layer frame.
                        let local = (pieces != [frame] || hits != [frame])
                            .then(|| ([frame[2], frame[3]], local_rects(frame, &pieces), local_rects(frame, &hits)));
                        if last.borrow().get(&key) == Some(&local) {
                            continue;
                        }
                        if set_mask(view.raw(), frame, &pieces, &hits) {
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
pub(super) use native::{clear_mask, set_mask, wrap};
#[cfg(target_os = "macos")]
use native::local_rects;

#[cfg(target_os = "macos")]
mod native {
    use super::R;
    use objc2::rc::Retained;
    use objc2::{DefinedClass, MainThreadMarker, define_class, msg_send};
    use objc2_app_kit::{NSAutoresizingMaskOptions, NSView};
    use objc2_core_graphics::CGMutablePath;
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use objc2_quartz_core::CAShapeLayer;
    use std::cell::RefCell;
    use wry::WebViewExtMacOS;

    define_class!(
        /// Hosts a live WKWebView and passes clicks through its covered regions.
        /// Keeping the web view's class intact preserves AppKit's property bookkeeping.
        #[unsafe(super = NSView)]
        #[thread_kind = objc2::MainThreadOnly]
        #[ivars = RefCell<Option<Vec<R>>>]
        struct CanopyBrowserClip;

        impl CanopyBrowserClip {
            #[unsafe(method(isFlipped))]
            fn is_flipped(&self) -> bool {
                true
            }

            #[unsafe(method(hitTest:))]
            fn hit_test(&self, point: NSPoint) -> *mut NSView {
                if let Some(rects) = self.ivars().borrow().as_ref() {
                    let Some(webview) = self.subviews().firstObject() else {
                        return std::ptr::null_mut();
                    };
                    // SAFETY: AppKit calls hitTest on the main thread with a live view.
                    let parent = unsafe { self.superview() };
                    let point = webview.convertPoint_fromView(point, parent.as_deref());
                    if !rects.iter().any(|&[x, y, w, h]| {
                        let (x, y, w, h) = (f64::from(x), f64::from(y), f64::from(w), f64::from(h));
                        point.x >= x && point.x < x + w && point.y >= y && point.y < y + h
                    }) {
                        return std::ptr::null_mut();
                    }
                }
                // SAFETY: NSView implements hitTest: with an unretained NSView return.
                let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
                if std::ptr::eq(hit, &**self) {
                    return std::ptr::null_mut();
                }
                hit
            }
        }
    );

    pub(in crate::window::canvas_view) fn wrap(
        view: &wry::WebView,
    ) -> Result<Retained<NSView>, String> {
        let mtm = MainThreadMarker::new().expect("web views are built on the main thread");
        let webview = view.webview();
        // SAFETY: wry attached the web view on the main thread.
        let parent = unsafe { webview.superview() }.ok_or("web view has no parent")?;
        let clip = mtm.alloc::<CanopyBrowserClip>().set_ivars(RefCell::new(None));
        // SAFETY: initialize the NSView superclass with the parent's bounds.
        let clip: Retained<CanopyBrowserClip> =
            unsafe { msg_send![super(clip), initWithFrame: parent.bounds()] };
        clip.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        clip.setWantsLayer(true);
        // wry retains the original parent for returning focus to GPUI.
        parent.addSubview(&clip);
        clip.addSubview(&webview);
        Ok(Retained::into_super(clip))
    }

    fn container(webview: &NSView) -> Option<Retained<CanopyBrowserClip>> {
        // SAFETY: only called on the main thread with a live web view.
        unsafe { webview.superview() }?.downcast().ok()
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
        if let Some(clip) = container(&webview)
            && clip.ivars().borrow_mut().take().is_some()
            && let Some(layer) = webview.layer()
        {
            // SAFETY: this is the view's layer on the main thread.
            unsafe {
                layer.setMask(None);
            }
        }
    }

    /// Paints the page only in `visible` and lets it take clicks only in `hits` (a subset).
    pub(in crate::window::canvas_view) fn set_mask(view: &wry::WebView, frame: R, visible: &[R], hits: &[R]) -> bool {
        let webview = view.webview();
        let Some(clip) = container(&webview) else {
            return false;
        };
        if visible == [frame] && hits == [frame] {
            clear_mask(view);
            return true;
        }
        let native: &NSView = &webview;
        native.setWantsLayer(true);
        let Some(layer) = native.layer() else {
            return false;
        };

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
        *clip.ivars().borrow_mut() = Some(local_rects(frame, hits));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{R, covers, intersection, visible, visible_in_host};

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
    fn native_page_uses_the_current_dock_clip_for_both_paint_and_input() {
        let live_frame = [690., 62., 600., 420.];
        let dock_before = [700., 80., 500., 360.];
        let dock_after_resize = [820., 80., 380., 360.];
        let popup = [880., 120., 130., 160.];
        for dock in [dock_before, dock_after_resize] {
            let parts = visible_in_host(live_frame, Some(dock), &[popup]);
            assert!(parts.iter().all(|&part| intersection(part, dock) == Some(part)
                && intersection(part, popup).is_none()));
            assert_area(&parts, dock[2] * dock[3] - popup[2] * popup[3]);
        }
        assert!(visible_in_host(live_frame, None, &[popup]).is_empty());
    }

    #[test]
    fn canvas_edge_covers_offscreen_half() {
        let holes = covers(PAGE, [50., 0., 100., 100.], &[]);
        assert_eq!(holes, [[0., 0., 50., 100.]]);
        assert_eq!(visible(PAGE, &holes), [[50., 0., 50., 100.]]);
    }
}
