//! Extra features, as Equicord lists its plugins: a card for each, two to
//! a row, with its name, what it does, a gear for those with settings and
//! its switch. The gear opens the feature's settings in a dialog over
//! Settings, which Escape, Close or a click outside closes.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, opaque, row, stack, text},
};

use super::{Pref, pref};
use crate::settings::{Event, Page};
use crate::{
    app::Message,
    controls::{choices, choices_within},
    fonts::BOLD,
    form::{self, switch},
    icon_viewer::{DEFAULT_SIZE, FORMATS, SIZE_NAMES},
    pictures::Icon,
    stream_proof::LOCATIONS,
};

/// The features with settings of their own, whose dialog the gear opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    EmulatePlatforms,
    MessageLogger,
    SilentTyping,
    ViewIcons,
    StreamProof,
    RelationshipNotifier,
    FakeNitro,
    BadgeSpoofer,
}

/// The dialog's width.
const DIALOG: f32 = 560.0;

/// Emulate Platforms' platforms, by the index the preferences keep (none,
/// then `Emulated::ALL`'s), in the two rows its dialog shows them in.
const PLATFORM_ROWS: [[usize; 5]; 2] = [[0, 1, 2, 3, 9], [4, 5, 6, 7, 8]];
const PLATFORM_NAMES: [&str; 10] = [
    "Don't emulate",
    "Windows",
    "Linux",
    "macOS",
    "Android",
    "iOS",
    "Xbox",
    "PlayStation",
    "VR Headset",
    "Web",
];
const DEPTHS: [&str; 3] = ["Low level", "Medium level", "High level"];

/// Fake Nitro's emoji sizes, as `backcord_core::fake_nitro::SIZES` has them.
const FAKE_NITRO_SIZES: [&str; 6] = ["32", "48", "64", "96", "128", "160"];

/// Hands Discord's side the platform Emulate Platforms says this account
/// is on, which its next identify sends.
pub fn emulate_platform(prefs: &crate::layout::Prefs) {
    use backcord_core::discord::{Emulated, EmulationDepth};
    let platform = prefs
        .emulated_platform
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| Emulated::ALL.get(index).copied());
    let depth = match prefs.emulation_depth.unwrap_or(0) {
        0 => EmulationDepth::Low,
        1 => EmulationDepth::Medium,
        _ => EmulationDepth::High,
    };
    backcord_core::discord::set_emulation(platform, depth);
}

/// Relationship Notifier's removals, in the order its dialog lists them and
/// `Pref::RelationshipKind` numbers them: each switch's name and what it
/// tells of.
const RELATIONSHIP_KINDS: [(&str, &str); 5] = [
    (
        "Friends",
        "When a friend removes you or blocks you, and when someone accepts your friend request.",
    ),
    (
        "Friend requests",
        "When someone sends you a friend request, declines the one you sent, or withdraws theirs.",
    ),
    ("Group DMs", "When you're removed from a group DM."),
    (
        "Servers",
        "When you're kicked or banned from a server, or it's deleted.",
    ),
    (
        "While offline",
        "As BACKCORD starts, what changed while it was closed.",
    ),
];

/// Relationship Notifier's switches, each on unless turned off, in
/// [`RELATIONSHIP_KINDS`]' order.
fn relationship_switches(prefs: &crate::layout::Prefs) -> [bool; 5] {
    [
        prefs.relationship_friends,
        prefs.relationship_requests,
        prefs.relationship_groups,
        prefs.relationship_servers,
        prefs.relationship_offline,
    ]
    .map(|on| on.unwrap_or(true))
}

/// Hands the core the removals Relationship Notifier tells of: none while
/// it's off.
pub fn relationship_notifier(prefs: &crate::layout::Prefs) {
    use backcord_core::relationship_alerts::Kinds;
    let [friends, requests, groups, servers, offline] = relationship_switches(prefs);
    let kinds = if prefs.relationship_notifier.unwrap_or(true) {
        Kinds {
            friends,
            requests,
            groups,
            servers,
            offline,
        }
    } else {
        Kinds::NONE
    };
    backcord_core::extras::set_relationship_alerts(kinds);
}

