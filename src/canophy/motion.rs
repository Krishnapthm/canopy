//! The camera's one motion state. Before it, "has the camera come to rest?" was answered by
//! three separate checks; now `settled` answers it.
use super::camera::Camera;
use super::units::Point;
use std::time::{Duration, Instant};

/// How long a camera ease takes.
pub const EASE: Duration = Duration::from_millis(180);
/// How long a zoom gesture must be still before it counts as over.
pub const GESTURE_REST: Duration = Duration::from_millis(160);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Motion {
    /// Nothing is moving the camera.
    #[default]
    Rested,
    /// A pinch or modified wheel moved the zoom at `since`; it rests `GESTURE_REST` later.
    Gesture { since: Instant },
    /// The camera glides from `from` to `to`, keeping screen point `anchor` on its path.
    Easing { from: Camera, to: Camera, start: Instant, anchor: Point },
}

impl Motion {
    pub fn ease(from: Camera, to: Camera, anchor: Point, now: Instant) -> Self { Self::Easing { from, to, start: now, anchor } }
    /// Whether terminal grids and PDF pages may refit to the zoom (nothing is moving it).
    pub fn settled(&self, now: Instant) -> bool {
        match self {
            Self::Rested => true,
            Self::Gesture { since } => now.saturating_duration_since(*since) >= GESTURE_REST,
            Self::Easing { .. } => false,
        }
    }
    /// Where an ease is headed.
    pub fn target(&self) -> Option<Camera> {
        match self { Self::Easing { to, .. } => Some(*to), Self::Rested | Self::Gesture { .. } => None }
    }
    /// The camera for this frame of an ease, and whether the ease goes on. None when not easing.
    pub fn frame(&self, now: Instant) -> Option<(Camera, bool)> {
        let Self::Easing { from, to, start, anchor } = *self else { return None };
        let t = now.saturating_duration_since(start).as_secs_f32() / EASE.as_secs_f32();
        if t >= 1. { return Some((to, false)); }
        let a = ease_out(t);
        let zoom = from.zoom() + (to.zoom() - from.zoom()) * a;
        let (f, w) = (from.to_world(anchor), to.to_world(anchor));
        Some((Camera::anchored([f[0] + (w[0] - f[0]) * a, f[1] + (w[1] - f[1]) * a], anchor, zoom), true))
    }
}

/// Cubic ease-out over `t` in 0..=1.
pub fn ease_out(t: f32) -> f32 { 1. - (1. - t.clamp(0., 1.)).powi(3) }
