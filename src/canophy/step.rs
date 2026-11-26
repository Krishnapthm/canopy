//! The one way in: input becomes an `Intent`, and `step` applies it and runs the laws in
//! their fixed order (see the module docs in `mod.rs`).
use super::camera::Camera;
use super::rules::resize_from;
use super::traits::traits;
use super::units::{Point, Rect, WorldRect};
use crate::canvas::{Document, Rest};

/// The canvas as the person sees it: its size and the part no floating chrome covers, both in
/// screen px relative to the canvas's top-left. Every rule that asks "how big is the view"
/// reads this one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub size: Point,
    /// `[x, y, w, h]`: the uncovered part (the toolbar leaves the rest).
    pub open: Rect,
}
impl Viewport {
    /// A viewport with nothing covering it.
    pub fn whole(size: Point) -> Self { Self { size, open: [0., 0., size[0], size[1]] } }
    /// The canvas's center, in screen px.
    pub fn center(&self) -> Point { [self.size[0] / 2., self.size[1] / 2.] }
}

/// One change to the canvas, from the person, an agent or the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// Drag, wheel or momentum: move the view by screen px.
    Pan(Point),
    /// Pinch or a modified wheel: zoom to `zoom` with screen point `anchor` held still, kept
    /// inside `range`.
    ZoomAt { zoom: f32, anchor: Point, range: [f32; 2] },
    /// Put the camera exactly here: one frame of an ease, a restore, or a computed fit.
    Camera(Camera),
    /// Show another place: a space, or Home for None. Each place keeps its own camera.
    Go(Option<String>),
    /// Move panes by `world` px; pinned ones by the same `screen` px instead.
    Move { ids: Vec<String>, world: Point, screen: Point },
    /// Drag one pane's edge or corner (`rules::resize_from`): by `world` px, or `screen` px
    /// for a pinned pane. The pane never shrinks below its kind's minimum.
    Resize { id: String, zone: (i8, i8), world: Point, screen: Point },
    /// Put panes at these rects: arrange, tile, a solved layout, an undo or an opened pane.
    Place(Vec<(String, WorldRect)>),
    /// Undo: put back the geometry of a snapshot (panes and spaces that still exist).
    Restore(Rest),
    /// Collapse: put the panes `moved` made room for back where `base` had them, unless they
    /// moved again since.
    PutBack { base: Rest, moved: Rest },
    /// Set or clear an expanded terminal's size (world px, header included).
    Expand { id: String, size: Option<Point> },
    /// Put a core terminal on the canvas at `point` (or move it there when it already is).
    Adopt { id: String, point: Point, size: Point },
    /// Bring a docked pane back to the canvas, centered on `center` when it never had a rect.
    Undock { id: String, center: Point },
    /// Add sub-agent panes beside their parents and drop the ones that ended (`sub_ids`).
    SyncSubs(Vec<(String, Vec<String>)>),
    /// Pin a pane at a screen rect, or unpin it (None).
    Pin { id: String, screen: Option<Rect> },
    /// The canvas size or its uncovered part changed.
    ViewportChanged,
}

/// What a step changed, so the window knows what to redraw and save.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Effects {
    pub camera: bool,
    pub panes: bool,
}

