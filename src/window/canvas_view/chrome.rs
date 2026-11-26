//! Places canvas controls in the current viewport, outside the panes they serve.
use super::*;

pub(super) type Placements = Rc<RefCell<Vec<Placed>>>;

pub(super) struct Placed {
    pub pane: String,
    pub bounds: Bounds<Pixels>,
    pub port: bool,
}

pub(super) struct Control {
    pub pane: String,
    pub host: Bounds<Pixels>,
    /// Canvas-local origin for a wire handle. Helper bars choose an edge.
    pub port: Option<Point<Pixels>>,
    pub element: AnyElement,
}

pub(super) struct Layer {
    pub controls: Vec<Control>,
    pub panes: Vec<Bounds<Pixels>>,
    pub placed: Placements,
    pub bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
}

impl IntoElement for Layer {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Layer {
    type RequestLayoutState = ();
    type PrepaintState = Vec<(usize, Bounds<Pixels>)>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let style = Style {
            position: Position::Absolute,
            size: Size::full(),
            ..Style::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        canvas: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let mut placed = self.placed.borrow_mut();
        placed.clear();
        let mut drawn = Vec::new();
        let local_canvas = Bounds::new(point(px(0.), px(0.)), canvas.size);
        let mut occupied = std::mem::take(&mut self.panes);
        occupied.retain(|pane| pane.intersects(&local_canvas));
        occupied.extend(
            self.bounds
                .borrow()
                .iter()
                .filter(|(key, _)| key.starts_with("cover:canvas-float:"))
                .map(|(_, b)| Bounds::new(b.origin - canvas.origin, b.size)),
        );

        for (index, control) in self.controls.iter_mut().enumerate() {
            let visible = control.host.intersect(&local_canvas);
            if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
                continue;
            }
            let size = control.element.layout_as_root(
                size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                window,
                cx,
            );
            let placement = if let Some(at) = control.port {
                let bounds = Bounds::new(at, size);
                fits(bounds, local_canvas, &occupied).then_some(bounds)
            } else {
                place_helper(size, control.host, local_canvas, &occupied)
            };
            if let Some(bounds) = placement {
                occupied.push(bounds);
                let bounds = Bounds::new(canvas.origin + bounds.origin, bounds.size);
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    control.element.prepaint_at(bounds.origin, window, cx);
                });
                placed.push(Placed {
                    pane: control.pane.clone(),
                    bounds,
                    port: control.port.is_some(),
                });
                drawn.push((index, bounds));
            }
        }
        drawn
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        drawn: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for &(index, bounds) in drawn.iter() {
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                self.controls[index].element.paint(window, cx);
            });
        }
    }
}

fn fits(candidate: Bounds<Pixels>, clip: Bounds<Pixels>, occupied: &[Bounds<Pixels>]) -> bool {
    candidate.size.width > px(0.)
        && candidate.size.height > px(0.)
        && candidate.left() >= clip.left()
        && candidate.right() <= clip.right()
        && candidate.top() >= clip.top()
        && candidate.bottom() <= clip.bottom()
        && occupied.iter().all(|rect| !candidate.intersects(rect))
}

/// For each edge, the closest open position is at the preferred origin or next
/// to the edge of an obstruction. Slide parallel to that edge only.
fn place_helper(
    size: Size<Pixels>,
    pane: Bounds<Pixels>,
    clip: Bounds<Pixels>,
    occupied: &[Bounds<Pixels>],
) -> Option<Bounds<Pixels>> {
    let gap = px(12.);
    if size.width > clip.size.width || size.height > clip.size.height {
        return None;
    }

    for (horizontal, across) in [
        (true, pane.top() - size.height - gap),
        (true, pane.bottom() + gap),
        (false, pane.right() + gap),
        (false, pane.left() - size.width - gap),
    ] {
        let (preferred, low, high, length) = if horizontal {
            (pane.left(), clip.left(), clip.right(), size.width)
        } else {
            (pane.top(), clip.top(), clip.bottom(), size.height)
        };
        let preferred = preferred.clamp(low, high - length);
        let anchor = if horizontal {
            (pane.left(), pane.right())
        } else {
            (pane.top(), pane.bottom())
        };
        let candidate = |along| {
            let origin = if horizontal {
                point(along, across)
            } else {
                point(across, along)
            };
            Bounds::new(origin, size)
        };

        // Obstacles in this band each forbid an interval of origins along the edge.
        let band = candidate(preferred);
        let mut blocked: Vec<_> = occupied
            .iter()
            .filter_map(|r| {
                if horizontal && r.top() < band.bottom() && r.bottom() > band.top() {
                    Some((r.left() - length, r.right()))
                } else if !horizontal && r.left() < band.right() && r.right() > band.left() {
                    Some((r.top() - length, r.bottom()))
                } else {
                    None
                }
            })
            .collect();
        blocked.sort_unstable();

        let open = |at, (start, end), forward| {
            if start < at && at < end {
                if forward { end } else { start }
            } else {
                at
            }
        };
        let before = blocked
            .iter()
            .rev()
            .fold(preferred, |at, &range| open(at, range, false));
        let after = blocked
            .into_iter()
            .fold(preferred, |at, range| open(at, range, true));
        let closest = [before, after]
            .into_iter()
            .filter(|along| *along < anchor.1 && *along + length > anchor.0)
            .map(candidate)
            .filter(|&bounds| fits(bounds, clip, occupied))
            .min_by_key(|bounds| {
                let along = if horizontal {
                    bounds.left()
                } else {
                    bounds.top()
                };
                ((along - preferred).abs(), along)
            });
        if closest.is_some() {
            return closest;
        }
    }
    None
}

