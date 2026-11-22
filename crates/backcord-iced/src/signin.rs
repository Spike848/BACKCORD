//! Signing in: a centred card on the rail's ground with the local preview
//! beside it. A build without Matrix links Discord: by the QR code
//! Discord's phone app scans, or a pasted token, with Discord's captcha in
//! the browser when it asks for one. A build with Matrix signs in with a
//! homeserver, Matrix ID and password, and offers the saved session again;
//! once in, the verify banner stays over the conversation until the device
//! is verified. Everything goes to the core.

use crate::widgets::scroll_view;
use backcord_core::application::{Action, AppState, DiscordLink};
use iced::{
    Alignment, Color, Element, Length, Padding, Task,
    widget::{Space, column, container, operation, row, stack, text},
};

use crate::{
    app::{App, Message},
    controls::{Kind, action, bar, field},
    ctx::{Ctx, Cx},
    fonts::BOLD,
    pictures::Icon,
    style::{card_shadow, fill, rounded, scrollbar},
    theme::Theme,
    widgets::Pic,
};

/// The fields, for focus and the tests.
pub const HOMESERVER: &str = "sign-in-homeserver";
pub const USERNAME: &str = "sign-in-username";
pub const PASSWORD: &str = "sign-in-password";
pub const TOKEN: &str = "sign-in-token";
pub const RECOVERY: &str = "sign-in-recovery";
pub const MFA: &str = "sign-in-mfa";

/// What the sign-in screen holds as it's filled in.
pub struct SignIn {
    pub homeserver: String,
    pub username: String,
    pub password: String,
    /// The pasted Discord token, and whether its field is open: kept
    /// outside the linking's phases, so a token Discord refused stays to
    /// fix.
    pub token: String,
    pub token_open: bool,
    /// The password's field shows its text, and the two-factor code typed.
    pub show_password: bool,
    pub show_token: bool,
    pub mfa: String,
    /// A Matrix recovery key, in the verify banner.
    pub recovery: String,
    /// A saved session can be tried again: worked out while the screen
    /// shows, as finding out reads the disk or the credential store.
    pub restore_available: bool,
    /// The QR code shown, the address it carries, and its side in pixels.
    pub code: Option<(String, iced::advanced::image::Handle, f32)>,
    /// The window opened on a saved account that's being restored: until
    /// it's in, or it fails, the window shows only the core's notice
    /// ("Connecting to Discord..."), not the card, whose text at its own
    /// sizes would stay in the renderer's glyph cache for nothing.
    pub starting: bool,
    /// The core's reset last taken in (`AppState::reset_version`).
    reset: u64,
}

impl Default for SignIn {
    fn default() -> Self {
        Self {
            homeserver: "https://matrix.org".into(),
            username: String::new(),
            password: String::new(),
            token: String::new(),
            token_open: false,
            show_password: false,
            show_token: false,
            mfa: String::new(),
            recovery: String::new(),
            restore_available: false,
            code: None,
            starting: false,
            reset: 0,
        }
    }
}

impl std::fmt::Debug for SignIn {
    /// Without the password, token or recovery key a log line could keep.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(backcord_iced_kit::i18n::tr("SignIn"))
            .field("homeserver", &self.homeserver)
            .field("username", &self.username)
            .field("token_open", &self.token_open)
            .field("restore_available", &self.restore_available)
            .field("starting", &self.starting)
            .finish_non_exhaustive()
    }
}

/// Lets go of a secret typed in, its bytes overwritten first so the freed
/// memory doesn't keep it.
pub fn forget(secret: &mut String) {
    let mut bytes = std::mem::take(secret).into_bytes();
    bytes.fill(0);
}

/// What's done on the sign-in screen and the verify banner.
#[derive(Clone)]
pub enum Event {
    Homeserver(String),
    Username(String),
    Password(String),
    Login,
    /// Discord: sign in with the email or phone and password, show or hide
    /// the password, type and send the two-factor code.
    DiscordLogin,
    TogglePassword,
    ToggleToken,
    Mfa(String),
    LinkMfa,
    /// Leave the code, captcha or two-factor step for the form.
    Back,
    Restore,
    Preview,
    /// Discord: show the code (or a new one), open the token's field,
    /// write the token, link with it, open the captcha again.
    ShowCode,
    OpenToken,
    Token(String),
    LinkToken,
    OpenCaptcha,
    /// Matrix's verify banner.
    Recovery(String),
    Recover,
    RefreshTrust,
}

