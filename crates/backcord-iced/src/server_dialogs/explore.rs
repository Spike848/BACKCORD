//! Explore servers (`state.explore`): a search, the orders, and the
//! Italian servers DISBOARD lists, each with its icon, name, tags, what it
//! says, how many are online and Join, or Open for one this account is in.

use backcord_core::{
    application::{ExploreJoin, ExploreSort},
    chat::ExploreServer,
};
use backcord_iced_kit::{i18n::tr, widgets::scroll_view};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text},
};

use super::{Event, View, message};
use crate::pictures::Icon;
use crate::{
    app::Message,
    controls::{Kind, action},
    form,
    style::rounded,
};

/// The search's id.
pub(super) const SEARCH: &str = "explore-search";
const WIDTH: f32 = 720.0;
/// How tall the list is.
const LIST: f32 = 460.0;
const ICON: f32 = 48.0;
/// The most servers drawn at once; the search finds the rest.
pub(super) const SHOWN: usize = 150;
/// How much of a description a row shows.
const DESCRIPTION: usize = 180;
/// The button's column, wide enough for its longest label.
const BUTTON: f32 = 150.0;

impl<'a> View<'a> {
    pub(super) fn explore_view(self) -> Option<Element<'a, Message>> {
        let explore = self.cx.state.explore.as_ref()?;
        let theme = self.cx.look.theme;
        let close = message(Event::CloseExplore);
        let mut body = vec![
            self.cx
                .look
                .dialog_head(tr("Explore Italian servers"), close.clone()),
            form::note(
                theme,
                tr(
                    "Servers listed on DISBOARD. Join enters them here when they have a public invite.",
                ),
            ),
            form::field(
                theme,
                SEARCH,
                tr("Search by name, tag or description"),
                &explore.search,
            )
            .padding([8.0, 10.0])
            .on_input(|typed| message(Event::ExploreSearch(typed)))
            .into(),
        ];
        let sort = |label: &'static str, which: ExploreSort| {
            let chosen = explore.sort == which;
            let arrow = match (chosen, which, explore.descending) {
                (false, _, _) => "",
                (true, ExploreSort::Name, false) => " A→Z",
                (true, ExploreSort::Name, true) => " Z→A",
                (true, ExploreSort::Online, true) => " ↓",
                (true, ExploreSort::Online, false) => " ↑",
            };
            Element::from(action(
                theme,
                format!("{}{arrow}", tr(label)),
                if chosen { Kind::Primary } else { Kind::Plain },
                Some(message(Event::ExploreSort(which))),
            ))
        };
        body.push(
            row![
                text(tr("Sort by")).size(13).color(theme.muted),
                sort("Members online", ExploreSort::Online),
                sort("Name", ExploreSort::Name),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        );
        if explore.saved {
            body.push(form::note(
                theme,
                tr("DISBOARD asks to slow down: these are the servers saved earlier."),
            ));
        }
        let shown = explore.shown();
        let mut list = column![].spacing(8);
        if explore.servers.is_empty() && explore.loading {
            list = list.push(form::note(theme, tr("Loading the servers...")));
        } else if let Some(error) = explore
            .error
            .as_ref()
            .filter(|_| explore.servers.is_empty())
        {
            list = list
                .push(form::note_in(
                    backcord_iced_kit::i18n::t(error),
                    theme.danger,
                ))
                .push(action(
                    theme,
                    tr("Try again"),
                    Kind::Plain,
                    Some(message(Event::ExploreAgain)),
                ));
        } else if shown.is_empty() {
            list = list.push(form::note(theme, tr("No server matches the search.")));
        }
        for server in shown.iter().take(SHOWN) {
            list = list.push(self.explore_row(server));
        }
        if !explore.servers.is_empty() && explore.more() {
            list = list.push(
                container(action(
                    theme,
                    if explore.loading {
                        tr("Loading...")
                    } else {
                        tr("Show more")
                    },
                    Kind::Plain,
                    (!explore.loading).then(|| message(Event::ExploreMore)),
                ))
                .center_x(Length::Fill),
            );
        }
        body.push(
            container(scroll_view(list.padding(iced::Padding {
                right: 12.0,
                ..iced::Padding::ZERO
            })))
            .height(LIST)
            .into(),
        );
        Some(self.cx.look.framed(body, WIDTH, close))
    }

    /// A server's row: its icon, name, how many are online,
    /// tags and what it says (its start, with an arrow to show it all),
    /// then its button, in a column of its own, and how joining goes.
    fn explore_row(self, server: &'a ExploreServer) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let explore = self.cx.state.explore.as_ref();
        let member = self.cx.app.explored_member(&server.guild).is_some();
        let joining = explore.and_then(|explore| explore.joining.get(&server.guild));
        let expanded = explore.is_some_and(|explore| explore.expanded.contains(&server.guild));
        let mut words = column![
            text(server.name.as_str())
                .size(15)
                .font(backcord_iced_kit::fonts::BOLD)
                .color(theme.text)
                .width(Length::Fill)
        ]
        .spacing(4)
        .width(Length::Fill);
        if let Some(online) = server.online {
            words = words.push(
                text(backcord_iced_kit::i18n::t(format!("{online} online")))
                    .size(12)
                    .color(theme.muted),
            );
        }
        if !server.tags.is_empty() {
            let mut tags = row![].spacing(4);
            for tag in server.tags.iter().take(6) {
                tags = tags.push(
                    container(text(tag.as_str()).size(11).color(theme.secondary))
                        .padding([2.0, 6.0])
                        .style(move |_| rounded(theme.raised, 4.0)),
                );
            }
            words = words.push(tags.wrap());
        }
        if !server.description.is_empty() {
            let flat = server.description.replace('\n', " ");
            let long = flat.chars().count() > DESCRIPTION || server.description.contains('\n');
            let shown = if expanded {
                server.description.clone()
            } else if flat.chars().count() > DESCRIPTION {
                format!(
                    "{}…",
                    flat.chars()
                        .take(DESCRIPTION)
                        .collect::<String>()
                        .trim_end()
                )
            } else {
                flat
            };
            words = words.push(
                text(shown)
                    .size(13)
                    .color(theme.secondary)
                    .width(Length::Fill),
            );
            if long {
                words = words.push(
                    button(self.cx.look.glyph(
                        if expanded {
                            Icon::ChevronUp
                        } else {
                            Icon::ChevronDown
                        },
                        16,
                        theme.muted,
                    ))
                    .padding([2.0, 8.0])
                    .on_press(message(Event::ExploreExpand(server.guild.clone())))
                    .style(move |_, status| button::Style {
                        background: matches!(
                            status,
                            button::Status::Hovered | button::Status::Pressed
                        )
                        .then(|| theme.raised.into()),
                        border: iced::Border::default().rounded(4.0),
                        ..button::Style::default()
                    }),
                );
            }
        }
        let (label, kind, press): (&str, Kind, Option<Message>) = match (member, joining) {
            (true, _) => (
                tr("Open"),
                Kind::Plain,
                Some(message(Event::ExploreJoin(server.guild.clone()))),
            ),
            (_, Some(ExploreJoin::Finding | ExploreJoin::Joining)) => {
                (tr("Joining..."), Kind::Primary, None)
            }
            (_, Some(ExploreJoin::Captcha)) => (tr("Captcha..."), Kind::Primary, None),
            (_, Some(ExploreJoin::NoInvite)) => (
                tr("Get the invite"),
                Kind::Plain,
                Some(message(Event::ExploreBrowser(server.guild.clone()))),
            ),
            (_, Some(ExploreJoin::Failed(_))) => (
                tr("Try again"),
                Kind::Primary,
                Some(message(Event::ExploreJoin(server.guild.clone()))),
            ),
            (_, None) => (
                tr("Join this server"),
                Kind::Primary,
                Some(message(Event::ExploreJoin(server.guild.clone()))),
            ),
        };
        let note = match (member, joining) {
            (true, _) => Some(tr("Already joined.")),
            (_, Some(ExploreJoin::Captcha)) => Some(tr(
                "Discord asks for a captcha first. Solve it in your browser.",
            )),
            (_, Some(ExploreJoin::NoInvite)) => Some(tr(
                "This server has no public invite. DISBOARD gives one after a check in the browser: paste it into Add a server.",
            )),
            _ => None,
        };
        if let Some(note) = note {
            words = words.push(text(note).size(12).color(theme.muted).width(Length::Fill));
        }
        if let Some(ExploreJoin::Failed(error)) = joining.filter(|_| !member) {
            words = words.push(
                text(backcord_iced_kit::i18n::t(error))
                    .size(12)
                    .color(theme.danger)
                    .width(Length::Fill),
            );
        }
        container(
            row![
                container(self.cx.avatar(
                    server.icon.as_deref(),
                    &server.name,
                    ICON,
                    ICON / 2.0,
                    1.0
                ))
                .width(ICON),
                words,
                container(action(theme, label, kind, press).wide(36.0)).width(BUTTON),
            ]
            .spacing(12)
            .align_y(Alignment::Start),
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_| rounded(theme.sunken, 8.0))
        .into()
    }
}
