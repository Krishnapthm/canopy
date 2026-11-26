//! The camera: where the canvas is looked at from, and how far in. It is the only bridge
//! between world px and screen px, so its fields are private and every conversion has a name.
use super::units::{Point, Rect, intersects, union};
use serde::{Deserialize, Serialize};

/// The zoom steps: keyboard and menu zoom land on these, and they bound every zoom.
pub const ZOOMS: [f32; 12] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1., 1.1, 1.25, 1.5, 1.75, 2.,
];
/// Screen px kept clear around panes (both sides together) when fitting or zooming to a limit.
pub const FIT_PAD: f32 = 80.;

/// `x`, `y`: the world point at the canvas's top-left corner. `zoom`: screen px per world px.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    x: f32,
    y: f32,
    zoom: f32,
}
impl Default for Camera {
    fn default() -> Self { Self::new(0., 0., 1.) }
}
impl Camera {
    pub const fn new(x: f32, y: f32, zoom: f32) -> Self { Self { x, y, zoom } }
    /// Screen px per world px.
    pub fn zoom(self) -> f32 { self.zoom }
    /// The world point at the canvas's top-left corner.
    pub fn origin(self) -> Point { [self.x, self.y] }
    /// Whether every field is a finite number (a saved camera may not be).
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() && self.zoom.is_finite() }
    /// This camera with its zoom held inside `ZOOMS`, keeping its origin.
    pub fn clamped(self) -> Self { Self { zoom: self.zoom.clamp(ZOOMS[0], ZOOMS[11]), ..self } }

    pub fn to_screen(self, p: Point) -> Point {
        [(p[0] - self.x) * self.zoom, (p[1] - self.y) * self.zoom]
    }
    pub fn to_world(self, p: Point) -> Point {
        [p[0] / self.zoom + self.x, p[1] / self.zoom + self.y]
    }
    /// A world length in screen px.
    pub fn len_to_screen(self, world: f32) -> f32 { world * self.zoom }
    /// A screen length in world px.
    pub fn len_to_world(self, screen: f32) -> f32 { screen / self.zoom }
    /// A world size in screen px.
    pub fn size_to_screen(self, s: Point) -> Point { [s[0] * self.zoom, s[1] * self.zoom] }
    /// A screen-px move or size in world px.
    pub fn size_to_world(self, s: Point) -> Point { [s[0] / self.zoom, s[1] / self.zoom] }
    /// A world rect as `[x, y, w, h]` screen px.
    pub fn rect_to_screen(self, r: impl Into<Rect>) -> Rect {
        let r = r.into();
        let [x, y] = self.to_screen([r[0], r[1]]);
        [x, y, r[2] * self.zoom, r[3] * self.zoom]
    }

    pub fn pan(&mut self, d: Point) {
        self.x -= d[0] / self.zoom;
        self.y -= d[1] / self.zoom;
    }
    pub fn zoom_at(&mut self, zoom: f32, anchor: Point) {
        let p = self.to_world(anchor);
        self.zoom = zoom;
        self.x = p[0] - anchor[0] / self.zoom;
        self.y = p[1] - anchor[1] / self.zoom;
    }
    /// Zooms to the next `ZOOMS` step in `direction`, kept inside `range` (see `zoom_range`).
    pub fn step(&mut self, direction: i32, anchor: Point, range: [f32; 2]) {
        let next = if direction > 0 {
            ZOOMS.into_iter().find(|z| *z > self.zoom * 1.01).unwrap_or(ZOOMS[11])
        } else {
            ZOOMS.into_iter().rev().find(|z| *z < self.zoom * 0.99).unwrap_or(ZOOMS[0])
        };
        self.zoom_at(next.clamp(range[0], range[1]), anchor);
    }
    pub fn fit(&mut self, rects: &[Rect], viewport: Point) {
        let Some(bounds) = union(rects) else { return };
        self.zoom = step_down(fit_zoom(bounds, viewport));
        self.x = bounds[0] + bounds[2] / 2. - viewport[0] / self.zoom / 2.;
        self.y = bounds[1] + bounds[3] / 2. - viewport[1] / self.zoom / 2.;
    }
    /// The camera at `zoom` that puts world point `world` under screen point `anchor`.
    pub fn anchored(world: Point, anchor: Point, zoom: f32) -> Self {
        Self::new(world[0] - anchor[0] / zoom, world[1] - anchor[1] / zoom, zoom)
    }
    /// The camera at `zoom` whose `viewport` (screen px) is centered on world point `center`.
    pub fn centered(center: Point, viewport: Point, zoom: f32) -> Self {
        Self::new(center[0] - viewport[0] / 2. / zoom, center[1] - viewport[1] / 2. / zoom, zoom)
    }
}

