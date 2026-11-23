//! Notifications: desktop notifications on or off, and BACKCORD's sounds:
//! their volume, the preset playing, and each sound's own volume and file.

use backcord_core::sounds::Sound;
use backcord_iced_kit::i18n::tr;
use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

use super::{Pref, pref};
use crate::settings::{Event, Page};
use crate::{
    app::Message,
    controls::{Kind, action, slider},
    form::{self, switch},
    pictures::Icon,
};

/// The field naming the preset playing.
pub const PRESET_FIELD: &str = "settings-sound-preset";

/// Each of BACKCORD's sounds, in `Sound::ALL`'s order.
pub(super) const SOUND_NAMES: [&str; 12] = [
    "Someone joins the call",
    "Someone leaves the call",
    "Muting",
    "Unmuting",
    "Deafening",
    "Undeafening",
    "A message",
    "A mention",
    "Someone starts watching a stream",
    "Someone stops watching a stream",
    "Someone starts streaming",
    "Someone stops streaming",
];

impl<'a> Page<'a> {
    pub(in crate::settings) fn notifications_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let on = prefs.notifications.unwrap_or(true);
        let rules = if cfg!(feature = "matrix") && self.linked() {
            tr(
                "Each Discord server's notification settings and your Matrix notification rules decide",
            )
        } else if cfg!(feature = "matrix") {
            tr("Your Matrix notification rules decide")
        } else {
            tr("Each Discord server's notification settings decide")
        };
        let voice = &self.cx.layout.voice;
        let volume = u32::from(voice.sound_volume.unwrap_or(100));
        let preset = voice.preset();
        let mut names = vec![tr("Discord")];
        names.extend(
            voice
                .sound_presets
                .iter()
                .map(|preset| preset.name.as_str()),
        );
        let chosen = voice
            .sound_preset
            .filter(|_| preset.is_some())
            .map_or(0, |index| index + 1);
        let mut buttons = row![
            action(
                theme,
                tr("New preset"),
                Kind::Plain,
                Some(pref(Pref::NewPreset))
            )
            .small()
        ]
        .spacing(8);
        if preset.is_some() {
            buttons = buttons.push(
                action(
                    theme,
                    tr("Delete preset"),
                    Kind::Danger,
                    Some(pref(Pref::DeletePreset)),
                )
                .small(),
            );
        }
        let mut group = vec![
            form::heading(theme, tr("PRESET")),
            crate::controls::choices_within(theme, &names, chosen, 520.0, |index| {
                pref(Pref::SoundPreset(index))
            }),
        ];
        if let Some(preset) = preset {
            group.push(
                form::field(theme, PRESET_FIELD, tr("Preset name"), &preset.name)
                    .on_input(|name| pref(Pref::PresetName(name)))
                    .into(),
            );
        }
        group.push(buttons.into());
        let mut parts = vec![
            form::title(theme, tr("Notifications")),
            switch(
                tr("Show desktop notifications"),
                on,
                Some(pref(Pref::Notifications(!on))),
            )
            .description(backcord_iced_kit::i18n::t(format!(
                "For new messages while BACKCORD isn't in front. {rules} which: usually DMs and mentions. Clicking one opens the conversation."
            )))
            .view(self.cx.look),
            form::heading(theme, tr("SOUNDS")),
            form::note(
                theme,
                tr(
                    "BACKCORD plays Discord's sounds as you and others join and leave a call, as you mute and deafen, as others start and stop streaming, as they start and stop watching your stream or the one you watch, and for messages that notify, except in the conversation in front of you. They play on the speakers chosen for calls. A preset plays your own files in place of some of them: choose a file for a sound to make one.",
                ),
            ),
            slider(tr("Volume"), volume, 200, |value| {
                pref(Pref::SoundVolume(value, false))
            })
            .on_release(pref(Pref::SoundVolume(volume, true)))
            .view(theme),
            form::group(group),
        ];
        if let Some(why) = &self.s.sound_note {
            parts.push(form::error(theme, why.as_str()).into());
        }
        let mut each = column![].spacing(12);
        for (index, (name, sound)) in SOUND_NAMES.iter().zip(Sound::ALL).enumerate() {
            let value = u32::from(voice.each_sound.get(&sound).copied().unwrap_or(100));
            let file = preset.and_then(|preset| preset.files.get(&sound));
            let mut controls = row![
                container(
                    slider(tr(name), value, 100, move |value| {
                        pref(Pref::EachSound(index, value, false))
                    })
                    .on_release(pref(Pref::EachSound(index, value, true)))
                    .view(theme)
                )
                .width(Length::Fill),
                self.cx
                    .look
                    .icon_button(Icon::Play, tr("Play"), pref(Pref::PlaySound(index)))
                    .size(32.0, 18),
                self.cx
                    .look
                    .icon_button(
                        Icon::Folder,
                        tr("Choose a sound"),
                        Message::Settings(Event::PickSound(index)),
                    )
                    .size(32.0, 18),
            ]
            .spacing(12)
            .align_y(Alignment::Center);
            if file.is_some() {
                controls = controls.push(
                    self.cx
                        .look
                        .icon_button(
                            Icon::Refresh,
                            tr("Play Discord's"),
                            pref(Pref::ClearSound(index)),
                        )
                        .size(32.0, 18),
                );
            }
            let source = file.map_or(tr("Discord's"), |file| file.name.as_str());
            each = each.push(
                column![
                    controls,
                    text(source)
                        .size(12)
                        .color(theme.muted)
                        .wrapping(text::Wrapping::None)
                ]
                .spacing(2),
            );
        }
        parts.push(each.into());
        form::page(parts)
    }
}