/// Hands the core Fake Nitro and its settings: off unless turned on.
pub fn fake_nitro(prefs: &crate::layout::Prefs) {
    use backcord_core::extras::FakeNitro;
    backcord_core::extras::set_fake_nitro(FakeNitro {
        on: prefs.fake_nitro.unwrap_or(false),
        size: fake_nitro_size(prefs),
        masked: prefs.fake_nitro_masked.unwrap_or(true),
        received: prefs.fake_nitro_received.unwrap_or(true),
    });
}

/// The size Fake Nitro's emoji links ask for, as chosen.
fn fake_nitro_size(prefs: &crate::layout::Prefs) -> u16 {
    use backcord_core::fake_nitro::{DEFAULT_SIZE, SIZES};
    prefs
        .fake_nitro_size
        .and_then(|index| SIZES.get(index).copied())
        .unwrap_or(DEFAULT_SIZE)
}

/// Badge Spoofer's Nitro levels and boost levels, each "None" first, in the
/// two rows its dialog shows each in.
const LEVEL_ROWS: [[usize; 5]; 2] = [[0, 1, 2, 3, 4], [5, 6, 7, 8, 9]];

/// A choice of none or one of `list`, by their labels, in [`LEVEL_ROWS`]'
/// rows: 0 for none, else the one after it.
fn level_rows<'a>(
    theme: &crate::theme::Theme,
    list: &[backcord_core::badge_spoofer::Spoofable],
    chosen: usize,
    choose: fn(usize) -> Pref,
) -> Vec<Element<'a, Message>> {
    let tr = backcord_iced_kit::i18n::tr;
    let label = |index: usize| match index.checked_sub(1) {
        None => tr("None"),
        Some(index) => list.get(index).map_or("", |badge| tr(badge.label)),
    };
    LEVEL_ROWS
        .iter()
        .map(|indices| {
            let indices = *indices;
            let current = indices
                .iter()
                .position(|index| *index == chosen)
                .unwrap_or(usize::MAX);
            choices_within(theme, &indices.map(label), current, 480.0, move |at| {
                pref(choose(indices[at]))
            })
        })
        .collect()
}

