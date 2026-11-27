//! canophy (canopy + physics): the one authority on where panes are, how big they are and how
//! the canvas is looked at. Pure Rust: no GPUI, no clock, no settings beyond what callers pass.
//! docs/canopy/canophy.md explains the laws and how to extend them.
//!
//! # The laws
//!
//! - Law 1. Zoom scales pane content and its header together.
//! - Law 2. Panes the person opens ease the camera to 100%. Panes agents open leave it in place.
//! - Law 3. The Files panel shrinks the visible canvas.
//! - Law 4. Terminal grids reflow on resize, once on release.
//! - Law 5. New file-backed panes use `open_size` to fit their content to the visible canvas at 100%.
//! - Law 7. Rasterized panes use `rest_scale` to draw for the scale at which they will rest.
//!
//! # Step order
//!
//! Every change to the canvas runs these steps in this order, and no step writes something an
//! earlier step read:
//!
//! 1. **apply**: the change itself (a move, a resize, a pan, an opened pane).
//! 2. **size laws**: a pane never shrinks below its kind's `min`, and keeps its `aspect` when it
//!    has one (`traits`).
//! 3. **push**: pushable neighbours make room (`rules::solve`, the canvas push setting).
//! 4. **camera laws**: the zoom stays inside `ZOOMS` and the place's `zoom_range`, and a
//!    viewport change brings the entered pane back into the uncovered part
//!    (`Intent::ViewportChanged`). The `Open` law (a pane the person opens eases the camera to
//!    100%; an agent's pane never moves it) is an ease over time, so the window starts it with
//!    `camera::reveal` and `Motion`, and each frame lands as an `Intent::Camera`.
//! 5. **check**: debug builds run `check::document` after every step. A broken invariant
//!    panics with the violation, the intent that broke it and the last 256 intents (the
//!    replay ring), so the bug can be reproduced by feeding those intents to `step`. Release
//!    builds skip the check and the ring.
//!
//! Every camera and pane-geometry change goes through `step(doc, Intent, &Viewport)`, which runs
//! steps 1, 2, 4 and 5 in that order. Push (step 3) needs each pill's measured size, so the
//! window runs it right after `step` (`solve_layout`), before anything reads the result.
//! Camera motion over time (eases, gestures) is one `Motion` value; time is passed in.
//!
//! # Spaces
//!
//! World px are canvas px at 100% (`units`); screen px are GPUI's `Pixels`. Only `Camera`
//! converts between them, and its fields are private so no caller does zoom math by hand.

pub mod camera;
#[cfg(any(test, debug_assertions))]
pub mod check;
pub mod layout;
pub mod motion;
pub mod open;
pub mod rules;
pub mod step;
pub mod traits;
pub mod units;
#[cfg(test)]
mod tests;

pub use camera::{Camera, focus_fit, reveal, visible, widen, zoom_range};
pub use layout::{Layout, layout, rest_scale};
pub use open::{Natural, open_size};
pub use rules::{arrange, cascade, child_position, free_spot, solve, tile};
pub use motion::Motion;
pub use step::{Effects, Intent, Viewport, derive_pinned, step};
pub use traits::traits;
pub use units::{Point, Rect, WorldRect, WorldSize, intersects, union};
