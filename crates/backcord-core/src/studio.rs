//! Studio mode: the Discord microphone goes out with none of BACKCORD's
//! processing (echo cancellation, noise suppression, gain control, leveling,
//! room-echo removal) and Opus at 320 kbit/s, stereo, fullband.

/// Turns studio mode on or off for every call; a call going on takes it at
/// its next block.
pub fn set(on: bool) {
    #[cfg(feature = "discord")]
    backcord_discord::opus::set_studio(on);
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn on() -> bool {
    ON.load(std::sync::atomic::Ordering::Relaxed)
}

static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