fn show(extra: Option<Extra>) -> Message {
    Message::Settings(Event::Extra(extra))
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn extras_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let tr = backcord_iced_kit::i18n::tr;
        let roles = prefs.clickable_roles.unwrap_or(true);
        let permissions = prefs.permissions_viewer.unwrap_or(true);
        let icons = prefs.view_icons.unwrap_or(true);
        let proof = prefs.stream_proof.unwrap_or(true);
        let logger = prefs.message_logger.unwrap_or(true);
        let split_long = prefs.split_long_messages.unwrap_or(true);
        let silent = !prefs.share_typing.unwrap_or(false);
        let no_ptt = prefs.no_push_to_talk.unwrap_or(true);
        let platforms = prefs.platform_indicators.unwrap_or(true);
        let anti_disconnect = prefs.anti_disconnect.unwrap_or(false);
        let follow = prefs.follow_voice.unwrap_or(true);
        let auto_unmute = prefs.auto_unmute.unwrap_or(false);
        let message_links = prefs.message_link_previews.unwrap_or(true);
        let hidden = prefs.show_hidden_channels;
        let relationships = prefs.relationship_notifier.unwrap_or(true);
        let fake_nitro = prefs.fake_nitro.unwrap_or(false);
        let badge_spoofer = prefs.badge_spoofer.unwrap_or(false);
        let cards = vec![
            self.plugin(
                tr("Emulate Platforms"),
                tr("Shows the platforms people are online on beside their names, coloured by their status, and chooses the platform Discord sees you on."),
                platforms,
                Pref::PlatformIndicators(!platforms),
                Some(Extra::EmulatePlatforms),
            ),
            self.plugin(
                tr("Anti-Disconnect"),
                tr("Joins the voice channel you were disconnected from again at once, while it's still there and you may join it."),
                anti_disconnect,
                Pref::AntiDisconnect(!anti_disconnect),
                None,
            ),
            self.plugin(
                tr("Follow"),
                tr("Adds Follow to people's menus: you join each voice channel they go to, in servers you share with them, until you turn it off."),
                follow,
                Pref::FollowVoice(!follow),
                None,
            ),
            self.plugin(
                tr("Auto Unmute"),
                tr("When a server mutes or deafens you in a voice channel where you may mute and deafen members, undoes it at once."),
                auto_unmute,
                Pref::AutoUnmute(!auto_unmute),
                None,
            ),
            self.plugin(
                tr("Show Hidden Channels"),
                tr("Shows the channels you can't access, with a lock: their names, and who's in the voice ones."),
                hidden,
                Pref::ShowHiddenChannels(!hidden),
                None,
            ),
            self.plugin(
                tr("Clickable Roles"),
                tr("Click on roles in user profiles to see which members have them."),
                roles,
                Pref::ClickableRoles(!roles),
                None,
            ),
            self.plugin(
                tr("Permissions Viewer"),
                tr("Adds Permissions to the menus of members, servers and channels, and to a role's card: what they may do there, and which role or channel override gives or takes each permission."),
                permissions,
                Pref::PermissionsViewer(!permissions),
                None,
            ),
            self.plugin(
                tr("No Push To Talk"),
                tr("Bypasses the push-to-talk requirement for voice activity in channels that enforce it."),
                no_ptt,
                Pref::NoPushToTalk(!no_ptt),
                None,
            ),
            self.plugin(
                tr("Message Logger"),
                tr("Temporarily logs deleted and edited messages."),
                logger,
                Pref::MessageLogger(!logger),
                Some(Extra::MessageLogger),
            ),
            self.plugin(
                tr("Split Long Messages"),
                tr("Sends a message longer than Discord allows (2,000 characters, 4,000 with Nitro) as several, cut at line ends and spaces, keeping code blocks whole."),
                split_long,
                Pref::SplitLongMessages(!split_long),
                None,
            ),
            self.plugin(
                tr("Silent Typing"),
                tr("Hide that you are typing."),
                silent,
                Pref::ShareTyping(silent),
                Some(Extra::SilentTyping),
            ),
            self.plugin(
                tr("View Icons"),
                tr("Makes avatars and banners in user profiles clickable, and adds View Avatar and View Icon to the menus of people and servers."),
                icons,
                Pref::ViewIcons(!icons),
                Some(Extra::ViewIcons),
            ),
            self.plugin(
                tr("Stream Proof"),
                tr("Hides messages on your screen while you stream, until you show one. Turn it on and off with its button."),
                proof,
                Pref::StreamProof(!proof),
                Some(Extra::StreamProof),
            ),
            self.plugin(
                tr("Message Link Previews"),
                tr("Shows the Discord messages a message links to in a card under it: who wrote them, where, and what they say. Click one to go there."),
                message_links,
                Pref::MessageLinkPreviews(!message_links),
                None,
            ),
            self.plugin(
                tr("Relationship Notifier"),
                tr("Tells you when a friend removes you, a friend request ends, or you're removed from a group DM or a server, and lists it in the inbox's Friend Alerts."),
                relationships,
                Pref::RelationshipNotifier(!relationships),
                Some(Extra::RelationshipNotifier),
            ),
            self.plugin(
                tr("Fake Nitro"),
                tr("Lets you pick any server's emoji and stickers without Nitro: those you can't use are sent as links to their pictures, and such links you receive show as emoji."),
                fake_nitro,
                Pref::FakeNitro(!fake_nitro),
                Some(Extra::FakeNitro),
            ),
            self.plugin(
                tr("Badge Spoofer"),
                tr("Shows the Discord badges you choose on your own profile, as if you had them. Only you see them, in BACKCORD on this computer: nothing is sent to Discord."),
                badge_spoofer,
                Pref::BadgeSpoofer(!badge_spoofer),
                Some(Extra::BadgeSpoofer),
            ),
        ];
        let mut grid = column![].spacing(12);
        let mut cards = cards.into_iter();
        while let Some(first) = cards.next() {
            let second = cards
                .next()
                .unwrap_or_else(|| Space::new().width(Length::Fill).into());
            grid = grid.push(row![first, second].spacing(12));
        }
        form::page([form::title(theme, tr("Extra Features")), grid.into()])
    }

    /// A feature's card: its name, its gear when it has settings and its
    /// switch on top, what it does under them.
    fn plugin(
        self,
        name: &'a str,
        description: &'a str,
        on: bool,
        flip: Pref,
        settings: Option<Extra>,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut top = row![
            text(name)
                .size(16)
                .font(BOLD)
                .color(theme.text)
                .width(Length::Fill)
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if let Some(extra) = settings {
            top = top.push(
                self.cx
                    .look
                    .icon_button(
                        Icon::Settings,
                        backcord_iced_kit::i18n::tr("Settings"),
                        show(Some(extra)),
                    )
                    .size(32.0, 20),
            );
        }
        top = top.push(form::toggle(theme, on, Some(pref(flip))));
        let ground = theme.sidebar;
        container(
            column![
                top,
                text(description)
                    .size(13)
                    .color(theme.secondary)
                    .width(Length::Fill)
            ]
            .spacing(10),
        )
        .padding(16)
        .width(Length::Fill)
        .height(132)
        .style(move |_| crate::style::rounded(ground, 8))
        .into()
    }

    /// A feature's settings, in a dialog over Settings: its name and Close,
    /// what it does, and its settings on a sunken ground.
    pub(in crate::settings) fn extra_dialog(self, extra: Extra) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let look = self.cx.look;
        let prefs = &self.cx.layout.prefs;
        let tr = backcord_iced_kit::i18n::tr;
        let (name, description, options): (&str, &str, Vec<Element<'a, Message>>) = match extra {
            Extra::EmulatePlatforms => {
                let chosen = prefs.emulated_platform.unwrap_or(0);
                let mut options = vec![
                    form::heading(theme, tr("PLATFORM TO EMULATE")),
                    form::note(
                        theme,
                        tr(
                            "Discord sees you on it from your next start. Saying you're on a platform you aren't on may get your account flagged: use it at your own risk.",
                        ),
                    ),
                ];
                for indices in PLATFORM_ROWS {
                    let names = indices.map(|index| tr(PLATFORM_NAMES[index]));
                    let current = indices
                        .iter()
                        .position(|index| *index == chosen)
                        .unwrap_or(usize::MAX);
                    options.push(choices(theme, &names, current, move |at| {
                        pref(Pref::EmulatedPlatform(indices[at]))
                    }));
                }
                options.push(form::heading(theme, tr("EMULATION LEVEL")));
                options.push(form::note(
                    theme,
                    tr("Low: device and locale. Medium: also the version and release channel. High: also the user agent and client build."),
                ));
                options.push(choices(
                    theme,
                    &DEPTHS.map(tr),
                    prefs.emulation_depth.unwrap_or(0),
                    |index| pref(Pref::EmulationDepth(index)),
                ));
                (
                    tr("Emulate Platforms"),
                    tr(
                        "Shows the platforms people are online on beside their names, coloured by their status, and chooses the platform Discord sees you on.",
                    ),
                    options,
                )
            }
            Extra::MessageLogger => {
                let own = prefs.message_logger_own.unwrap_or(true);
                (
                    tr("Message Logger"),
                    tr("Temporarily logs deleted and edited messages."),
                    vec![
                        switch(
                            tr("Log my own messages"),
                            own,
                            Some(pref(Pref::MessageLoggerOwn(!own))),
                        )
                        .description(tr("Your deleted messages stay, marked deleted, and your edited ones keep what they said before, as others' do."))
                        .view(look),
                    ],
                )
            }
            Extra::SilentTyping => {
                let others = prefs.hide_others_typing.unwrap_or(false);
                (
                    tr("Silent Typing"),
                    tr("Hide that you are typing."),
                    vec![
                        switch(
                            tr("Hide others typing"),
                            others,
                            Some(pref(Pref::HideOthersTyping(!others))),
                        )
                        .description(tr(
                            "Hide other users' typing indicators from above the chat bar.",
                        ))
                        .view(look),
                    ],
                )
            }
            Extra::ViewIcons => (
                tr("View Icons"),
                tr(
                    "Makes avatars and banners in user profiles clickable, and adds View Avatar and View Icon to the menus of people and servers.",
                ),
                vec![
                    form::heading(theme, tr("FORMAT")),
                    form::note(
                        theme,
                        tr("For still pictures. Moving ones always open animated."),
                    ),
                    choices(theme, &FORMATS, prefs.icon_format.unwrap_or(0), |index| {
                        pref(Pref::IconFormat(index))
                    }),
                    form::heading(theme, tr("IMAGE SIZE")),
                    choices(
                        theme,
                        &SIZE_NAMES,
                        prefs.icon_size.unwrap_or(DEFAULT_SIZE),
                        |index| pref(Pref::IconSize(index)),
                    ),
                ],
            ),
            Extra::StreamProof => {
                let blackout = prefs.stream_proof_blackout.unwrap_or(false);
                let blur = prefs.stream_proof_blur.unwrap_or(true);
                let auto = prefs.stream_proof_auto.unwrap_or(true);
                (
                    tr("Stream Proof"),
                    tr("Hides messages on your screen while you stream, until you show one. Turn it on and off with its button."),
                    vec![
                        form::heading(theme, tr("BUTTON LOCATION")),
                        choices(
                            theme,
                            &LOCATIONS.map(tr),
                            prefs.stream_proof_location.unwrap_or(1),
                            |index| pref(Pref::StreamProofLocation(index)),
                        ),
                        switch(tr("Blackout Stream"), blackout, Some(pref(Pref::StreamProofBlackout(!blackout))))
                            .description(tr("While it's on, screen shares and captures show BACKCORD's window black, on Windows."))
                            .view(look),
                        switch(tr("Local Blur"), blur, Some(pref(Pref::StreamProofBlur(!blur))))
                            .description(tr("While it's on, messages hide on your own screen until you show one."))
                            .view(look),
                        switch(tr("Auto Stream Proof"), auto, Some(pref(Pref::StreamProofAuto(!auto))))
                            .description(tr("Turns it on when you start streaming, and off when you stop."))
                            .view(look),
                    ],
                )
            }
            Extra::FakeNitro => {
                use backcord_core::fake_nitro::SIZES;
                let masked = prefs.fake_nitro_masked.unwrap_or(true);
                let received = prefs.fake_nitro_received.unwrap_or(true);
                let size = fake_nitro_size(prefs);
                (
                    tr("Fake Nitro"),
                    tr("Lets you pick any server's emoji and stickers without Nitro: those you can't use are sent as links to their pictures, and such links you receive show as emoji."),
                    vec![
                        form::heading(theme, tr("EMOJI SIZE")),
                        form::note(theme, tr("The size, in pixels, of the emoji pictures you send as links.")),
                        choices_within(
                            theme,
                            &FAKE_NITRO_SIZES,
                            SIZES.iter().position(|known| *known == size).unwrap_or(1),
                            480.0,
                            |index| pref(Pref::FakeNitroSize(index)),
                        ),
                        switch(tr("Use masked links"), masked, Some(pref(Pref::FakeNitroMasked(!masked))))
                            .description(tr("Sends each link under the emoji's or sticker's name, as [name](link), rather than bare."))
                            .view(look),
                        switch(tr("Show received emoji links as emoji"), received, Some(pref(Pref::FakeNitroReceived(!received))))
                            .description(tr("Links to emoji pictures in messages show as the emoji, in the text, without their picture under it."))
                            .view(look),
                    ],
                )
            }
            Extra::BadgeSpoofer => {
                use backcord_core::badge_spoofer::{BADGES, BOOSTS, HOUSES, NITRO};
                let settings = prefs.badge_spoofer();
                let tick = |index: usize| -> Element<'a, Message> {
                    let ticked = settings.has(index);
                    button(
                        row![
                            look.checkbox(ticked, false),
                            text(tr(BADGES[index].label))
                                .size(14)
                                .color(theme.text)
                                .width(Length::Fill)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .padding(0)
                    .width(Length::Fill)
                    .on_press(pref(Pref::SpoofedBadge(index, !ticked)))
                    .style(|_, _| iced::widget::button::Style::default())
                    .into()
                };
                let mut options = vec![
                    form::heading(theme, tr("BADGES")),
                    form::note(
                        theme,
                        tr(
                            "Your profile shows them in Discord's order, with Discord's own pictures.",
                        ),
                    ),
                ];
                // Two to a row, in Discord's order down each row.
                let mut ticks = column![].spacing(10);
                for pair in (0..BADGES.len()).collect::<Vec<_>>().chunks(2) {
                    let second: Element<'a, Message> = match pair.get(1) {
                        Some(index) => tick(*index),
                        None => Space::new().width(Length::Fill).into(),
                    };
                    ticks = ticks.push(row![tick(pair[0]), second].spacing(12));
                }
                options.push(ticks.into());
                options.push(form::heading(theme, tr("HYPESQUAD HOUSE")));
                let houses = [tr("None")]
                    .into_iter()
                    .chain(HOUSES.iter().map(|house| tr(house.label)))
                    .collect::<Vec<_>>();
                options.push(choices(
                    theme,
                    &houses,
                    usize::from(settings.house),
                    |index| pref(Pref::SpoofedHouse(index)),
                ));
                options.push(form::heading(theme, tr("NITRO")));
                options.extend(level_rows(
                    theme,
                    &NITRO,
                    usize::from(settings.nitro),
                    Pref::SpoofedNitro,
                ));
                options.push(form::heading(theme, tr("SERVER BOOSTER")));
                options.extend(level_rows(
                    theme,
                    &BOOSTS,
                    usize::from(settings.boost),
                    Pref::SpoofedBoost,
                ));
                options.push(
                    switch(
                        tr("Show my real badges too"),
                        settings.keep_real,
                        Some(pref(Pref::SpoofRealBadges(!settings.keep_real))),
                    )
                    .description(tr("The badges your profile really has stay beside those chosen. A chosen house, Nitro or boost badge takes the place of your own."))
                    .view(look),
                );
                (
                    tr("Badge Spoofer"),
                    tr(
                        "Shows the Discord badges you choose on your own profile, as if you had them. Only you see them, in BACKCORD on this computer: nothing is sent to Discord.",
                    ),
                    options,
                )
            }
            Extra::RelationshipNotifier => {
                let switches = relationship_switches(prefs);
                (
                    tr("Relationship Notifier"),
                    tr(
                        "Tells you when a friend removes you, a friend request ends, or you're removed from a group DM or a server, and lists it in the inbox's Friend Alerts.",
                    ),
                    RELATIONSHIP_KINDS
                        .into_iter()
                        .zip(switches)
                        .enumerate()
                        .map(|(index, ((name, description), on))| {
                            switch(tr(name), on, Some(pref(Pref::RelationshipKind(index, !on))))
                                .description(tr(description))
                                .view(look)
                        })
                        .collect(),
                )
            }
        };
        let sunken = theme.sidebar;
        let settings = container(column(options).spacing(12))
            .padding(16)
            .width(Length::Fill)
            .style(move |_| crate::style::rounded(sunken, 8));
        // A long list of settings scrolls in a window too short for it.
        let settings =
            container(crate::widgets::scroll_view(settings).style(crate::style::scrollbar(theme)))
                .max_height((look.window.height - 240.0).max(160.0));
        let body = column![
            row![
                text(name)
                    .size(20)
                    .font(BOLD)
                    .color(theme.text)
                    .width(Length::Fill),
                look.icon_button(Icon::X, tr("Close"), show(None))
                    .size(32.0, 18)
            ]
            .align_y(Alignment::Center),
            text(description)
                .size(14)
                .color(theme.secondary)
                .width(Length::Fill),
            text(tr("Settings")).size(16).font(BOLD).color(theme.text),
            settings,
        ]
        .spacing(12)
        .padding(24)
        .width(DIALOG);
        let ground = theme.panel;
        let edge = theme.edge;
        let card = container(body).style(move |_| {
            crate::style::rounded(ground, 12)
                .border(iced::Border {
                    color: edge,
                    width: 1.0,
                    radius: 12.0.into(),
                })
                .shadow(crate::style::raised_shadow())
        });
        opaque(stack![
            backcord_iced_kit::controls::dimmed(show(None), crate::sheet::DIM),
            container(opaque(card))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
        ])
    }
}
