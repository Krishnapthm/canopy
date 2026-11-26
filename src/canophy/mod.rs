//! canophy (canopy + physics): the one authority on where panes are, how big they are and how
//! the canvas is looked at. Pure Rust: no GPUI, no clock, no settings beyond what callers pass.
//! See docs/canopy/plans/canophy/plan.md for why it exists.
//!
//! # The laws, in their fixed order
//!
//! Every change to the canvas runs these steps in this order, and no step writes something an
//! earlier step read:
//!
//! 1. **apply**: the change itself (a move, a resize, a pan, an opened pane).
//! 2. **size laws**: a pane never shrinks below its kind's `min`, and keeps its `aspect` when it
//!    has one (`traits`).
//! 3. **push**: pushable neighbours make room (`rules::solve`, the canvas push setting).
//! 4. **camera laws**: the zoom stays inside `ZOOMS` and the place's `zoom_range`, and a pane
//!    the person opens is revealed (`camera`).
//! 5. **check**: in debug builds the invariants in `check` hold.
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
pub mod motion;
pub mod rules;
pub mod step;
pub mod traits;
pub mod units;
#[cfg(test)]
mod tests;

pub use camera::{Camera, focus_fit, reveal, visible, widen, zoom_range};
pub use rules::{arrange, cascade, child_position, free_spot, solve, tile};
pub use motion::Motion;
pub use step::{Effects, Intent, Viewport, derive_pinned, step};
pub use traits::traits;
pub use units::{Point, Rect, WorldRect, WorldSize, intersects, union};
