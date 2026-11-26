//! Layout: where a pane is drawn on screen and at what scale. Render, hit tests, browser masks
//! and wires all read this one answer instead of doing their own zoom math.
//!
//! A pane is a picture of its world rect: the header and the body scale together by `scale`,
//! which is the camera zoom on the canvas and 1 for a pinned pane (it keeps its screen size).
//! Only the 1 px frame is screen-fixed, like the rest of the overlay chrome.
use super::camera::Camera;
use super::traits::traits;
use super::units::{Point, Rect};
use crate::canvas::Pane;

/// The pane frame, in screen px at every zoom.
pub const BORDER: f32 = 1.;

/// One pane as drawn: everything in screen px relative to the canvas's top-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// The whole pane, frame included.
    pub screen: Rect,
    /// The header strip's height (0 for a pane without one).
    pub header: f32,
    /// Inside the frame, under the header: where the content draws.
    pub body: Rect,
    /// Screen px per world px for everything inside the pane.
    pub scale: f32,
    /// The body in world px (frame and header taken off in world px): the same at every zoom,
    /// so content sized from it (a terminal's grid, a page's CSS size) never depends on it.
    pub body_world: Point,
}

impl Layout {
    /// A pane docked in the Files panel: its body fills the panel body (`size`, screen px) at
    /// scale 1, with the panel drawing the header.
    pub fn docked(size: Point) -> Self {
        let body = [0., 0., size[0], size[1]];
        Self { screen: body, header: 0., body, scale: 1., body_world: size }
    }
    /// The body's size on screen.
    pub fn body_size(&self) -> Point { [self.body[2], self.body[3]] }
}

/// Where `pane` draws under `camera`. A pinned pane's world rect is derived from its screen
/// rect (`Document::pin`), so its origin comes from the camera, its size from the pin, and
/// its scale is 1.
pub fn layout(camera: Camera, pane: &Pane) -> Layout {
    let [x, y, ..] = screen(camera, pane, pane.rect.array());
    // The pane's own size, in the units its scale draws: world px, or screen px when pinned.
    let (scale, [w, h]) = match pane.pinned { Some([_, _, w, h]) => (1., [w, h]), None => (camera.zoom(), [pane.rect.w, pane.rect.h]) };
    let header = traits(&pane.kind).header.unwrap_or(0.).min(h);
    let body_world = [(w - 2. * BORDER).max(0.), (h - header - 2. * BORDER).max(0.)];
    let [sw, sh, header] = [w * scale, h * scale, header * scale];
    let body = [x + BORDER, y + header + BORDER, (sw - 2. * BORDER).max(0.), (sh - header - 2. * BORDER).max(0.)];
    Layout { screen: [x, y, sw, sh], header, body, scale, body_world }
}

/// Where world rect `rect` of `pane` (its own, or as drawn: a pill's measured size) lands on
/// screen: under the camera, or at its pinned screen size.
pub fn screen(camera: Camera, pane: &Pane, rect: Rect) -> Rect {
    let [x, y, w, h] = camera.rect_to_screen(rect);
    let [w, h] = pane.pinned.map_or([w, h], |[_, _, w, h]| [w, h]);
    [x, y, w, h]
}

/// A canvas surface that is not a pane (a space card on Home): its screen rect and the scale
/// its contents draw at.
pub fn surface(camera: Camera, rect: Rect) -> (Rect, f32) { (camera.rect_to_screen(rect), camera.zoom()) }
