//! World geometry. A world px is a canvas px at 100% zoom; screen geometry is GPUI's own
//! `Point`/`Size`/`Bounds<Pixels>`, and the camera is the only bridge between the two.
//! Each type saves as a plain array, so `canvas.json` keeps its shape.
use serde::{Deserialize, Serialize};

/// An untyped `[x, y]` (or `[w, h]`): world or screen, as the name using it says.
pub type Point = [f32; 2];
/// An untyped `[x, y, w, h]`: world or screen, as the name using it says.
pub type Rect = [f32; 4];

/// A point on the canvas, in world px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Point", into = "Point")]
pub struct WorldPoint {
    pub x: f32,
    pub y: f32,
}

/// A size on the canvas, in world px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Point", into = "Point")]
pub struct WorldSize {
    pub w: f32,
    pub h: f32,
}

/// A rectangle on the canvas, in world px: its top-left corner and size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Rect", into = "Rect")]
pub struct WorldRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl WorldPoint {
    pub const fn new(x: f32, y: f32) -> Self { Self { x, y } }
}
impl WorldSize {
    pub const fn new(w: f32, h: f32) -> Self { Self { w, h } }
}
impl WorldRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self { Self { x, y, w, h } }
    pub fn at(origin: WorldPoint, size: WorldSize) -> Self { Self::new(origin.x, origin.y, size.w, size.h) }
    pub fn origin(self) -> WorldPoint { WorldPoint::new(self.x, self.y) }
    pub fn size(self) -> WorldSize { WorldSize::new(self.w, self.h) }
    pub fn center(self) -> WorldPoint { WorldPoint::new(self.x + self.w / 2., self.y + self.h / 2.) }
    /// The `[x, y, w, h]` form the geometry rules take.
    pub fn array(self) -> Rect { self.into() }
}

impl From<Point> for WorldPoint { fn from([x, y]: Point) -> Self { Self { x, y } } }
impl From<WorldPoint> for Point { fn from(p: WorldPoint) -> Self { [p.x, p.y] } }
impl From<Point> for WorldSize { fn from([w, h]: Point) -> Self { Self { w, h } } }
impl From<WorldSize> for Point { fn from(s: WorldSize) -> Self { [s.w, s.h] } }
impl From<Rect> for WorldRect { fn from([x, y, w, h]: Rect) -> Self { Self { x, y, w, h } } }
impl From<WorldRect> for Rect { fn from(r: WorldRect) -> Self { [r.x, r.y, r.w, r.h] } }

/// The smallest rect holding all of `rects`; None for none.
pub fn union(rects: &[Rect]) -> Option<Rect> {
    let mut out = *rects.first()?;
    for r in &rects[1..] {
        let x = out[0].min(r[0]);
        let y = out[1].min(r[1]);
        out = [
            x,
            y,
            (out[0] + out[2]).max(r[0] + r[2]) - x,
            (out[1] + out[3]).max(r[1] + r[3]) - y,
        ];
    }
    Some(out)
}

/// Whether two rects overlap (touching edges do not).
pub fn intersects(a: Rect, b: Rect) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}