impl AppWindow {
    /// Measured canvas viewport, in window pixels, excluding persistent chrome.
    pub(in crate::window) fn canvas_bounds(&self) -> Option<Bounds<Pixels>> {
        if !self.has_canvas() || self.canvas_state.panel_expanded {
            return None;
        }
        self.bounds.borrow().get("canopy-canvas").copied()
    }

    /// The part of the canvas the floating toolbar leaves open, as canvas-local `[x, y, w, h]`
    /// screen px (the window when the canvas is not measured): fitting, zoom limits and the
    /// empty-canvas hints use it, so nothing lands under the toolbar.
    pub(in crate::window) fn open_canvas(&self, window: &Window) -> canophy::Rect {
        self.open_canvas_in(window.viewport_size())
    }

    pub(in crate::window) fn open_canvas_in(&self, fallback: Size<Pixels>) -> canophy::Rect {
        let canvas = self.canvas_bounds().unwrap_or(Bounds::new(point(px(0.), px(0.)), fallback));
        let bounds = self.bounds.borrow();
        let edge = self.chrome.walk().iter().flat_map(|n| &n.floating).find(|f| f.id == "toolbar" && !f.hidden).map(|f| f.edge);
        let toolbar = bounds.get("cover:canvas-float:toolbar").zip(edge)
            .map(|(b, edge)| (Bounds::new(b.origin - canvas.origin, b.size), edge));
        uncovered(canvas.size, toolbar)
    }

    /// Whether canvas chrome owns an input position in the displayed frame.
    pub(in crate::window) fn canvas_chrome_at(&self, point: Point<Pixels>) -> bool {
        self.canvas_state
            .chrome
            .borrow()
            .iter()
            .any(|control| control.bounds.contains(&point))
            || self
                .bounds
                .borrow()
                .iter()
                .any(|(key, b)| key.starts_with("cover:canvas-float:") && b.contains(&point))
    }

    /// A pane's actual screen footprint, relative to the canvas viewport.
    pub(super) fn pane_screen_bounds(&self, pane: &canvas::Pane) -> Option<Bounds<Pixels>> {
        let [x, y, w, h] = canophy::layout::screen(self.canvas_state.doc.camera, pane, self.wire_cell_rect(pane)?);
        Some(Bounds::new(point(px(x), px(y)), size(px(w), px(h))))
    }
}

/// `size` less the strip from its `edge` to the far side of `cover` (canvas-local), as `[x, y, w, h]`.
fn uncovered(size: Size<Pixels>, cover: Option<(Bounds<Pixels>, crate::layout::Edge)>) -> canophy::Rect {
    use crate::layout::Edge;
    let [w, h] = [f32::from(size.width), f32::from(size.height)];
    let Some((b, edge)) = cover else { return [0., 0., w, h] };
    let [left, top, right, bottom] = [b.left(), b.top(), b.right(), b.bottom()].map(f32::from);
    match edge {
        Edge::Top => { let y = bottom.clamp(0., h); [0., y, w, h - y] }
        Edge::Bottom => [0., 0., w, top.clamp(0., h)],
        Edge::Left => { let x = right.clamp(0., w); [x, 0., w - x, h] }
        Edge::Right => [0., 0., left.clamp(0., w), h],
    }
}

#[cfg(test)]
mod tests {
    use super::{fits, place_helper, uncovered};
    use crate::layout::Edge;
    use gpui::{Bounds, Pixels, point, px, size};

    use crate::window::tests::{in_own_process, open_window, running_alone};
    use gpui::TestAppContext;
    use serde_json::json;

    const DOCK: &str = "window::canvas_view::chrome::tests::toolbar_floats_over_the_canvas_column";

    #[test]
    fn toolbar_and_dock_have_separate_bounds() {
        in_own_process(DOCK);
    }

