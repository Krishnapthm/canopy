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

/// A pane kind's declared behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Traits {
    /// The world size a new pane of this kind opens at (a terminal: its collapsed pill).
    pub default: WorldSize,
    /// The smallest size a resize leaves it (a terminal: its expanded size).
    pub min: WorldSize,
    /// Width over height when the shape is locked; None when it is free.
    pub aspect: Option<f32>,
    /// The header strip's height in world px. None when there is no header.
    pub header: Option<f32>,
    /// Whether it pushes and is pushed by neighbours (see `rules::solve`).
    pub pushable: bool,
    /// Whether it opens in the Files panel by default (`canvas.pane_new` without a place).
    pub dockable: bool,
    pub content: Content,
}

/// The standard minimum resize size.
pub(crate) const MIN: WorldSize = WorldSize::new(240., 120.);
/// Where a file or image preview opens, and an image is fitted inside.
pub const PREVIEW: WorldSize = WorldSize::new(560., 420.);
/// Where a browser pane opens, and the size Web Inspector grows it to.
pub const BROWSER: WorldSize = WorldSize::new(960., 680.);
/// A phone in its bezel (about 1:2) under the header.
pub const SIM: WorldSize = WorldSize::new(340., 700.);

/// The declared behavior of a pane of `kind`.
pub fn traits(kind: &Kind) -> Traits {
    let base = Traits { default: PREVIEW, min: MIN, aspect: None, header: Some(HEADER), pushable: true, dockable: false, content: Content::Text };
    match kind {
        // An expanded terminal keeps at least a 1.5-pill-wide, three-header-tall body.
        Kind::Terminal { .. } => Traits { default: WorldSize::new(PILL_WIDTH, HEADER), min: WorldSize::new(PILL_WIDTH * 1.5, HEADER * 3.), content: Content::Grid, ..base },
        Kind::Sub { .. } => Traits { default: WorldSize::new(PILL_WIDTH, HEADER), ..base },
        Kind::File { .. } => base,
        Kind::Image { .. } => Traits { content: Content::Picture, ..base },
        Kind::Pdf { .. } => Traits { default: WorldSize::new(560., 720.), content: Content::Document, ..base },
        Kind::Mod { kind, .. } => {
            let shape = crate::mods::pane_shape(kind).unwrap_or_default();
            Traits { default: shape.size, min: shape.min, header: shape.header.then_some(HEADER), content: Content::Mod, ..base }
        }
        Kind::Browser { .. } => Traits { default: BROWSER, header: Some(BROWSER_HEADER), dockable: true, content: Content::Page, ..base },
        Kind::Sim { .. } => Traits { default: SIM, dockable: true, content: Content::Device, ..base },
    }
}
