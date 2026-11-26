//! The sidebar's top: a server's banner and name, its boost goal and
//! events, or the DMs' search.

use iced::{
    Alignment, Background, Color, Element, Length, Padding,
    widget::{Space, button, column, container, row, stack, text, text_input},
};

use crate::{
    app::Message,
    controls::{bar, lit_style},
    elide::elide,
    fonts::BOLD,
    pictures::Icon,
    style::corners,
    widgets::{Pic, RightPress},
};

use super::{BANNER, Event, FILTER, HEADER, INSET, INVITE};

/// The number in a server's Boost level badge, measured (237, 237, 239).
const TIER_INK: Color = Color::from_rgb(237.0 / 255.0, 237.0 / 255.0, 239.0 / 255.0);

impl<'a> super::View<'a> {
    /// The sidebar's top: a server's name (with Invite people beside it),
    /// or the DMs' search field, over the outline; with a banner, the
    /// header grows over it, shaded at its top, with no line under it.
    pub(super) fn side_header_view(self) -> Element<'a, Message> {
        let header = self.side_header_inner();
        if self.s.header.server.is_none() {
            return header;
        }
        // A right click on the banner or the name opens the server's menu
        // where the pointer is, View Icon and View Banner with it.
        let spot = self.cx.pressed;
        RightPress::new(header, move |_| Message::Side(Event::HeaderAt(spot.get()))).into()
    }

    fn side_header_inner(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let head = &self.s.header;
        let inner: Element<'a, Message> = match &head.server {
            None => {
                let (ground, ink, hint, selection) =
                    (theme.field, theme.text, theme.muted, theme.selection);
                text_input(
                    backcord_iced_kit::i18n::tr("Find a conversation"),
                    &self.s.filter,
                )
                .id(FILTER)
                .on_input(|filter| Message::Side(Event::Filter(filter)))
                .size(13)
                .padding([8, 10])
                .style(move |_, _| text_input::Style {
                    background: ground.into(),
                    border: crate::style::round(6),
                    icon: ink,
                    placeholder: hint,
                    value: ink,
                    selection,
                })
                .into()
            }
            Some(_) => {
                let mut line = row![self.server_name_button()].spacing(INSET);
                if head.invite {
                    line = line.push(
                        self.cx
                            .look
                            .icon_button(
                                Icon::UserPlus,
                                backcord_iced_kit::i18n::tr("Invite people"),
                                Message::Side(Event::Invite),
                            )
                            .size(INVITE, 20)
                            .below(),
                    );
                }
                line.into()
            }
        };
        let bar_ = container(inner)
            .padding(INSET)
            .height(HEADER - 1.0)
            .width(Length::Fill);
        let banner = head.banner.as_deref().filter(|_| head.server.is_some());
        // Faded out as the list scrolled, the banner leaves the bar as
        // it is without one.
        if banner.is_none() || self.s.banner_shown() <= 0.0 {
            return column![bar_, bar(theme.outline, Length::Fill, 1)].into();
        }
        let mut layers = stack![];
        let shade = Background::Gradient(
            iced::gradient::Linear::new(iced::Degrees(180.0))
                .add_stop(
                    0.0,
                    Color::from_rgba(0.0, 0.0, 0.0, 0.6 * self.s.banner_shown()),
                )
                .add_stop(1.0, Color::TRANSPARENT)
                .into(),
        );
        layers = layers
            .push(
                container(Space::new())
                    .width(Length::Fill)
                    .height(HEADER)
                    .style(move |_| container::Style {
                        background: Some(shade),
                        border: crate::style::round(corners([12.0, 0.0, 0.0, 0.0])),
                        ..Default::default()
                    }),
            )
            .push(bar_);
        layers.width(Length::Fill).height(HEADER).into()
    }

    /// The server's banner behind the header and the list's top: it
    /// stays where it is and fades as the list scrolls over it, as
    /// Discord's.
    pub(super) fn banner_back(self) -> Option<Element<'a, Message>> {
        let head = &self.s.header;
        let banner = head.banner.as_deref().filter(|_| head.server.is_some())?;
        let height = HEADER + BANNER;
        let mut layers = stack![];
        let width = super::width(self.cx.layout);
        let size = iced::Size::new(width, height);
        let still = self.cx.picture(banner, width as u32, height as u32, true);
        // A moving banner plays, round and round, its still showing until
        // it's ready.
        let moving = backcord_core::chat::moving_banner(banner).and_then(|url| {
            self.cx
                .moving(&url, size, iced::Rectangle::with_size(size), true)
        });
        // It stays where it is and fades as the list scrolls, as Discord's.
        let shown = self.s.banner_shown();
        if shown <= 0.0 {
            // Faded out: nothing to draw.
        } else if let Some(moving) = moving {
            layers = layers.push(
                moving
                    .forever()
                    .radius_each([12.0, 0.0, 0.0, 0.0])
                    .opacity(shown)
                    .still(still),
            );
        } else if let Some(picture) = still {
            layers = layers.push(
                Pic::new(picture, size)
                    .radius_each([12.0, 0.0, 0.0, 0.0])
                    .opacity(shown),
            );
        }
        Some(layers.width(Length::Fill).height(height).into())
    }

    /// A server's name at the top of its channels: its Community badge,
    /// the name and a chevron, lit under the pointer or while its menu is
    /// open, when the chevron turns.
    pub(super) fn server_name_button(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let head = &self.s.header;
        let open = self.cx.menu.is_some_and(|menu| menu.header);
        // Over the banner, in white.
        // Once the banner has mostly faded, as the list's own.
        let on_banner = head.banner.is_some() && self.s.banner_shown() > 0.5;
        let (ink, chevron) = if on_banner {
            (Color::WHITE, Color::WHITE)
        } else {
            (theme.text, theme.secondary)
        };
        let mut face = row![].spacing(10).align_y(Alignment::Center);
        // Discord marks a verified server with its green badge, and one
        // that is only a Community server with the Community badge; any
        // other shows its boost level in a scallop.
        if head.verified {
            face = face.push(self.cx.look.drawn(Icon::Verified, 16));
        } else if head.community {
            face = face.push(self.cx.look.glyph(Icon::Community, 16, ink));
        } else if let Some(tier) = head.tier {
            face = face.push(
                stack![
                    self.cx.look.glyph(Icon::Scallop, 18, theme.nav_read),
                    container(text(tier.to_string()).size(11).font(BOLD).color(TIER_INK))
                        .center(Length::Fill)
                ]
                .width(18)
                .height(18),
            );
        }
        face = face.push(
            row![
                elide(head.name.as_str()).size(15).font(BOLD).color(ink),
                self.cx.look.glyph(
                    if open {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    },
                    16,
                    chevron
                ),
            ]
            .align_y(Alignment::Center),
        );
        let hover = theme.hover;
        button(
            container(face)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .padding(Padding {
            left: if head.community || head.verified || head.tier.is_some() {
                10.0
            } else {
                8.0
            },
            right: 8.0,
            ..Padding::ZERO
        })
        .height(32)
        .width(Length::Fill)
        .on_press(Message::Side(Event::Header))
        .style(lit_style(open.then_some(hover), hover, 6.0))
        .into()
    }
}