impl std::fmt::Debug for Event {
    /// The secrets typed in show as their kind only.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Event::Homeserver(text) => return write!(formatter, "Homeserver({text:?})"),
            Event::Username(text) => return write!(formatter, "Username({text:?})"),
            Event::Password(_) => backcord_iced_kit::i18n::tr("Password(..)"),
            Event::Login => backcord_iced_kit::i18n::tr("Login"),
            Event::DiscordLogin => "DiscordLogin",
            Event::TogglePassword => "TogglePassword",
            Event::ToggleToken => "ToggleToken",
            Event::Mfa(_) => "Mfa(..)",
            Event::LinkMfa => "LinkMfa",
            Event::Back => "Back",
            Event::Restore => backcord_iced_kit::i18n::tr("Restore"),
            Event::Preview => backcord_iced_kit::i18n::tr("Preview"),
            Event::ShowCode => backcord_iced_kit::i18n::tr("ShowCode"),
            Event::OpenToken => backcord_iced_kit::i18n::tr("OpenToken"),
            Event::Token(_) => backcord_iced_kit::i18n::tr("Token(..)"),
            Event::LinkToken => backcord_iced_kit::i18n::tr("LinkToken"),
            Event::OpenCaptcha => backcord_iced_kit::i18n::tr("OpenCaptcha"),
            Event::Recovery(_) => backcord_iced_kit::i18n::tr("Recovery(..)"),
            Event::Recover => backcord_iced_kit::i18n::tr("Recover"),
            Event::RefreshTrust => backcord_iced_kit::i18n::tr("RefreshTrust"),
        };
        formatter.write_str(name)
    }
}

/// Whether this build signs in with Matrix; one without links Discord.
pub const MATRIX: bool = cfg!(feature = "matrix");

/// Whether the window shows the workspace: signed in, or previewing.
pub fn workspace(app: &App) -> bool {
    let state = app.client.state();
    state.preview || state.account.is_some()
}

impl SignIn {
    pub fn update(&mut self, event: Event, cx: &mut Cx<'_>) -> Task<Message> {
        let busy = cx.state().busy;
        match event {
            Event::Homeserver(text) => self.homeserver = text,
            Event::Username(text) => self.username = text,
            Event::Password(text) => self.password = text,
            Event::Login => {
                let form = &*self;
                cx.dispatch(Action::Login {
                    homeserver: form.homeserver.clone(),
                    username: form.username.clone(),
                    password: form.password.clone(),
                });
                // The password goes once it's on its way.
                if !busy && cx.state().busy {
                    forget(&mut self.password);
                }
                return Task::none();
            }
            Event::DiscordLogin => {
                if !self.username.trim().is_empty() && !self.password.is_empty() {
                    cx.dispatch(Action::LinkDiscordPassword {
                        login: self.username.trim().to_owned(),
                        password: self.password.clone(),
                    });
                    // The password goes once it's on its way.
                    if matches!(cx.state().discord_link, Some(DiscordLink::Checking)) {
                        forget(&mut self.password);
                    }
                }
                return Task::none();
            }
            Event::TogglePassword => self.show_password = !self.show_password,
            Event::ToggleToken => self.show_token = !self.show_token,
            Event::Mfa(text) => self.mfa = text,
            Event::LinkMfa => {
                if !self.mfa.trim().is_empty() {
                    cx.dispatch(Action::LinkDiscordMfa(self.mfa.clone()));
                    forget(&mut self.mfa);
                }
                return Task::none();
            }
            Event::Back => {
                forget(&mut self.mfa);
                cx.dispatch(Action::CancelDiscordLink);
                return Task::none();
            }
            Event::Restore => {
                cx.dispatch(Action::Restore);
                return Task::none();
            }
            Event::Preview => {
                cx.dispatch(Action::Preview);
                return Task::none();
            }
            Event::ShowCode => {
                cx.dispatch(Action::LinkDiscord);
                return Task::none();
            }
            Event::OpenToken => {
                self.token_open = true;
                return operation::focus(TOKEN);
            }
            Event::Token(text) => self.token = text,
            Event::LinkToken => {
                if !self.token.is_empty() {
                    cx.dispatch(Action::LinkDiscordToken(self.token.clone()));
                    return Task::none();
                }
            }
            Event::OpenCaptcha => {
                cx.dispatch(Action::OpenDiscordCaptcha);
                return Task::none();
            }
            Event::Recovery(text) => self.recovery = text,
            Event::Recover => {
                cx.dispatch(Action::Recover(self.recovery.clone()));
                if !busy && cx.state().busy {
                    forget(&mut self.recovery);
                }
                return Task::none();
            }
            Event::RefreshTrust => {
                cx.dispatch(Action::RefreshTrust);
                return Task::none();
            }
        }
        Task::none()
    }

