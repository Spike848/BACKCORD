//! Who is in a voice channel, under its row and on its card.

use backcord_core::chat::VoicePerson;
use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{column, container, mouse_area, row, text, tooltip},
};

use crate::{
    app::Message,
    controls::{self},
    elide::elide,
    fonts::{BOLD, MEDIUM, SEMIBOLD, shown_name, style_font},
    pictures::Icon,
    style::{faded, rounded},
    widgets::{Drags, RightPress},
};

use super::{Event, PERSON, VOICE_CARD_PEOPLE, VOICE_CARD_WIDTH};

impl<'a> super::View<'a> {
    /// Who is in a busy voice channel, beside its row: a line summing them
    /// up, then each person with their camera and LIVE, the first eight
    /// and how many more. This account on another client is dimmed, without
    /// its marks.
    pub(super) fn voice_channel_card(self, people: &'a [VoicePerson]) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let summary = backcord_core::voice_roster::Roster::of(people).summary();
        let mut card = column![elide(summary).size(12).font(BOLD).color(theme.secondary)]
            .spacing(4)
            .padding(Padding {
                top: 10.0,
                ..Padding::new(12.0)
            })
            .width(VOICE_CARD_WIDTH);
        for person in people.iter().take(VOICE_CARD_PEOPLE) {
            let opacity = if person.elsewhere { 0.5 } else { 1.0 };
            let mut line = row![
                self.cx
                    .avatar(person.avatar.as_deref(), &person.name, 20.0, 10.0, opacity),
                elide(person.name.as_str())
                    .size(14)
                    .font(MEDIUM)
                    .color(faded(theme.text, opacity)),
            ]
            .spacing(8)
            .height(26)
            .align_y(Alignment::Center);
            if !person.elsewhere {
                if person.video {
                    line = line.push(self.cx.look.glyph(Icon::Video, 16, theme.muted));
                }
                if person.streaming {
                    line = line.push(controls::live_pill(theme, None));
                }
            }
            card = card.push(line);
        }
        let more = people.len().saturating_sub(VOICE_CARD_PEOPLE);
        if more > 0 {
            card = card.push(
                text(backcord_iced_kit::i18n::t(format!("and {more} more")))
                    .size(12)
                    .color(theme.muted),
            );
        }
        container(card).style(controls::tip_style(theme)).into()
    }