/// The zoom at which `rect` just fits `viewport` with `FIT_PAD` around it.
fn fit_zoom(rect: Rect, viewport: Point) -> f32 {
    ((viewport[0] - FIT_PAD).max(1.) / rect[2].max(1.)).min((viewport[1] - FIT_PAD).max(1.) / rect[3].max(1.))
}
/// The largest `ZOOMS` step at or below `z`.
fn step_down(z: f32) -> f32 { ZOOMS.into_iter().rev().find(|v| *v <= z).unwrap_or(ZOOMS[0]) }
/// The [min, max] zoom for a place whose panes are `rects`: zooming in stops while the biggest
/// pane still fits the viewport with padding, zooming out once everything fits (the fit-all
/// zoom, though never forced above 100%). Both stay inside `ZOOMS`.
pub fn zoom_range(rects: &[Rect], viewport: Point) -> [f32; 2] {
    let max = rects.iter().map(|r| fit_zoom(*r, viewport)).fold(ZOOMS[11], f32::min).max(ZOOMS[0]);
    let min = union(rects).map_or(ZOOMS[0], |b| step_down(fit_zoom(b, viewport))).min(1.).min(max);
    [min, max]
}
/// `range` stretched to hold `zoom`, so a zoom already past a limit is not pulled back mid-gesture.
pub fn widen(range: [f32; 2], zoom: f32) -> [f32; 2] { [range[0].min(zoom), range[1].max(zoom)] }

/// The camera that centers `rect` at the same zoom when it is not wholly inside `visible`
/// (screen px); None when it already is.
pub fn reveal(camera: Camera, rect: impl Into<Rect>, visible: Point) -> Option<Camera> {
    let rect = rect.into();
    let [x, y, w, h] = camera.rect_to_screen(rect);
    if x >= 0. && y >= 0. && x + w <= visible[0] && y + h <= visible[1] { return None; }
    Some(Camera::centered([rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.], visible, camera.zoom))
}

/// Toggle a space fit in the visible canvas (`[x, y, w, h]` screen px, the part no chrome
/// covers): a second press restores the camera from before the fit, unless the camera moved
/// since, in which case it fits again.
pub fn focus_fit(camera: &mut Camera, previous: &mut Option<(Camera, Camera)>, rects: &[Rect], visible: Rect) {
    match previous.take() {
        Some((before, fitted)) if *camera == fitted => *camera = before,
        _ if !rects.is_empty() => {
            let before = *camera;
            camera.fit(rects, [visible[2], visible[3]]);
            camera.x -= visible[0] / camera.zoom;
            camera.y -= visible[1] / camera.zoom;
            *previous = Some((before, *camera));
        }
        _ => {}
    }
}

/// Whether `rect` (world) is near enough to `viewport` (screen px) to draw: the view grown by a
/// pill's width sideways and its height up and down, so pills slide in rather than pop.
pub fn visible(camera: Camera, rect: impl Into<Rect>, viewport: Point) -> bool {
    let p = camera.to_world([0., 0.]);
    intersects(
        rect.into(),
        [
            p[0] - 320.,
            p[1] - 36.,
            viewport[0] / camera.zoom + 640.,
            viewport[1] / camera.zoom + 72.,
        ],
    )
}