    /// Takes in what changed for the sign-in screen: the QR code to show,
    /// and whether a saved session can be tried again. Back on it with no
    /// account (signed out, the preview left), Matrix's ID takes the keys.
    pub fn sync(&mut self, state: &AppState, root: &std::path::Path) -> Task<Message> {
        let reset = std::mem::replace(&mut self.reset, state.reset_version) != state.reset_version;
        self.take_in(state, root);
        if MATRIX && reset && !state.preview && state.account.is_none() {
            operation::focus(USERNAME)
        } else {
            Task::none()
        }
    }

    fn take_in(&mut self, state: &AppState, root: &std::path::Path) {
        if !state.busy {
            self.starting = false;
        }
        if state.preview || state.account.is_some() {
            self.code = None;
            self.starting = false;
            // Linked: the token pasted, kept until now to fix a refused
            // one, goes with its field.
            if state.account.is_some() && !self.token.is_empty() {
                forget(&mut self.token);
                self.token_open = false;
            }
            return;
        }
        match &state.discord_link {
            Some(DiscordLink::Code(address)) => {
                if self
                    .code
                    .as_ref()
                    .is_none_or(|(shown, ..)| shown != address)
                {
                    self.code = qr_code(address).map(|(code, side)| (address.clone(), code, side));
                }
            }
            _ => self.code = None,
        }
        if MATRIX {
            self.restore_available = backcord_core::chat::can_restore(root);
        }
    }
}

/// The sign-in screen: the wordmark, then the card, centred, on the
/// rail's ground.
pub fn view<'a>(cx: Ctx<'a>) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let state = cx.state;
    let rail = theme.rail;
    if cx.sign_in.starting {
        return container(
            text(backcord_iced_kit::i18n::t(
                state.notice.clone().unwrap_or_default(),
            ))
            .size(14)
            .color(theme.muted),
        )
        .center(Length::Fill)
        .style(move |_| fill(rail))
        .into();
    }
    if !MATRIX {
        return backcord_view(cx);
    }
    let form = matrix_form(cx);
    let divider = bar(theme.line, 1, Length::Fill);
    let mark = cx
        .look
        .glyph(Icon::Logo, 44, theme.accent)
        .boxed(72)
        .style({
            let raised = theme.raised;
            move |_| rounded(raised, 24)
        });
    let preview = column![
        mark,
        text(backcord_iced_kit::i18n::tr("Just looking?"))
            .size(20)
            .font(BOLD)
            .color(theme.text)
            .align_x(Alignment::Center),
        text(backcord_iced_kit::i18n::tr(
            "Try the layout with sample conversations. Nothing leaves this computer."
        ))
        .size(15)
        .color(theme.secondary)
        .align_x(Alignment::Center)
        .width(Length::Fill),
        action(
            theme,
            backcord_iced_kit::i18n::tr("Explore local preview"),
            Kind::Plain,
            (!state.busy).then_some(Message::SignIn(Event::Preview)),
        )
        .wide(36.0),
    ]
    .spacing(12)
    .width(220)
    .align_x(Alignment::Center);
    let card = container(
        // Shrink, so the line between takes the height of what's beside it.
        row![container(form).width(Length::Fill), divider, preview]
            .spacing(32)
            .height(Length::Shrink)
            .align_y(Alignment::Center),
    )
    .padding(32)
    .max_width(760)
    .style({
        let chat = theme.chat;
        move |_| container::Style {
            shadow: card_shadow(),
            ..rounded(chat, 12)
        }
    });
    // That the GPU failed to start shows here too, above the card.
    let body = column![]
        .push(crate::banners::gpu_banner(cx))
        .push(card)
        .spacing(16)
        .max_width(760);
    let page = scroll_view(
        container(body)
            .padding(Padding {
                top: 0.0,
                right: 24.0,
                bottom: 24.0,
                left: 24.0,
            })
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fixed((cx.look.window.height - 64.0).max(0.0))),
    )
    .style(scrollbar(theme))
    .height(Length::Fill);
    let wordmark = container(
        text(backcord_iced_kit::i18n::tr("BACKCORD"))
            .size(24)
            .font(BOLD)
            .color(theme.accent),
    )
    .padding(Padding {
        top: 24.0,
        right: 0.0,
        bottom: 0.0,
        left: 32.0,
    })
    .height(64);
    container(stack![column![wordmark, page]])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| fill(rail))
        .into()
}

