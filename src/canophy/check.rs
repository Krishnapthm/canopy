//! The invariants (I-numbers from the plan's Enforcement list). Debug builds run `document`
//! after every `step`; the tests run the rest too. Each returns the first violation it finds.
// Debug builds call only `document`; the rest serve the tests.
#![cfg_attr(not(test), allow(dead_code))]
use super::camera::{Camera, ZOOMS};
use super::units::{Point, Rect};
use crate::canvas::Document;

/// A broken invariant, with what broke it.
#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// I1: a rect has a non-finite number or no area.
    BadRect { id: String, rect: Rect },
    /// I1: a resize left a rect below its kind's minimum.
    BelowMin { size: Point, min: Point },
    /// I3: two pushable panes overlap (closer than the gap).
    Overlap { a: usize, b: usize },
    /// I4: the zoom left `ZOOMS` or the allowed range.
    Zoom { zoom: f32, range: [f32; 2] },
    /// I4: a camera field is not a finite number.
    BadCamera,
    /// I7: screen to world and back moved a point.
    RoundTrip { point: Point, back: Point },
    /// I8: a zoom at an anchor moved the world point under it.
    AnchorMoved { anchor: Point, before: Point, after: Point },
    /// I9: a revealed pane is still not on screen.
    NotRevealed { rect: Rect },
    /// I12: a camera-only action moved or resized a pane.
    PanesMoved,
}

/// How far two floats may differ, relative to their size, and still count as equal.
fn close(a: f32, b: f32) -> bool { (a - b).abs() <= 1e-3 * a.abs().max(b.abs()).max(1.) }
fn close2(a: Point, b: Point) -> bool { close(a[0], b[0]) && close(a[1], b[1]) }

/// I1 and I4 over a document: every pane on the canvas has a finite rect with an area, and the
/// camera is finite with its zoom in `ZOOMS` (what `Document::parse` guarantees on load). A
/// docked pane may keep an empty rect until it first comes back to the canvas.
pub fn document(doc: &Document) -> Result<(), Violation> {
    for p in doc.panes.iter().filter(|p| !doc.docked(&p.id)) {
        let r = p.rect.array();
        if !(r.iter().all(|n| n.is_finite()) && r[2] > 0. && r[3] > 0.) {
            return Err(Violation::BadRect { id: p.id.clone(), rect: r });
        }
    }
    zoom(doc.camera, [ZOOMS[0], ZOOMS[11]])
}

/// I12: a camera-only action left every pane where it was.
pub fn untouched(before: &[Rect], after: &[Rect]) -> Result<(), Violation> {
    if before == after { Ok(()) } else { Err(Violation::PanesMoved) }
}

/// I1 for a resize: `size` is at least `min` on both axes.
pub fn at_least(size: Point, min: Point) -> Result<(), Violation> {
    if size[0] >= min[0] && size[1] >= min[1] { Ok(()) } else { Err(Violation::BelowMin { size, min }) }
}

/// I3: no two of `rects` come closer than `gap` (touching at exactly `gap` is fine).
pub fn apart(rects: &[Rect], gap: f32) -> Result<(), Violation> {
    // A hair of slack: solve places rects at exact float sums.
    let gap = gap - 1e-3;
    for a in 0..rects.len() {
        for b in a + 1..rects.len() {
            let (r, s) = (rects[a], rects[b]);
            let clear = r[0] + r[2] + gap <= s[0] || s[0] + s[2] + gap <= r[0] || r[1] + r[3] + gap <= s[1] || s[1] + s[3] + gap <= r[1];
            if !clear { return Err(Violation::Overlap { a, b }); }
        }
    }
    Ok(())
}

/// I4: the camera is finite and its zoom inside `range` (itself inside `ZOOMS`).
pub fn zoom(camera: Camera, range: [f32; 2]) -> Result<(), Violation> {
    if !camera.is_finite() { return Err(Violation::BadCamera); }
    let z = camera.zoom();
    let ok = z >= range[0].max(ZOOMS[0]) - 1e-6 && z <= range[1].min(ZOOMS[11]) + 1e-6;
    if ok { Ok(()) } else { Err(Violation::Zoom { zoom: z, range }) }
}

/// I7: `to_world(to_screen(p))` is `p`.
pub fn round_trip(camera: Camera, p: Point) -> Result<(), Violation> {
    let back = camera.to_world(camera.to_screen(p));
    if close2(back, p) { Ok(()) } else { Err(Violation::RoundTrip { point: p, back }) }
}

/// I8: the world point under `anchor` is the same before and after a zoom there.
pub fn anchored(before: Camera, after: Camera, anchor: Point) -> Result<(), Violation> {
    let (b, a) = (before.to_world(anchor), after.to_world(anchor));
    if close2(a, b) { Ok(()) } else { Err(Violation::AnchorMoved { anchor, before: b, after: a }) }
}

/// I9: `rect` (world) lies wholly inside the `visible` screen size, when it can fit at all.
pub fn revealed(camera: Camera, rect: Rect, visible: Point) -> Result<(), Violation> {
    let [x, y, w, h] = camera.rect_to_screen(rect);
    let fits = w <= visible[0] && h <= visible[1];
    let slack = 1e-2 * visible[0].max(visible[1]).max(1.);
    let inside = x >= -slack && y >= -slack && x + w <= visible[0] + slack && y + h <= visible[1] + slack;
    if !fits || inside { Ok(()) } else { Err(Violation::NotRevealed { rect }) }
}
