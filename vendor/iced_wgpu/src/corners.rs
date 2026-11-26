//! BACKCORD port: Appearance's corner roundness, which scales every
//! quad's and picture's corners, in one place for the whole window. Two
//! scales: one for corners (panels, buttons, cards), and one for circles
//! and pills (corners at least half the short side: avatars, dots,
//! switches, server icons), so each can be set apart. Corners stay within
//! half the short side.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::core::{Rectangle, border::Radius};

/// The corners' scale in hundredths: 100 leaves them as drawn.
static SCALE: AtomicU32 = AtomicU32::new(100);
/// The circles' and pills' scale in hundredths: 100 round, 0 square.
static ROUND: AtomicU32 = AtomicU32::new(100);

/// Sets how round corners are, in hundredths of how they're drawn: 0 for
/// square, 100 as drawn, up to 200.
pub fn set_corner_scale(percent: u32) {
    SCALE.store(percent.min(200), Ordering::Relaxed);
}

/// Sets how round circles and pills are: 0 square, 100 round.
pub fn set_round_scale(percent: u32) {
    ROUND.store(percent.min(100), Ordering::Relaxed);
}

/// A radius at least this on every corner is a circle that stays one
/// whatever the scales say: an avatar a decoration is drawn round.
pub const ALWAYS_ROUND: f32 = 1.0e5;

/// `radius` for a quad or picture of `bounds`, scaled.
pub(crate) fn corners(radius: Radius, bounds: Rectangle) -> Radius {
    let half = bounds.width.min(bounds.height) / 2.0;
    let radii = <[f32; 4]>::from(radius);
    if radii.iter().all(|radius| *radius >= ALWAYS_ROUND) {
        return Radius::from(half.max(0.0));
    }
    let scale = SCALE.load(Ordering::Relaxed);
    let round = ROUND.load(Ordering::Relaxed);
    if scale == 100 && round == 100 {
        return radius;
    }
    let circle = radii.iter().all(|radius| *radius >= half - 0.5);
    let scale = if circle {
        if round == 100 {
            return radius;
        }
        round as f32 / 100.0
    } else {
        if scale == 100 {
            return radius;
        }
        scale as f32 / 100.0
    };
    let [a, b, c, d] = radii.map(|radius| (radius.min(half) * scale).min(half).max(0.0));
    Radius {
        top_left: a,
        top_right: b,
        bottom_right: c,
        bottom_left: d,
    }
}
