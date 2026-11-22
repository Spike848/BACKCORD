//! Studio mode: the Discord microphone goes out at Opus's best: 320 kbit/s,
//! complexity 10, fullband, in its music mode. Noise suppression and echo
//! reduction stay as chosen, so a noisy or echoey room can have studio mode
//! too; both off, the microphone goes out as it came, at its volume.

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
