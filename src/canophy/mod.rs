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
//! Phase 1 (this module today) holds the types, the camera, the per-kind traits, the pure
//! placement rules and the invariants. The single `step` entry point that runs the laws in
//! order comes in phase 2; until then callers in the window still call the rules directly.
//!
//! # Spaces
//!
//! World px are canvas px at 100% (`units`); screen px are GPUI's `Pixels`. Only `Camera`
//! converts between them, and its fields are private so no caller does zoom math by hand.

pub mod camera;
// shortcut: tests are its only caller, until phase 2 runs it after every step in debug builds.
#[cfg(test)]
pub mod check;
pub mod rules;
pub mod traits;
pub mod units;
#[cfg(test)]
mod tests;

pub use camera::{Camera, FIT_PAD, focus_fit, reveal, visible, widen, zoom_range};
pub use rules::{arrange, cascade, child_position, free_spot, resize_from, solve, tile};
pub use traits::traits;
pub use units::{Point, Rect, WorldRect, WorldSize, intersects, union};