/// Applies `intent` to `doc`, then runs the laws in order: size, camera, pinned panes, and
/// (debug builds) the invariants. Push (`rules::solve`) runs after this in the window, which
/// knows each pill's measured size. Nothing here reads a clock or settings, so replaying the
/// same intents gives the same document.
pub fn step(doc: &mut Document, intent: Intent, view: &Viewport) -> Effects {
    #[cfg(debug_assertions)]
    ring::record(&intent);
    let before = doc.camera;
    let mut panes = false;
    // 1. apply, with the size law where the intent changes a size.
    match &intent {
        Intent::Pan(d) => doc.camera.pan(*d),
        Intent::ZoomAt { zoom, anchor, range } => doc.camera.zoom_at(zoom.clamp(range[0], range[1]), *anchor),
        Intent::Camera(camera) => doc.camera = *camera,
        Intent::Go(room) => doc.go(room.clone()),
        Intent::Move { ids, world, screen } => {
            for p in doc.panes.iter_mut().filter(|p| ids.contains(&p.id)) {
                match &mut p.pinned {
                    Some(s) => { s[0] += screen[0]; s[1] += screen[1]; }
                    None => { p.rect.x += world[0]; p.rect.y += world[1]; }
                }
                panes = true;
            }
        }
        Intent::Resize { id, zone, world, screen } => {
            if let Some(p) = doc.panes.iter_mut().find(|p| &p.id == id) {
                let min = traits(&p.kind).min.into();
                if let Some(size) = p.size() {
                    let ([x, y], size) = resize_from([p.rect.x, p.rect.y], size, *zone, *world, min);
                    (p.rect.x, p.rect.y) = (x, y);
                    *p.size_mut().unwrap() = Some(size);
                } else {
                    let ([x, y, w, h], d) = p.pinned.map_or((p.rect.array(), *world), |r| (r, *screen));
                    let ([x, y], [w, h]) = resize_from([x, y], [w, h], *zone, d, min);
                    match &mut p.pinned {
                        Some(pinned) => *pinned = [x, y, w, h],
                        None => p.rect = WorldRect::new(x, y, w, h),
                    }
                }
                panes = true;
            }
        }
        Intent::Place(rects) => {
            for (id, rect) in rects {
                if let Some(p) = doc.panes.iter_mut().find(|p| &p.id == id) { p.rect = *rect; panes = true; }
            }
        }
        Intent::Expand { id, size } => {
            if let Some(slot) = doc.panes.iter_mut().find(|p| &p.id == id).and_then(|p| p.size_mut()) { *slot = *size; panes = true; }
        }
        Intent::Restore(rest) => { rest.restore(doc); panes = true; }
        Intent::PutBack { base, moved } => { base.put_back(moved, doc); panes = true; }
        Intent::Adopt { id, point, size } => { doc.place(id, *point, *size); panes = true; }
        Intent::Undock { id, center } => { doc.undock(id, *center); panes = true; }
        Intent::SyncSubs(subs) => panes = doc.sync_subs(subs),
        Intent::Pin { id, screen } => {
            if let Some(p) = doc.panes.iter_mut().find(|p| &p.id == id) { p.pinned = *screen; panes = true; }
        }
        Intent::ViewportChanged => {}
    }
    // Size law: a resized or unpinned pane is at least its kind's minimum on both axes (an
    // unpinned pane's world size was its screen size over the zoom, which can be smaller).
    if let Intent::Resize { id, .. } | Intent::Pin { id, screen: None } = &intent { at_least_min(doc, id); }
    // 2. camera laws: the zoom never leaves `ZOOMS`.
    doc.camera = doc.camera.clamped();
    let camera = doc.camera != before;
    // 3. pinned panes follow the camera and the viewport (their world rect is derived).
    if camera || panes || intent == Intent::ViewportChanged { let c = doc.camera; derive_pinned(doc, c, view); }
    // 4. check.
    #[cfg(debug_assertions)]
    if let Err(v) = super::check::document(doc) {
        panic!("canophy: {v:?} after {intent:?}\nrecent intents:\n{}", ring::dump());
    }
    Effects { camera, panes }
}

/// Grows a boxed pane, or an expanded terminal, to its kind's minimum. Pills (collapsed
/// terminals and sub-agent cells) have their own fixed size and are left alone.
fn at_least_min(doc: &mut Document, id: &str) {
    let Some(p) = doc.panes.iter_mut().find(|p| p.id == id && p.pinned.is_none()) else { return };
    let min = traits(&p.kind).min;
    if let Some(Some(size)) = p.size_mut() {
        *size = [size[0].max(min.w), size[1].max(min.h)];
    } else if p.boxed() {
        (p.rect.w, p.rect.h) = (p.rect.w.max(min.w), p.rect.h.max(min.h));
    }
}

/// Puts each pinned pane's world rect under its screen rect, kept inside the viewport (see
/// `Document::pin`). A copy of the document drawn mid-animation needs the same, at the live
/// document's camera.
pub fn derive_pinned(doc: &mut Document, camera: Camera, view: &Viewport) {
    doc.pin(camera, view.size);
}

/// Debug builds keep the last 256 intents, so a broken invariant prints how to replay it.
#[cfg(debug_assertions)]
mod ring {
    use super::Intent;
    use std::{cell::RefCell, collections::VecDeque};
    const LEN: usize = 256;
    thread_local! { static RECENT: RefCell<VecDeque<String>> = RefCell::new(VecDeque::with_capacity(LEN)); }
    pub(super) fn record(intent: &Intent) {
        RECENT.with_borrow_mut(|r| {
            if r.len() == LEN { r.pop_front(); }
            r.push_back(format!("{intent:?}"));
        });
    }
    pub(super) fn dump() -> String { RECENT.with_borrow(|r| r.iter().cloned().collect::<Vec<_>>().join("\n")) }
}
