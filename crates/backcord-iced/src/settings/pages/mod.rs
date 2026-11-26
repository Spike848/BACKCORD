//! Settings' pages of switches and choices: My account, Messages,
//! Notifications, Sounds, Advanced, About, Discord and Game Activity; and
//! what their preferences do when changed.

use backcord_core::{application::Action, sounds::Sound};
use iced::Element;

use super::{Event, Page, Settings};
use crate::{
    app::Message,
    ctx::{Cx, Pref as Shown, Request},
    form::switch,
    layout::Animate,
};

mod about;
mod account;
pub(super) use account::BLOCKED_STEP;
mod advanced;
mod discord;
pub use discord::blocked_pictures;
mod extras;
pub use extras::{Extra, auto_delete, emulate_platform, fake_nitro, relationship_notifier};
mod games;
mod messages;
mod notifications;
mod security;
pub use security::network;

/// The choices of Messages' animations, of the date's order, of the sound
/// set and of the text's size, in the order they show.
const ANIMATES: [Animate; 3] = [Animate::Focused, Animate::Always, Animate::Never];

/// A preference changed, by a switch, a choice or a slider (with whether
/// the slider was let go, when it's kept).
#[derive(Clone, Debug)]
pub enum Pref {
    TwelveHour(bool),
    Seconds(bool),
    DateFormat(usize),
    TextSize(usize),
    Animate(usize),
    LinkPreviews(bool),
    CollapseLong(bool),
    BoostGems(bool),
    ShareTyping(bool),
    ReplyPing(bool),
    StripTracking(bool),
    Notifications(bool),
    /// A preset of sounds chosen: 0 Discord's, then those made.
    SoundPreset(usize),
    NewPreset,
    DeletePreset,
    PresetName(String),
    SoundVolume(u32, bool),
    EachSound(usize, u32, bool),
    PlaySound(usize),
    /// The file chosen for a sound, by its place in `Sound::ALL`, let go:
    /// it plays Discord's again.
    ClearSound(usize),
    GpuRendering(bool),
    GpuEncoding(bool),
    CloseToTray(bool),
    MultipleWindows(bool),
    DeveloperMode(bool),
    ShowHiddenChannels(bool),
    SyncReads(bool),
    HideBlocked(bool),
    CheckUpdates(bool),
    Nightlies(bool),
    StartOnSignIn(bool),
    SystemTitleBar(bool),
    /// The neon glow's slider, 0 to 100, and whether it's let go of.
    NeonGlow(u32, bool),
    /// The corners' roundness slider, 0 to 200, and whether let go of.
    Corners(u32, bool),
    /// Circles' and pills' roundness, 0 to 100, and whether let go of.
    Round(u32, bool),
    /// The Dynamic Island: on, its style (0 glass, 1 OLED), its sliders
    /// (and whether let go of), its glow's and text's colours.
    Island(bool),
    IslandStyle(usize),
    IslandOpacity(u32, bool),
    IslandGlow(u32, bool),
    IslandSpeed(u32, bool),
    IslandRadius(u32, bool),
    IslandBorder(u32, bool),
    IslandColor(Option<u32>),
    IslandInk(Option<u32>),
    HideOthersTyping(bool),
    NoPushToTalk(bool),
    MessageLogger(bool),
    SplitLongMessages(bool),
    MessageLoggerOwn(bool),
    PlatformIndicators(bool),
    EmulatedPlatform(usize),
    EmulationDepth(usize),
    AntiDisconnect(bool),
    FollowVoice(bool),
    AutoUnmute(bool),
    KeepNickname(bool),
    AntiGroupCall(bool),
    AutoDelete(bool),
    /// How long after sending Auto Delete deletes, by its place in its
    /// dialog's times.
    AutoDeleteAfter(usize),
    MessageLinkPreviews(bool),
    RelationshipNotifier(bool),
    /// One of Relationship Notifier's removals, by its place in its dialog.
    RelationshipKind(usize, bool),
    FakeNitro(bool),
    /// The size Fake Nitro's emoji links ask for, by its place in
    /// `backcord_core::fake_nitro::SIZES`.
    FakeNitroSize(usize),
    FakeNitroMasked(bool),
    FakeNitroReceived(bool),
    BadgeSpoofer(bool),
    /// A badge of `backcord_core::badge_spoofer::BADGES` ticked or not, by
    /// its place.
    SpoofedBadge(usize, bool),
    /// The house, Nitro level and boost level Badge Spoofer shows: none for
    /// 0, else the one after it in their lists.
    SpoofedHouse(usize),
    SpoofedNitro(usize),
    SpoofedBoost(usize),
    SpoofRealBadges(bool),
    ProxyOn(bool),
    Proxy(String),
    Dns(usize),
    DnsCustom(String),
    ClickableRoles(bool),
    PermissionsViewer(bool),
    ViewIcons(bool),
    IconFormat(usize),
    IconSize(usize),
    StreamProof(bool),
    StreamProofLocation(usize),
    StreamProofBlackout(bool),
    StreamProofBlur(bool),
    StreamProofAuto(bool),
}

