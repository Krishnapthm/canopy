//! The size a new pane opens at, before the camera moves.
use super::layout::BORDER;
use super::step::Viewport;
use super::traits::{Fit, TEXT_COLUMN, Traits};
use super::units::WorldSize;

/// Natural content and its fixed extra chrome, both in world px at 100%.
pub struct Natural {
    pub content: WorldSize,
    pub chrome: WorldSize,
}

/// Margin per side between a new pane and the uncovered canvas edge, in px at 100%.
pub const OPEN_MARGIN: f32 = 32.;

/// Size a new pane at 100% (law 5). Shrink natural content uniformly, then add its
/// chrome, the header and the frame. The kind's minimum takes priority if space runs out.
/// Fixed panes and natural panes without dimensions use the kind's default.
pub fn open_size(t: &Traits, natural: Option<Natural>, view: &Viewport) -> WorldSize {
    let [_, _, w, h] = view.open;
    let (w, h) = (w - 2. * OPEN_MARGIN, h - 2. * OPEN_MARGIN);
    let (w, h) = match t.fit {
        Fit::Fixed => return t.default,
        Fit::Column => (TEXT_COLUMN.min(w), h),
        Fit::Natural => {
            let Some(Natural { content, chrome }) = natural else {
                return t.default;
            };
            let chrome_w = chrome.w + 2. * BORDER;
            let chrome_h = chrome.h + 2. * BORDER + t.header.unwrap_or(0.);
            let scale = ((w - chrome_w) / content.w)
                .min((h - chrome_h) / content.h)
                .clamp(0., 1.);
            (content.w * scale + chrome_w, content.h * scale + chrome_h)
        }
    };
    WorldSize::new(w.max(t.min.w), h.max(t.min.h))
}