/// The BACKCORD sign-in: the big logo on the left, the card on the right
/// with email and password, or Discord itself.
fn backcord_view<'a>(cx: Ctx<'a>) -> Element<'a, Message> {
    use iced::{Background, Border, Gradient, Radians, gradient, widget::button};
    let theme = cx.look.theme;
    let state = cx.state;
    let form = cx.sign_in;
    let busy = state.busy;
    let linking = matches!(
        state.discord_link,
        Some(
            DiscordLink::Checking
                | DiscordLink::Captcha(_)
                | DiscordLink::Mfa { .. }
                | DiscordLink::Starting
                | DiscordLink::Code(_)
                | DiscordLink::Scanned(_)
        )
    );
    let small_logo: Element<'a, Message> = match crate::rail::home_logo(56) {
        Some(logo) => Pic::square(logo, 56.0).into(),
        None => cx.look.glyph(Icon::Logo, 44, theme.accent).into(),
    };
    let heading = column![
        small_logo,
        text("BACKCORD").size(30).font(BOLD).color(theme.text),
        text("© BACKDOOR™").size(12).color(theme.muted),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Length::Fill);
    let mut card = column![heading, Space::new().height(8)]
        .spacing(10)
        .width(Length::Fill);
    if linking {
        card = card.push(link_panel(cx));
        card = card.push(
            action(
                theme,
                "Indietro",
                Kind::Plain,
                Some(Message::SignIn(Event::Back)),
            )
            .wide(40.0),
        );
    } else {
        let open = !busy;
        let mut login = field(theme, USERNAME, "E-mail", &form.username, false, open);
        let mut password = field(
            theme,
            PASSWORD,
            "Password",
            &form.password,
            !form.show_password,
            open,
        );
        if open {
            login = login
                .on_input(|text| Message::SignIn(Event::Username(text)))
                .on_submit(Message::SignIn(Event::DiscordLogin));
            password = password
                .on_input(|text| Message::SignIn(Event::Password(text)))
                .on_submit(Message::SignIn(Event::DiscordLogin));
        }
        let mut token = field(theme, TOKEN, "Token", &form.token, !form.show_token, open);
        if open {
            token = token
                .on_input(|text| Message::SignIn(Event::Token(text)))
                .on_submit(Message::SignIn(Event::LinkToken));
        }
        let token_eye = button(
            container(cx.look.glyph(
                if form.show_token {
                    Icon::EyeOff
                } else {
                    Icon::Eye
                },
                18,
                theme.muted,
            ))
            .center(36),
        )
        .padding(0)
        .on_press(Message::SignIn(Event::ToggleToken))
        .style(|_, _| button::Style::default());
        let eye = button(
            container(cx.look.glyph(
                if form.show_password {
                    Icon::EyeOff
                } else {
                    Icon::Eye
                },
                18,
                theme.muted,
            ))
            .center(36),
        )
        .padding(0)
        .on_press(Message::SignIn(Event::TogglePassword))
        .style(|_, _| button::Style::default());
        let (c1, c2) = (
            theme.accent,
            Color {
                r: 0.95,
                g: 0.35,
                b: 0.1,
                a: 1.0,
            },
        );
        let with_token = !form.token.trim().is_empty();
        let ready =
            open && (with_token || (!form.username.trim().is_empty() && !form.password.is_empty()));
        let go = button(
            container(
                row![
                    text(if busy { "Connessione..." } else { "Accedi" })
                        .size(16)
                        .font(BOLD)
                        .color(Color::WHITE),
                    cx.look.glyph(Icon::ArrowRight, 16, Color::WHITE),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .center(Length::Fill),
        )
        .padding(0)
        .width(Length::Fill)
        .height(44)
        .on_press_maybe(ready.then_some(Message::SignIn(if with_token {
            Event::LinkToken
        } else {
            Event::DiscordLogin
        })))
        .style(move |_, status| {
            let fade = if matches!(status, button::Status::Disabled) {
                0.45
            } else {
                1.0
            };
            let linear = gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, Color { a: fade, ..c1 })
                .add_stop(1.0, Color { a: fade, ..c2 });
            button::Style {
                background: Some(Background::Gradient(Gradient::Linear(linear))),
                text_color: Color::WHITE,
                border: Border::default().rounded(8),
                ..Default::default()
            }
        });
        card = card
            .push(
                text("Entra nel tuo account")
                    .size(20)
                    .font(BOLD)
                    .color(theme.text)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(Space::new().height(4))
            .push(login)
            .push(row![container(password).width(Length::Fill), eye].align_y(Alignment::Center))
            .push(row![container(token).width(Length::Fill), token_eye].align_y(Alignment::Center))
            .push(Space::new().height(4))
            .push(go)
            .push(
                row![
                    container(Space::new())
                        .width(Length::Fill)
                        .height(1)
                        .style({
                            let line = theme.line;
                            move |_| fill(line)
                        }),
                    text("OPPURE").size(12).color(theme.muted),
                    container(Space::new())
                        .width(Length::Fill)
                        .height(1)
                        .style({
                            let line = theme.line;
                            move |_| fill(line)
                        }),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .push(
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("QR CODE"),
                    Kind::Plain,
                    open.then_some(Message::SignIn(Event::ShowCode)),
                )
                .icon(
                    cx.look
                        .icon(Icon::Discord, 18, crate::theme::rgb(theme.text)),
                )
                .wide(44.0),
            );
        if let Some(DiscordLink::Failed(note)) = &state.discord_link {
            card = card.push(crate::form::error(theme, note.clone()).width(Length::Fill));
        }
        if let Some(notice) = &state.notice {
            card = card.push(
                text(backcord_iced_kit::i18n::t(notice))
                    .size(14)
                    .color(if busy { theme.secondary } else { theme.danger })
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            );
        }
        card = card.push(
            text(
                "BACKCORD non è in alcun modo affiliato a DISCORD, sono due client \
                 diversi con potenzialità differenti",
            )
            .size(12)
            .color(theme.muted)
            .align_x(Alignment::Center)
            .width(Length::Fill),
        );
        card = card.push(
            action(
                theme,
                "Anteprima locale",
                Kind::Plain,
                (!busy).then_some(Message::SignIn(Event::Preview)),
            )
            .small(),
        );
        card = card.align_x(Alignment::Center);
    }
    let card = container(card).padding(32).width(420).style({
        let chat = theme.chat;
        move |_| container::Style {
            shadow: card_shadow(),
            ..rounded(chat, 16)
        }
    });
    let art: Element<'a, Message> = match crate::rail::logo_large(420) {
        Some(logo) => Pic::square(logo, 420.0).into(),
        None => Space::new().into(),
    };
    let footer = column![
        text("BACKCORD v1.2.5").size(12).color(theme.muted),
        text("© BACKDOOR™ Tutti i diritti riservati.")
            .size(12)
            .color(theme.muted),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill);
    let body = row![
        container(art)
            .width(Length::FillPortion(1))
            .center_x(Length::Fill),
        container(column![card, footer].spacing(16).align_x(Alignment::Center))
            .width(Length::FillPortion(1))
            .center_x(Length::Fill),
    ]
    .align_y(Alignment::Center);
    let page = scroll_view(
        container(body)
            .padding(24)
            .width(Length::Fill)
            .center_y(Length::Fixed((cx.look.window.height - 16.0).max(0.0))),
    )
    .style(scrollbar(theme))
    .height(Length::Fill);
    let top = container(
        row![
            match crate::rail::home_logo(28) {
                Some(logo) => Pic::square(logo, 28.0).into(),
                None => Element::from(Space::new()),
            },
            text("BACKCORD").size(16).font(BOLD).color(theme.text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(16);
    let ground = Color {
        r: 0.05,
        g: 0.02,
        b: 0.02,
        a: 1.0,
    };
    container(stack![page, column![crate::banners::gpu_banner(cx), top]])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| fill(ground))
        .into()
}

/// Discord's linking, phase by phase: the warning and its buttons (the
/// token's field under them once asked for), the code and what to do with
/// it, the captcha, the token being checked.
pub(crate) fn link_panel<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let form = ctx.sign_in;
    let link = ctx.state.discord_link.as_ref();
    match link {
        None | Some(DiscordLink::Failed(_)) => {
            let warning = container(
                row![
                    ctx.look.glyph(Icon::TriangleAlert, 18, theme.danger),
                    text(
                        "Discord doesn't allow apps like BACKCORD on personal accounts. \
                         Linking puts your account at some risk of being suspended."
                    )
                    .size(14)
                    .color(theme.text)
                    .width(Length::Fill),
                ]
                .spacing(10),
            )
            .padding(12)
            .width(Length::Fill)
            .style({
                let danger = theme.danger;
                move |_| {
                    crate::style::outlined(
                        Color { a: 0.1, ..danger },
                        Color { a: 0.35, ..danger },
                        8.0,
                    )
                }
            });
            let mut panel = column![warning].spacing(12).width(Length::Fill);
            if let Some(DiscordLink::Failed(note)) = link {
                panel = panel.push(crate::form::error(theme, note.clone()).width(Length::Fill));
            }
            let again = link.is_some() && !form.token_open;
            let mut buttons = row![action(
                theme,
                if again {
                    backcord_iced_kit::i18n::tr("Try again")
                } else {
                    backcord_iced_kit::i18n::tr("Show the code")
                },
                if form.token_open {
                    Kind::Plain
                } else {
                    Kind::Primary
                },
                Some(Message::SignIn(Event::ShowCode)),
            )]
            .spacing(8);
            if !form.token_open {
                buttons = buttons.push(action(
                    theme,
                    backcord_iced_kit::i18n::tr("Use a token"),
                    Kind::Plain,
                    Some(Message::SignIn(Event::OpenToken)),
                ));
            }
            panel = panel.push(buttons);
            if form.token_open {
                panel = panel.push(
                    column![
                        label(theme, backcord_iced_kit::i18n::tr("TOKEN")),
                        row![
                            field(
                                theme,
                                TOKEN,
                                backcord_iced_kit::i18n::tr("Discord token"),
                                &form.token,
                                true,
                                true
                            )
                            .on_input(|text| Message::SignIn(Event::Token(text)))
                            .on_submit(Message::SignIn(Event::LinkToken)),
                            action(
                                theme,
                                backcord_iced_kit::i18n::tr("Link"),
                                Kind::Primary,
                                (!form.token.is_empty())
                                    .then_some(Message::SignIn(Event::LinkToken)),
                            ),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(
                            "Anyone with your token can use your account. Paste it only \
                             here, and never share it."
                        )
                        .size(13)
                        .color(theme.secondary)
                        .width(Length::Fill),
                    ]
                    .spacing(8),
                );
            }
            panel.into()
        }
        Some(DiscordLink::Mfa { error, .. }) => {
            let mut panel = column![
                text("Codice di verifica")
                    .size(17)
                    .font(BOLD)
                    .color(theme.text),
                text(
                    "Inserisci il codice a 6 cifre della tua app di autenticazione \
                     (o un codice di backup)."
                )
                .size(14)
                .color(theme.secondary)
                .width(Length::Fill),
                field(theme, MFA, "123456", &form.mfa, false, true)
                    .on_input(|text| Message::SignIn(Event::Mfa(text)))
                    .on_submit(Message::SignIn(Event::LinkMfa)),
            ]
            .spacing(8)
            .width(Length::Fill);
            if let Some(error) = error {
                panel = panel.push(
                    crate::form::error(theme, backcord_iced_kit::i18n::t(error))
                        .width(Length::Fill),
                );
            }
            panel
                .push(
                    action(
                        theme,
                        "Conferma",
                        Kind::Primary,
                        (!form.mfa.trim().is_empty()).then_some(Message::SignIn(Event::LinkMfa)),
                    )
                    .wide(40.0),
                )
                .into()
        }
        Some(DiscordLink::Checking) => text("Accesso in corso...")
            .size(15)
            .color(theme.secondary)
            .into(),
        Some(DiscordLink::Captcha(_)) => column![
            text(backcord_iced_kit::i18n::tr(
                "Solve the captcha in your browser"
            ))
            .size(17)
            .font(BOLD)
            .color(theme.text),
            text(
                "Discord vuole verificare che tu sia una persona prima di collegare il tuo \
                 account. BACKCORD ha aperto il controllo nel browser: l'accesso finisce \
                 appena lo risolvi."
            )
            .size(14)
            .color(theme.secondary)
            .width(Length::Fill),
            container(action(
                theme,
                backcord_iced_kit::i18n::tr("Open it again"),
                Kind::Plain,
                Some(Message::SignIn(Event::OpenCaptcha)),
            ))
            .padding(Padding {
                top: 8.0,
                ..Padding::ZERO
            }),
        ]
        .spacing(8)
        .into(),
        Some(DiscordLink::Starting | DiscordLink::Code(_) | DiscordLink::Scanned(_)) => {
            let scanned = match link {
                Some(DiscordLink::Scanned(name)) => Some(name.as_str()),
                _ => None,
            };
            let code = form.code.as_ref().filter(|_| scanned.is_none());
            let inside: Element<'_, Message> = match (code, scanned) {
                (Some((_, code, side)), _) => Pic::square(code.clone(), *side).into(),
                (None, Some(_)) => column![
                    ctx.look.glyph(Icon::Check, 40, theme.live),
                    text(backcord_iced_kit::i18n::tr("Scanned"))
                        .size(15)
                        .font(BOLD)
                        .color(theme.text),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
                .into(),
                (None, None) => text(backcord_iced_kit::i18n::tr("Getting a code..."))
                    .size(14)
                    .color(theme.muted)
                    .into(),
            };
            let ground = if code.is_some() {
                Color::WHITE
            } else {
                theme.sunken
            };
            let square = container(inside)
                .center(184)
                .style(move |_| rounded(ground, 12));
            let (heading, words) = match scanned {
                Some(name) => (
                    backcord_iced_kit::i18n::tr("Confirm on your phone").to_owned(),
                    format!("Collego {name}. Tocca Accedi sul telefono per finire."),
                ),
                None => (
                    backcord_iced_kit::i18n::tr("Scan it with the Discord app").to_owned(),
                    "Sul telefono apri Discord, vai in Impostazioni, tocca Scansiona codice \
                     QR e inquadra questo codice. Dopo un paio di minuti ne arriva uno nuovo."
                        .to_owned(),
                ),
            };
            row![
                square,
                column![
                    text(heading).size(17).font(BOLD).color(theme.text),
                    text(words)
                        .size(14)
                        .color(theme.secondary)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .width(Length::Fill)
            ]
            .spacing(20)
            .align_y(Alignment::Center)
            .into()
        }
    }
}

/// Matrix's sign-in, in a build with Matrix.
fn matrix_form<'a>(cx: Ctx<'a>) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let state = cx.state;
    let form = &cx.sign_in;
    let open = !state.busy;
    let input = |id, placeholder, value, secure| field(theme, id, placeholder, value, secure, open);
    let mut homeserver = input(HOMESERVER, "", &form.homeserver, false);
    let mut username = input(USERNAME, "@you:matrix.org", &form.username, false);
    let mut password = input(PASSWORD, "", &form.password, true);
    if open {
        homeserver = homeserver.on_input(|text| Message::SignIn(Event::Homeserver(text)));
        username = username.on_input(|text| Message::SignIn(Event::Username(text)));
        password = password
            .on_input(|text| Message::SignIn(Event::Password(text)))
            .on_submit(Message::SignIn(Event::Login));
    }
    let mut form = column![
        title(theme, backcord_iced_kit::i18n::tr("Sign in to BACKCORD")),
        text(backcord_iced_kit::i18n::tr(
            "Use your existing Matrix account."
        ))
        .color(theme.secondary)
        .align_x(Alignment::Center)
        .width(Length::Fill),
        Space::new().height(12),
        label(theme, backcord_iced_kit::i18n::tr("HOMESERVER")),
        homeserver,
        Space::new().height(4),
        label(theme, backcord_iced_kit::i18n::tr("MATRIX ID")),
        username,
        Space::new().height(4),
        label(theme, backcord_iced_kit::i18n::tr("PASSWORD")),
        password,
        Space::new().height(12),
        action(
            theme,
            if state.busy {
                backcord_iced_kit::i18n::tr("Connecting...")
            } else {
                backcord_iced_kit::i18n::tr("Sign in")
            },
            Kind::Primary,
            open.then_some(Message::SignIn(Event::Login)),
        )
        .wide(44.0),
    ]
    .spacing(8)
    .width(Length::Fill);
    if let Some(notice) = &state.notice {
        form = form.push(
            text(backcord_iced_kit::i18n::t(notice))
                .size(15)
                .color(theme.danger)
                .width(Length::Fill),
        );
    }
    if cx.sign_in.restore_available && open {
        form = form.push(
            action(
                theme,
                backcord_iced_kit::i18n::tr("Retry saved session"),
                Kind::Plain,
                Some(Message::SignIn(Event::Restore)),
            )
            .wide(36.0),
        );
    }
    form.push(
        text(backcord_iced_kit::i18n::tr(
            "Sign in with your Matrix password. Single sign-on isn't available yet.",
        ))
        .size(13)
        .color(theme.muted)
        .width(Length::Fill),
    )
    .into()
}

/// Above the conversation until this Matrix device is verified: a
/// recovery key verifies it, or verifying it in another client and
/// checking again.
pub fn verify_banner<'a>(cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let state = cx.state;
    if state.preview || state.trusted || !MATRIX {
        return None;
    }
    let theme = cx.look.theme;
    let open = !state.busy;
    let mut key = field(
        theme,
        RECOVERY,
        backcord_iced_kit::i18n::tr("Recovery key"),
        &cx.sign_in.recovery,
        true,
        open,
    );
    if open {
        key = key
            .on_input(|text| Message::SignIn(Event::Recovery(text)))
            .on_submit(Message::SignIn(Event::Recover));
    }
    let lock = cx
        .look
        .glyph(Icon::Lock, 18, theme.accent)
        .boxed(36)
        .style({
            let soft = theme.accent_soft;
            move |_| rounded(soft, 18)
        });
    let raised = theme.raised;
    Some(
        container(
            row![
                lock,
                column![
                    text(backcord_iced_kit::i18n::tr(
                        "Verify this device to send messages"
                    ))
                    .font(BOLD)
                    .color(theme.text),
                    text(
                        "Use your Matrix recovery key, or verify BACKCORD in another client \
                         and check again."
                    )
                    .size(14)
                    .color(theme.secondary),
                    row![
                        key,
                        action(
                            theme,
                            backcord_iced_kit::i18n::tr("Recover"),
                            Kind::Primary,
                            open.then_some(Message::SignIn(Event::Recover)),
                        ),
                        action(
                            theme,
                            backcord_iced_kit::i18n::tr("Check again"),
                            Kind::Plain,
                            open.then_some(Message::SignIn(Event::RefreshTrust)),
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(8),
            ]
            .spacing(14),
        )
        .padding(16)
        .width(Length::Fill)
        .style(move |_| fill(raised))
        .into(),
    )
}

/// The card's heading.
fn title<'a>(theme: &Theme, words: &'a str) -> Element<'a, Message> {
    text(words)
        .size(26)
        .font(BOLD)
        .color(theme.text)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// A field's label over it.
fn label<'a>(theme: &Theme, words: &'a str) -> Element<'a, Message> {
    text(words)
        .size(12)
        .font(BOLD)
        .color(theme.secondary)
        .into()
}

/// The QR code Discord's phone app scans to link, black on white with the
/// blank margin scanners need, at whole pixels a module, up to 176 px.
/// Grey pixels read the same in either renderer's format.
pub fn qr_code(address: &str) -> Option<(iced::advanced::image::Handle, f32)> {
    /// The blank margin scanners need, in modules.
    const QUIET: usize = 4;
    let code = qrcode::QrCode::new(address.as_bytes()).ok()?;
    let width = code.width();
    let modules = code.to_colors();
    let side = width + 2 * QUIET;
    let scale = (176 / side).max(1);
    let pixels_side = side * scale;
    let mut pixels = vec![255u8; pixels_side * pixels_side * 4];
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        let (x, y) = (index % pixels_side / scale, index / pixels_side / scale);
        let dark = (QUIET..QUIET + width).contains(&x)
            && (QUIET..QUIET + width).contains(&y)
            && modules[(y - QUIET) * width + (x - QUIET)] == qrcode::Color::Dark;
        if dark {
            pixel[..3].fill(0);
        }
    }
    let handle =
        iced::advanced::image::Handle::from_rgba(pixels_side as u32, pixels_side as u32, pixels);
    Some((handle, pixels_side as f32))
}