/// My account's and the Discord page's buttons.
#[derive(Clone, Debug)]
pub enum AccountEvent {
    Reconnect,
    CustomStatus,
    SignOut,
    Unlink,
    Unblock(String),
    MoreBlocked,
    Open(&'static str),
}

/// Game Activity's draft: the executable and name being written, the game
/// being edited (by its place), and whether the last save was refused.
#[derive(Default)]
pub struct GameDraft {
    pub executable: String,
    pub name: String,
    pub editing: Option<usize>,
    pub invalid: bool,
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    Show(bool),
    Executable(String),
    Name(String),
    Edit(usize),
    Remove(usize),
    Save,
    Cancel,
}

fn pref(pref: Pref) -> Message {
    Message::Settings(Event::Pref(pref))
}

impl Settings {
    pub(in crate::settings) fn pref(&mut self, pref: Pref, cx: &mut Cx<'_>) {
        let prefs = &mut cx.layout.prefs;
        match pref {
            Pref::TwelveHour(on) => {
                prefs.twelve_hour_clock = on;
                prefs.apply_clock();
                cx.keep("twelve_hour_clock", on);
                cx.request(Request::Changed(Shown::Clock));
            }
            Pref::Seconds(on) => {
                prefs.clock_seconds = on;
                cx.keep("clock_seconds", on);
                cx.request(Request::Changed(Shown::Clock));
            }
            Pref::DateFormat(index) => {
                prefs.date_format = Some(index as i32);
                prefs.apply_clock();
                cx.keep("date_format", index);
                cx.request(Request::Changed(Shown::Clock));
            }
            Pref::TextSize(index) => {
                prefs.text_size = Some(index as i32);
                cx.keep("text_size", index);
                cx.request(Request::Changed(Shown::Rows));
            }
            Pref::Animate(index) => {
                let animate = ANIMATES[index.min(2)];
                prefs.animate = animate;
                cx.keep("animate", animate);
            }
            Pref::LinkPreviews(on) => {
                prefs.link_previews = Some(on);
                cx.keep("link_previews", on);
                cx.request(Request::Changed(Shown::Rows));
            }
            Pref::CollapseLong(on) => {
                cx.layout.collapse_long = on;
                cx.keep("collapse_long", on);
                cx.request(Request::Changed(Shown::Rows));
            }
            Pref::BoostGems(on) => {
                prefs.boost_gems = Some(on);
                cx.keep("boost_gems", on);
                cx.request(Request::Changed(Shown::Roster));
            }
            Pref::ShareTyping(on) => {
                prefs.share_typing = Some(on);
                cx.client.share_typing = on;
                cx.keep("share_typing", on);
            }
            Pref::ReplyPing(on) => {
                prefs.reply_ping = Some(on);
                cx.keep("reply_ping", on);
                cx.dispatch(Action::ReplyPing(on));
            }
            Pref::StripTracking(on) => {
                prefs.strip_tracking = Some(on);
                cx.keep("strip_tracking", on);
                cx.dispatch(Action::StripTracking(on));
            }
            Pref::Notifications(on) => {
                prefs.notifications = Some(on);
                cx.keep("notifications", on);
            }
            Pref::SoundPreset(index) => {
                let voice = &mut cx.layout.voice;
                voice.sound_preset = index
                    .checked_sub(1)
                    .filter(|&preset| preset < voice.sound_presets.len());
                cx.keep("sound_preset", cx.layout.voice.sound_preset);
                try_sound(cx, Sound::Join);
            }
            Pref::NewPreset => {
                new_preset(cx);
            }
            Pref::DeletePreset => {
                let voice = &mut cx.layout.voice;
                if let Some(index) = voice.sound_preset.take()
                    && index < voice.sound_presets.len()
                {
                    voice.sound_presets.remove(index);
                }
                keep_presets(cx);
            }
            Pref::PresetName(name) => {
                let voice = &mut cx.layout.voice;
                if let Some(preset) = voice
                    .sound_preset
                    .and_then(|index| voice.sound_presets.get_mut(index))
                {
                    preset.name = name.chars().take(PRESET_NAME).collect();
                    cx.keep("sound_presets", &cx.layout.voice.sound_presets);
                }
            }
            Pref::ClearSound(index) => {
                let voice = &mut cx.layout.voice;
                if let Some(&sound) = Sound::ALL.get(index)
                    && let Some(preset) = voice
                        .sound_preset
                        .and_then(|index| voice.sound_presets.get_mut(index))
                {
                    preset.files.remove(&sound);
                    keep_presets(cx);
                    try_sound(cx, sound);
                }
            }
            Pref::SoundVolume(value, done) => {
                cx.layout.voice.sound_volume = Some(value.min(200) as u8);
                if done {
                    cx.keep("sound_volume", value.min(200));
                    try_sound(cx, Sound::Message);
                }
            }
            Pref::EachSound(index, value, done) => {
                let Some(&sound) = Sound::ALL.get(index) else {
                    return;
                };
                let each = &mut cx.layout.voice.each_sound;
                if value >= 100 {
                    each.remove(&sound);
                } else {
                    each.insert(sound, value as u8);
                }
                if done {
                    cx.keep("each_sound", &cx.layout.voice.each_sound);
                    try_sound(cx, sound);
                }
            }
            Pref::PlaySound(index) => {
                if let Some(&sound) = Sound::ALL.get(index) {
                    try_sound(cx, sound);
                }
            }
            Pref::GpuRendering(on) => {
                prefs.gpu_rendering = Some(on);
                cx.keep("gpu_rendering", on);
            }
            Pref::GpuEncoding(on) => {
                cx.layout.voice.gpu_encoding = Some(on);
                backcord_core::stream_worker::set_hardware_encoding(on);
                cx.keep("gpu_encoding", on);
            }
            Pref::CloseToTray(on) => {
                prefs.close_to_tray = Some(on);
                cx.keep("close_to_tray", on);
            }
            Pref::MultipleWindows(on) => {
                prefs.multiple_windows = on;
                cx.keep("multiple_windows", on);
            }
            Pref::DeveloperMode(on) => {
                cx.layout.developer_mode = on;
                cx.keep("developer_mode", on);
            }
            Pref::ShowHiddenChannels(on) => {
                prefs.show_hidden_channels = on;
                cx.keep("show_hidden_channels", on);
                cx.dispatch(Action::ShowHiddenChannels(on));
            }
            Pref::SyncReads(on) => {
                prefs.sync_discord_reads = Some(on);
                cx.keep("sync_discord_reads", on);
                cx.dispatch(Action::SyncDiscordReads(on));
            }
            Pref::HideBlocked(on) => {
                cx.layout.hide_blocked = on;
                cx.keep("hide_blocked", on);
                cx.dispatch(Action::HideBlocked(on));
            }
            Pref::CheckUpdates(on) => {
                prefs.check_for_updates = Some(on);
                cx.keep("check_for_updates", on);
                if on {
                    cx.request(Request::LookForUpdates(true));
                }
            }
            Pref::Nightlies(on) => {
                prefs.update_nightlies = on;
                cx.keep("update_nightlies", on);
            }
            Pref::StartOnSignIn(on) => {
                match crate::desktop::desktop().set_autostart(on) {
                    Ok(()) => tracing::info!(start = on, "starting at sign-in changed"),
                    Err(error) => tracing::warn!(%error, "could not change starting at sign-in"),
                }
                self.read_platform_choices();
            }
            Pref::SystemTitleBar(on) => {
                cx.layout.system_title_bar = Some(on);
                cx.keep("system_title_bar", on);
                cx.request(Request::TitleBar(on));
            }
            Pref::NeonGlow(value, done) => {
                let percent = value.min(100) as u8;
                prefs.neon_glow = Some(percent);
                backcord_iced_kit::style::set_glow(percent);
                if done {
                    cx.keep("neon_glow", percent);
                }
            }
            Pref::Corners(value, done) => {
                let percent = value.min(200) as u8;
                prefs.corners = Some(percent);
                backcord_iced_kit::renderer::set_corners(u32::from(percent));
                if done {
                    cx.keep("corners", percent);
                }
            }
            Pref::Round(value, done) => {
                let percent = value.min(100) as u8;
                prefs.round = Some(percent);
                backcord_iced_kit::renderer::set_rounds(u32::from(percent));
                if done {
                    cx.keep("round", percent);
                }
            }
            Pref::Island(on) => {
                prefs.island = Some(on);
                cx.keep("island", on);
            }
            Pref::IslandStyle(style) => {
                let style = style.min(1) as u8;
                prefs.island_style = Some(style);
                cx.keep("island_style", style);
            }
            Pref::IslandOpacity(value, done) => {
                let value = value.min(100) as u8;
                prefs.island_opacity = Some(value);
                if done {
                    cx.keep("island_opacity", value);
                }
            }
            Pref::IslandGlow(value, done) => {
                let value = value.min(100) as u8;
                prefs.island_glow = Some(value);
                if done {
                    cx.keep("island_glow", value);
                }
            }
            Pref::IslandSpeed(value, done) => {
                let value = value.min(100) as u8;
                prefs.island_speed = Some(value);
                if done {
                    cx.keep("island_speed", value);
                }
            }
            Pref::IslandRadius(value, done) => {
                let value = value.min(22) as u8;
                prefs.island_radius = Some(value);
                if done {
                    cx.keep("island_radius", value);
                }
            }
            Pref::IslandBorder(value, done) => {
                let value = value.min(4) as u8;
                prefs.island_border = Some(value);
                if done {
                    cx.keep("island_border", value);
                }
            }
            Pref::IslandColor(colour) => {
                prefs.island_color = colour;
                cx.keep("island_color", colour);
            }
            Pref::IslandInk(colour) => {
                prefs.island_ink = colour;
                cx.keep("island_ink", colour);
            }
            Pref::HideOthersTyping(on) => {
                prefs.hide_others_typing = Some(on);
                cx.keep("hide_others_typing", on);
            }
            Pref::StreamProof(on) => {
                prefs.stream_proof = Some(on);
                cx.keep("stream_proof", on);
                cx.request(Request::StreamProof);
            }
            Pref::StreamProofLocation(index) => {
                prefs.stream_proof_location = Some(index);
                cx.keep("stream_proof_location", index);
            }
            Pref::StreamProofBlackout(on) => {
                prefs.stream_proof_blackout = Some(on);
                cx.keep("stream_proof_blackout", on);
                cx.request(Request::StreamProof);
            }
            Pref::StreamProofBlur(on) => {
                prefs.stream_proof_blur = Some(on);
                cx.keep("stream_proof_blur", on);
                cx.request(Request::StreamProof);
            }
            Pref::StreamProofAuto(on) => {
                prefs.stream_proof_auto = Some(on);
                cx.keep("stream_proof_auto", on);
            }
            Pref::ViewIcons(on) => {
                prefs.view_icons = Some(on);
                cx.keep("view_icons", on);
            }
            Pref::IconFormat(index) => {
                prefs.icon_format = Some(index);
                cx.keep("icon_format", index);
            }
            Pref::IconSize(index) => {
                prefs.icon_size = Some(index);
                cx.keep("icon_size", index);
            }
            Pref::ClickableRoles(on) => {
                prefs.clickable_roles = Some(on);
                cx.keep("clickable_roles", on);
            }
            Pref::PermissionsViewer(on) => {
                prefs.permissions_viewer = Some(on);
                cx.keep("permissions_viewer", on);
            }
            Pref::SplitLongMessages(on) => {
                prefs.split_long_messages = Some(on);
                backcord_core::extras::set_split_long(on);
                cx.keep("split_long_messages", on);
            }
            Pref::MessageLogger(on) => {
                prefs.message_logger = Some(on);
                backcord_core::extras::set_message_logger(on);
                cx.keep("message_logger", on);
            }
            Pref::MessageLoggerOwn(on) => {
                prefs.message_logger_own = Some(on);
                backcord_core::extras::set_log_own(on);
                cx.keep("message_logger_own", on);
            }
            Pref::PlatformIndicators(on) => {
                prefs.platform_indicators = Some(on);
                cx.keep("platform_indicators", on);
            }
            Pref::EmulatedPlatform(index) => {
                prefs.emulated_platform = Some(index);
                crate::settings::emulate_platform(prefs);
                cx.keep("emulated_platform", index);
            }
            Pref::EmulationDepth(index) => {
                prefs.emulation_depth = Some(index);
                crate::settings::emulate_platform(prefs);
                cx.keep("emulation_depth", index);
            }
            Pref::AntiDisconnect(on) => {
                prefs.anti_disconnect = Some(on);
                cx.dispatch(backcord_core::application::Action::AntiDisconnect(on));
                cx.keep("anti_disconnect", on);
            }
            Pref::AutoUnmute(on) => {
                prefs.auto_unmute = Some(on);
                cx.dispatch(backcord_core::application::Action::AutoUnmute(on));
                cx.keep("auto_unmute", on);
            }
            Pref::KeepNickname(on) => {
                prefs.keep_nickname = Some(on);
                backcord_core::extras::set_keep_nickname(on);
                cx.keep("keep_nickname", on);
            }
            Pref::AntiGroupCall(on) => {
                prefs.anti_group_call = Some(on);
                cx.dispatch(backcord_core::application::Action::AntiGroupCall(on));
                cx.keep("anti_group_call", on);
            }
            Pref::AutoDelete(on) => {
                prefs.auto_delete = Some(on);
                crate::settings::auto_delete(prefs);
                cx.keep("auto_delete", on);
            }
            Pref::AutoDeleteAfter(index) => {
                prefs.auto_delete_after = Some(index);
                crate::settings::auto_delete(prefs);
                cx.keep("auto_delete_after", index);
            }
            Pref::MessageLinkPreviews(on) => {
                prefs.message_link_previews = Some(on);
                cx.dispatch(backcord_core::application::Action::HideMessageLinks(!on));
                cx.keep("message_link_previews", on);
            }
            Pref::RelationshipNotifier(on) => {
                prefs.relationship_notifier = Some(on);
                crate::settings::relationship_notifier(prefs);
                cx.keep("relationship_notifier", on);
            }
            Pref::RelationshipKind(index, on) => {
                let (switch, key) = match index {
                    0 => (&mut prefs.relationship_friends, "relationship_friends"),
                    1 => (&mut prefs.relationship_requests, "relationship_requests"),
                    2 => (&mut prefs.relationship_groups, "relationship_groups"),
                    3 => (&mut prefs.relationship_servers, "relationship_servers"),
                    _ => (&mut prefs.relationship_offline, "relationship_offline"),
                };
                *switch = Some(on);
                crate::settings::relationship_notifier(prefs);
                cx.keep(key, on);
            }
            Pref::FakeNitro(on) => {
                prefs.fake_nitro = Some(on);
                crate::settings::fake_nitro(prefs);
                cx.keep("fake_nitro", on);
            }
            Pref::FakeNitroSize(index) => {
                prefs.fake_nitro_size = Some(index);
                crate::settings::fake_nitro(prefs);
                cx.keep("fake_nitro_size", index);
            }
            Pref::FakeNitroMasked(on) => {
                prefs.fake_nitro_masked = Some(on);
                crate::settings::fake_nitro(prefs);
                cx.keep("fake_nitro_masked", on);
            }
            Pref::FakeNitroReceived(on) => {
                prefs.fake_nitro_received = Some(on);
                crate::settings::fake_nitro(prefs);
                cx.keep("fake_nitro_received", on);
            }
            Pref::BadgeSpoofer(on) => {
                prefs.badge_spoofer = Some(on);
                let settings = prefs.badge_spoofer();
                cx.dispatch(Action::SpoofBadges(settings));
                cx.keep("badge_spoofer", on);
            }
            Pref::SpoofedBadge(index, on) => {
                let badges = prefs.badge_spoofer().with(index, on).badges;
                prefs.badge_spoofer_badges = Some(badges);
                let settings = prefs.badge_spoofer();
                cx.dispatch(Action::SpoofBadges(settings));
                cx.keep("badge_spoofer_badges", badges);
            }
            Pref::SpoofedHouse(index) | Pref::SpoofedNitro(index) | Pref::SpoofedBoost(index) => {
                let (chosen, key) = match pref {
                    Pref::SpoofedHouse(_) => {
                        (&mut prefs.badge_spoofer_house, "badge_spoofer_house")
                    }
                    Pref::SpoofedNitro(_) => {
                        (&mut prefs.badge_spoofer_nitro, "badge_spoofer_nitro")
                    }
                    _ => (&mut prefs.badge_spoofer_boost, "badge_spoofer_boost"),
                };
                *chosen = Some(index);
                let settings = prefs.badge_spoofer();
                cx.dispatch(Action::SpoofBadges(settings));
                cx.keep(key, index);
            }
            Pref::SpoofRealBadges(on) => {
                prefs.badge_spoofer_real = Some(on);
                let settings = prefs.badge_spoofer();
                cx.dispatch(Action::SpoofBadges(settings));
                cx.keep("badge_spoofer_real", on);
            }
            Pref::FollowVoice(on) => {
                prefs.follow_voice = Some(on);
                if !on {
                    cx.dispatch(backcord_core::application::Action::FollowVoice(None));
                }
                cx.keep("follow_voice", on);
            }
            Pref::ProxyOn(on) => {
                prefs.proxy_on = Some(on);
                cx.keep("proxy_on", on);
            }
            Pref::Proxy(address) => {
                prefs.proxy = Some(address.clone());
                cx.keep("proxy", address);
            }
            Pref::Dns(index) => {
                prefs.dns = Some(index);
                cx.keep("dns", index);
            }
            Pref::DnsCustom(server) => {
                prefs.dns_custom = Some(server.clone());
                cx.keep("dns_custom", server);
            }
            Pref::NoPushToTalk(on) => {
                prefs.no_push_to_talk = Some(on);
                backcord_core::push_to_talk::set_ignore_required(on);
                cx.keep("no_push_to_talk", on);
            }
        }
        cx.touch();
    }
}

/// A preset's name, at its longest.
pub(super) const PRESET_NAME: usize = 40;

/// A new preset, a copy of the one playing (empty for Discord's), named
/// after its place, which then plays. Its place.
fn new_preset(cx: &mut Cx<'_>) -> usize {
    let voice = &mut cx.layout.voice;
    let files = voice
        .preset()
        .map(|preset| preset.files.clone())
        .unwrap_or_default();
    let name = (voice.sound_presets.len() + 1..)
        .map(|number| format!("Preset {number}"))
        .find(|name| {
            voice
                .sound_presets
                .iter()
                .all(|preset| &preset.name != name)
        })
        .unwrap_or_default();
    voice
        .sound_presets
        .push(crate::layout::SoundPreset { name, files });
    let index = voice.sound_presets.len() - 1;
    voice.sound_preset = Some(index);
    keep_presets(cx);
    index
}

/// Keeps the presets and the one playing, and lets go of the files no
/// preset chooses any more.
fn keep_presets(cx: &Cx<'_>) {
    let voice = &cx.layout.voice;
    cx.keep("sound_presets", &voice.sound_presets);
    cx.keep("sound_preset", voice.sound_preset);
    let folder = cx.client.root().join(backcord_core::sounds::FOLDER);
    let Ok(files) = std::fs::read_dir(&folder) else {
        return;
    };
    for file in files.flatten() {
        let name = file.file_name();
        let name = name.to_string_lossy();
        let chosen = voice
            .sound_presets
            .iter()
            .any(|preset| preset.files.values().any(|kept| kept.file == name));
        if name.starts_with(CHOSEN)
            && !chosen
            && let Err(error) = std::fs::remove_file(file.path())
        {
            tracing::warn!(%error, "could not delete a sound no preset chooses");
        }
    }
}

/// How the copies of chosen files start their names in the sounds' folder.
const CHOSEN: &str = "chosen-";

/// Copies the file at `path` into the data folder `root`'s sounds, once
/// it's a sound BACKCORD can play of at most [`LARGEST_FILE`]. Off the
/// window's thread: it reads and decodes the file.
///
/// [`LARGEST_FILE`]: backcord_core::sounds::LARGEST_FILE
pub fn copy_sound(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<crate::layout::ChosenSound, String> {
    use backcord_iced_kit::i18n::tr;
    let large = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .len();
    if large > backcord_core::sounds::LARGEST_FILE as u64 {
        return Err(tr("That file is over 4 MB.").to_owned());
    }
    let data = std::fs::read(path).map_err(|error| error.to_string())?;
    backcord_core::sounds::check(&data)
        .map_err(|_| tr("That file isn't a sound BACKCORD can play.").to_owned())?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| extension.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("mp3")
        .to_ascii_lowercase();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file = format!("{CHOSEN}{stamp}.{extension}");
    let folder = root.join(backcord_core::sounds::FOLDER);
    std::fs::create_dir_all(&folder)
        .and_then(|()| std::fs::write(folder.join(&file), &data))
        .map_err(|error| error.to_string())?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.clone());
    Ok(crate::layout::ChosenSound { file, name })
}

impl Settings {
    /// A file chosen for the sound at `index` in `Sound::ALL`, copied in:
    /// it goes in the preset playing, or in a new one in place of
    /// Discord's.
    pub(in crate::settings) fn sound_picked(
        &mut self,
        index: usize,
        picked: Result<crate::layout::ChosenSound, String>,
        cx: &mut Cx<'_>,
    ) {
        let Some(&sound) = Sound::ALL.get(index) else {
            return;
        };
        match picked {
            Err(why) => self.sound_note = Some(why),
            Ok(chosen) => {
                self.sound_note = None;
                let preset = match cx.layout.voice.sound_preset {
                    Some(preset) if preset < cx.layout.voice.sound_presets.len() => preset,
                    _ => new_preset(cx),
                };
                cx.layout.voice.sound_presets[preset]
                    .files
                    .insert(sound, chosen);
                keep_presets(cx);
                try_sound(cx, sound);
            }
        }
        cx.touch();
    }
}

/// One of BACKCORD's sounds, to try a setting, on the call's speakers.
fn try_sound(cx: &Cx<'_>, sound: Sound) {
    let voice = &cx.state().voice;
    let root = cx.client.root();
    backcord_core::sounds::play(
        root,
        cx.layout.voice.sound_file(root, sound),
        sound,
        cx.layout.voice.sound_gain(sound),
        voice.speaker.as_deref(),
    );
}

impl<'a> Page<'a> {
    /// A switch that flips a preference.
    fn flip(
        self,
        label: &'a str,
        description: &'a str,
        on: bool,
        make: fn(bool) -> Pref,
    ) -> Element<'a, Message> {
        switch(label, on, Some(pref(make(!on))))
            .description(description)
            .view(self.cx.look)
    }
}

fn account_message(event: AccountEvent) -> Message {
    Message::Settings(Event::Account(event))
}

/// Messages' text at a size: 14 px small, 15 default, 18 large.
pub fn message_size(text_size: Option<i32>) -> f32 {
    match text_size {
        Some(0) => 14.0,
        Some(2) => 18.0,
        _ => 15.0,
    }
}
