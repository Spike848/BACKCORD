//! Extra features, as Equicord lists its plugins: a card for each, two to
//! a row, with its name, what it does, a gear for those with settings and
//! its switch. The gear opens the feature's settings in a dialog over
//! Settings, which Escape, Close or a click outside closes.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, container, opaque, row, stack, text},
};

use super::{Pref, pref};
use crate::settings::{Event, Page};
use crate::{
    app::Message,
    controls::choices,
    fonts::BOLD,
    form::{self, switch},
    icon_viewer::{DEFAULT_SIZE, FORMATS, SIZE_NAMES},
    pictures::Icon,
    stream_proof::LOCATIONS,
};

/// The features with settings of their own, whose dialog the gear opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    MessageLogger,
    SilentTyping,
    ViewIcons,
    StreamProof,
}

/// The dialog's width.
const DIALOG: f32 = 560.0;

fn show(extra: Option<Extra>) -> Message {
    Message::Settings(Event::Extra(extra))
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn extras_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let tr = backcord_iced_kit::i18n::tr;
        let roles = prefs.clickable_roles.unwrap_or(true);
        let icons = prefs.view_icons.unwrap_or(true);
        let proof = prefs.stream_proof.unwrap_or(true);
        let logger = prefs.message_logger.unwrap_or(true);
        let silent = !prefs.share_typing.unwrap_or(false);
        let no_ptt = prefs.no_push_to_talk.unwrap_or(true);
        let cards = vec![
            self.plugin(
                tr("Clickable Roles"),
                tr("Click on roles in user profiles to see which members have them."),
                roles,
                Pref::ClickableRoles(!roles),
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
        };
        let sunken = theme.sidebar;
        let settings = container(column(options).spacing(12))
            .padding(16)
            .width(Length::Fill)
            .style(move |_| crate::style::rounded(sunken, 8));
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