    /// Someone in a voice channel, under its row: their avatar and name, and
    /// their camera, LIVE (which watches their stream), and mic and
    /// headphone marks. In the call this account is in, `live` says who
    /// speaks, ringed and in the text colour, and who lost their
    /// connection, faint; neither changes the row's size. A
    /// right click opens their voice menu.
    pub(super) fn voice_person(
        self,
        room: &str,
        person: &'a VoicePerson,
        live: Option<&backcord_core::voice::Participant>,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let id = format!("person:{room}:{}", person.id);
        let pointed = self.cx.pointed.side.as_deref() == Some(id.as_str());
        let opacity = if person.elsewhere && !pointed {
            0.5
        } else {
            1.0
        };
        let speaking = live.is_some_and(crate::call::speaking);
        let ink = if speaking {
            theme.text
        } else if live.is_some_and(|live| !live.connected) {
            theme.muted
        } else {
            theme.secondary
        };
        let avatar = self
            .cx
            .avatar(person.avatar.as_deref(), &person.name, 22.0, 11.0, opacity);
        // The ring of someone speaking, over its edge, at the avatar's own
        // size, so nothing grows when it comes and goes.
        let avatar: Element<'a, Message> = if speaking {
            let ring = theme.live;
            iced::widget::stack![
                avatar,
                container(iced::widget::Space::new())
                    .width(22)
                    .height(22)
                    .style(move |_| iced::widget::container::Style {
                        border: iced::Border {
                            color: ring,
                            width: 2.0,
                            radius: 11.0.into(),
                        },
                        ..Default::default()
                    }),
            ]
            .into()
        } else {
            avatar
        };
        let avatar: Element<'a, Message> = container(avatar).center(28.0).into();
        // The name, then the server tag Discord draws after it.
        // Their display name style's font, as Discord draws it here.
        let look = crate::looks::name_look(
            self.cx.state.cosmetics.style(person.style.as_deref()),
            &person.name,
            true,
            theme.dark,
        );
        let styled = look.as_ref().and_then(crate::looks::NameLook::font);
        let mut name = row![
            elide(shown_name(person.name.as_str(), styled))
                .size(15)
                .font(match styled {
                    Some(_) => style_font(styled, 500),
                    None => MEDIUM,
                })
                .color(faded(ink, opacity))
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if let Some(tag) = self.cx.state.cosmetics.tag(person.tag.as_ref()) {
            let size = 15.0 * 0.72;
            name = name.push(self.cx.tag_chip(
                &tag.text,
                tag.badge.as_deref(),
                crate::looks::Chip {
                    size,
                    font: SEMIBOLD,
                    height: size * 1.3333,
                    pad: 4.0,
                    radius: 4.0,
                    fill: crate::style::CHIP,
                    ink: theme.text,
                },
                opacity,
            ));
        }
        let name = container(name).width(Length::Fill);
        // The name takes the room, its marks at the row's end.
        let mut face = row![avatar, name,]
            // The avatar's frame is 6 px wider than it, which this takes back.
            .spacing(2)
            .align_y(Alignment::Center);
        if !person.elsewhere {
            if person.video {
                face = face.push(self.cx.look.glyph(Icon::Video, 16, theme.muted));
            }
            if person.streaming {
                let local = live.is_some_and(|live| live.local);
                let watch = (!local).then(|| {
                    Message::Call(crate::call::Event::ShowPerson(
                        room.to_owned(),
                        person.id.clone(),
                    ))
                });
                face = face.push(controls::live_pill(theme, watch));
            }
            // Each mark as Discord's voice state says, so muted, deafened
            // and both tell apart.
            if person.muted {
                let ink = if person.server_muted {
                    theme.danger
                } else {
                    theme.muted
                };
                face = face.push(self.cx.look.glyph(Icon::MicOff, 16, ink));
            }
            if person.deafened {
                let ink = if person.server_deafened {
                    theme.danger
                } else {
                    theme.muted
                };
                face = face.push(self.cx.look.glyph(Icon::HeadphoneOff, 16, ink));
            }
        }
        // Picked with others, its row stays lit as a selected one.
        let ground = if crate::picks::picked(self.cx.state, &person.id) {
            theme.active
        } else if pointed {
            theme.hover
        } else {
            Color::TRANSPARENT
        };
        let area = mouse_area(
            container(face)
                .padding(Padding {
                    left: 4.0,
                    right: 8.0,
                    ..Padding::ZERO
                })
                .height(PERSON)
                .width(Length::Fill)
                .align_y(Alignment::Center)
                .style(move |_| rounded(ground, 4)),
        )
        .on_enter(Message::Side(Event::Point(id.clone(), true)))
        .on_exit(Message::Side(Event::Point(id, false)));
        let (room, user) = (room.to_owned(), person.id.clone());
        let spot = self.cx.pressed;
        let menu = {
            let (room, user) = (room.clone(), user.clone());
            RightPress::new(area, move |_| {
                Message::Call(crate::call::Event::Person(
                    room.clone(),
                    user.clone(),
                    spot.get(),
                ))
            })
        };
        // A click opens their card; pressed and moved, they're dragged
        // toward another voice channel, where their menu's Move to offers
        // one.
        let drags = Drags::new(menu)
            .click(Message::Call(crate::call::Event::Click(
                room.clone(),
                user.clone(),
            )))
            .drag(move |_| {
                Message::Arrange(crate::arrange::Event::Take(room.clone(), user.clone()))
            });
        if !person.elsewhere {
            return drags.into();
        }
        // This account, in the call from another client.
        tooltip(
            drags,
            controls::tip(
                theme,
                backcord_iced_kit::i18n::tr("Connected on another client"),
            ),
            tooltip::Position::Top,
        )
        .gap(4)
        .into()
    }
}
