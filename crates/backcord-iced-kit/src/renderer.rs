//! Which renderer draws: tiny-skia on the CPU or wgpu on the GPU, both in
//! the one build, chosen once as BACKCORD starts (`choose`), before any
//! picture is made, as each takes its pixels in its own format. The CPU's
//! unless chosen otherwise, as the tests draw.

use std::sync::atomic::{AtomicBool, Ordering};

static GPU: AtomicBool = AtomicBool::new(false);

/// Draws with the GPU, or on the CPU: only that renderer may start when
/// the window opens, so a GPU that can't start is the window's error,
/// not a window drawn on the CPU from pictures made for the GPU.
pub fn choose(gpu: bool) {
    GPU.store(gpu, Ordering::Relaxed);
    iced_wgpu::set_enabled(gpu);
    iced_tiny_skia::set_enabled(!gpu);
}

/// Whether the GPU draws: pictures are then straight RGBA, as wgpu uploads
/// them; on the CPU, premultiplied BGRA, as tiny-skia copies them.
pub fn gpu() -> bool {
    GPU.load(Ordering::Relaxed)
}

/// How round corners are drawn, in hundredths of how they're asked for:
/// 0 square, 100 as asked, up to 200. Circles and pills stay round.
static CORNERS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// A corner's `radius` as Appearance's corner roundness draws it, for what
/// the client cuts itself (a video's corners): scaled, up to `most`.
pub fn corner(radius: f32, most: f32) -> f32 {
    let scale = CORNERS.load(std::sync::atomic::Ordering::Relaxed) as f32 / 100.0;
    (radius * scale).min(most).max(0.0)
}

pub fn set_corners(percent: u32) {
    CORNERS.store(percent.min(200), std::sync::atomic::Ordering::Relaxed);
    iced_wgpu::set_corner_scale(percent);
    iced_tiny_skia::set_corner_scale(percent);
}

static ROUNDS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// How round circles and pills are now, for what the client draws itself
/// (status dots): 0 square, 100 round.
pub fn rounds() -> u32 {
    ROUNDS.load(std::sync::atomic::Ordering::Relaxed)
}

/// A radius that keeps a circle round whatever the roundness sliders say
/// (the renderers' `corners::ALWAYS_ROUND`): an avatar under a decoration,
/// which is drawn for a round one.
pub const ALWAYS_ROUND: f32 = 1.0e5;

/// How round circles and pills are (avatars, dots, server icons), in
/// hundredths: 0 square, 100 round.
pub fn set_rounds(percent: u32) {
    ROUNDS.store(percent.min(100), std::sync::atomic::Ordering::Relaxed);
    iced_wgpu::set_round_scale(percent);
    iced_tiny_skia::set_round_scale(percent);
}
