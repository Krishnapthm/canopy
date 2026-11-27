//! How each pane kind behaves, declared once. The match in `traits` has no catch-all, so a new
//! kind does not compile until it says what it is.
use super::units::WorldSize;
use crate::canvas::{BROWSER_HEADER, HEADER, Kind, PILL_WIDTH};

/// What a pane's body draws, which decides how it follows zoom and resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Content {
    /// A terminal grid of cells: resizing reflows it.
    Grid,
    /// A web page.
    Page,
    /// PDF pages.
    Document,
    /// A device screen (sim).
    Device,
    /// An image.
    Picture,
    /// Plain text or a status line (file previews, sub-agent cells).
    Text,
    /// Whatever a mod draws.
    Mod,
}

/// How a new pane gets its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    /// Use the kind's default size.
    Fixed,
    /// Use the content's own size, passed by the caller.
    Natural,
    /// Fit a text column to the visible canvas.
    Column,
}

/// A pane's shape rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Any width and height.
    Free,
    /// The body keeps its content's width over height. The caller supplies the ratio
    /// because only the running content knows it (a simulator's device and bezel).
    Content,
}

/// A pane kind's declared behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Traits {
    /// The default or fallback world size (a terminal starts as a collapsed pill).
    pub default: WorldSize,
    /// How a new pane of this kind gets its size (see `open::open_size`).
    pub fit: Fit,
    /// The smallest size a resize leaves it (a terminal: its expanded size).
    pub min: WorldSize,
    /// Whether the pane keeps its content's shape when tiled or fitted.
    pub shape: Shape,
    /// The header strip's height in world px. None when there is no header.
    pub header: Option<f32>,
    /// Whether it pushes and is pushed by neighbours (see `rules::solve`).
    pub pushable: bool,
    /// Whether it opens in the Files panel by default (`canvas.pane_new` without a place).
    pub dockable: bool,
    /// Whether the content handles pinch when the person is working in it.
    pub zooms: bool,
    /// Whether the content scrolls with the wheel when the person is working in it.
    pub scrolls: bool,
    pub content: Content,
}

/// The standard minimum resize size.
pub(crate) const MIN: WorldSize = WorldSize::new(240., 120.);
/// The fallback preview size.
pub const PREVIEW: WorldSize = WorldSize::new(560., 420.);
/// The file preview's horizontal padding per side, in world px.
pub(crate) const FILE_PAD_X: f32 = 16.;
/// The width of 80 monospace columns including the preview's horizontal padding.
// Glyphs advance about 0.6 × 13 px. Add the preview padding, code block padding and both frames.
pub const TEXT_COLUMN: f32 = 80. * (0.6 * 13.) + 2. * (FILE_PAD_X + 13. + 1. + super::layout::BORDER);
/// Where a browser pane opens, and the size Web Inspector grows it to.
pub const BROWSER: WorldSize = WorldSize::new(960., 680.);
/// A phone in its bezel (about 1:2) under the header.
pub const SIM: WorldSize = WorldSize::new(340., 700.);

/// The declared behavior of a pane of `kind`.
pub fn traits(kind: &Kind) -> Traits {
    let base = Traits { default: PREVIEW, fit: Fit::Fixed, min: MIN, shape: Shape::Free, header: Some(HEADER), pushable: true, dockable: false, zooms: false, scrolls: false, content: Content::Text };
    match kind {
        // An expanded terminal keeps at least a 1.5-pill-wide, three-header-tall body.
        Kind::Terminal { .. } => Traits { default: WorldSize::new(PILL_WIDTH, HEADER), min: WorldSize::new(PILL_WIDTH * 1.5, HEADER * 3.), scrolls: true, content: Content::Grid, ..base },
        Kind::Sub { .. } => Traits { default: WorldSize::new(PILL_WIDTH, HEADER), ..base },
        Kind::File { .. } => Traits { fit: Fit::Column, scrolls: true, ..base },
        Kind::Image { .. } => Traits { fit: Fit::Natural, content: Content::Picture, ..base },
        Kind::Pdf { .. } => Traits { default: WorldSize::new(560., 720.), fit: Fit::Natural, scrolls: true, content: Content::Document, ..base },
        Kind::Mod { kind, .. } => {
            let shape = crate::mods::pane_shape(kind).unwrap_or_default();
            Traits { default: shape.size, min: shape.min, header: shape.header.then_some(HEADER), content: Content::Mod, ..base }
        }
        Kind::Browser { .. } => Traits { default: BROWSER, header: Some(BROWSER_HEADER), dockable: true, zooms: true, scrolls: true, content: Content::Page, ..base },
        Kind::Sim { .. } => Traits { default: SIM, shape: Shape::Content, dockable: true, content: Content::Device, ..base },
    }
}