    #[gpui::test]
    fn toolbar_floats_over_the_canvas_column(cx: &mut TestAppContext) {
        if !running_alone(DOCK) {
            return;
        }
        let (view, cx) = open_window(
            cx,
            "chrome-probe",
            "",
            json!({
                "id":"root", "type":"canvas",
                "floating":[{"id":"toolbar","content":{"mod":"toolbar"},"edge":"bottom","align":"center"}]
            }),
        );
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| {
            this.canvas_state.files = true;
            this.canvas_state.doc.panel.width = Some(480.);
            this.canvas_state.doc.panes = serde_json::from_value(json!([
                {"id":"image","kind":"image","props":{"path":"/missing.png"},"rect":[40,100,200,160],"z":1},
                {"id":"note","kind":"mod:note","props":{"text":"Note"},"rect":[145,135,210,160],"z":2}
            ])).unwrap();
            this.canvas_state.selected = vec!["image".into(), "note".into()];
            this.sync(window, cx);
            cx.notify();
        });
        cx.update(|window, cx| window.simulate_next_frame(cx));
        cx.run_until_parked();

        cx.update(|window, cx| view.read_with(cx, |this, _| {
            let viewport = this.canvas_bounds().unwrap();
            let bounds = this.bounds.borrow();
            let toolbar = bounds["float:toolbar"];
            // The toolbar floats over the canvas, which runs under it to the window edge.
            assert!(toolbar.size.width > px(20.));
            assert!(viewport.top() < toolbar.top() && toolbar.bottom() <= viewport.bottom());
            assert!(toolbar.right() <= viewport.right());
            assert!(viewport.right() < bounds["root"].right());
            assert!(this.canvas_chrome_at(toolbar.center()));
            let cover = bounds["cover:canvas-float:toolbar"];
            drop(bounds);
            // Fitting and zoom limits use the canvas above the toolbar.
            let open = this.open_canvas(window);
            assert!(cover.top() >= toolbar.top());
            assert_eq!(open[3], f32::from(cover.top() - viewport.top()));
            assert_eq!(open[2], f32::from(viewport.size.width));

            let placed = this.canvas_state.chrome.borrow();
            assert_eq!(placed.len(), 2);
            for (index, control) in placed.iter().enumerate() {
                assert!(fits(control.bounds, viewport, &[]));
                assert!(
                    placed[index + 1..]
                        .iter()
                        .all(|other| !control.bounds.intersects(&other.bounds))
                );
                assert!(this.canvas_chrome_at(control.bounds.center()));
            }
        }));
    }

    #[test]
    fn the_toolbar_strip_is_left_out_of_the_open_canvas() {
        let size = size(px(800.), px(600.));
        assert_eq!(uncovered(size, None), [0., 0., 800., 600.]);
        assert_eq!(uncovered(size, Some((rect(300., 540., 200., 44.), Edge::Bottom))), [0., 0., 800., 540.]);
        assert_eq!(uncovered(size, Some((rect(300., 16., 200., 44.), Edge::Top))), [0., 60., 800., 540.]);
        assert_eq!(uncovered(size, Some((rect(16., 200., 44., 200.), Edge::Left))), [60., 0., 740., 600.]);
        assert_eq!(uncovered(size, Some((rect(740., 200., 44., 200.), Edge::Right))), [0., 0., 740., 600.]);
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn helpers_slide_then_use_another_edge_without_covering_content() {
        let clip = rect(0., 0., 320., 240.);
        let pane = rect(40., 50., 100., 100.);
        let blocker = rect(30., 10., 70., 30.);
        let size = size(px(50.), px(24.));
        let first = place_helper(size, pane, clip, &[pane, blocker]).unwrap();
        assert_eq!(first, rect(100., 14., 50., 24.));
        let second = place_helper(size, pane, clip, &[pane, blocker, first]).unwrap();
        assert_eq!(second, rect(40., 162., 50., 24.));
        assert!(fits(second, clip, &[pane, blocker, first]));
    }

    #[test]
    fn clipped_edges_and_oversized_helpers_have_no_out_of_bounds_fallback() {
        let clip = rect(0., 0., 140., 120.);
        let pane = rect(100., -25., 60., 100.);
        let size = size(px(65.), px(24.));
        assert_eq!(
            place_helper(size, pane, clip, &[pane]),
            Some(rect(75., 87., 65., 24.))
        );
        assert_eq!(
            place_helper(
                size,
                pane,
                clip,
                &[pane, rect(0., 75., 140., 45.), rect(0., 0., 100., 75.)]
            ),
            None
        );
        assert_eq!(
            place_helper(gpui::size(px(141.), px(24.)), pane, clip, &[pane]),
            None
        );
    }
}
