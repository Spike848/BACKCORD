//! My Account's account security, as Discord's My Account has it: the
//! username, the email address and the password, each changed in a dialog
//! with the current password, and two-factor authentication with an
//! authenticator app, turned on with its secret's QR code, off, or its
//! backup codes shown. The core keeps the dialogs (`AppState::security`);
//! this is their view and what their controls do. Discord may confirm a
//! change with two-factor authentication first, which its own prompt asks.

use std::time::Duration;

use backcord_core::application::{
    AccountDialog, AccountField, AccountStep, Action, LogOut, SecurityInput,
};
use iced::{
    Alignment, Element, Length, Task,
    widget::{column, container, operation, row, text},
};

use crate::settings::{Event, Page, Settings};
use crate::{
    app::Message,
    controls::{Kind, action, field},
    ctx::Cx,
    fonts::BOLD,
    form,
    widgets::Pic,
};

/// The dialogs' fields, for focus and the tests.
pub const NAME_FIELD: &str = "account-name";
pub const PASSWORD_FIELD: &str = "account-password";
pub const NEW_PASSWORD_FIELD: &str = "account-new-password";
pub const CONFIRM_FIELD: &str = "account-confirm";
pub const CODE_FIELD: &str = "account-code";

/// How long the username written rests before Discord is asked whether
/// it's taken, as Discord's client waits.
const CHECK_AFTER: Duration = Duration::from_millis(500);

/// What's done on My Account's security, on Devices and in their dialogs.
#[derive(Clone)]
pub enum SecurityEvent {
    ListDevices,
    AskLogOut(Option<LogOut>),
    ConfirmLogOut,
    Edit(Option<AccountDialog>),
    Write(AccountField, String),
    /// The username written, once it rested.
    CheckUsername(String),
    Submit,
    /// Copies the secret or the backup codes.
    Copy(String),
}

/// Names the event alone: what's typed is a password or a code.
impl std::fmt::Debug for SecurityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ListDevices => "ListDevices",
            Self::AskLogOut(_) => "AskLogOut",
            Self::ConfirmLogOut => "ConfirmLogOut",
            Self::Edit(_) => "Edit",
            Self::Write(..) => "Write",
            Self::CheckUsername(_) => "CheckUsername",
            Self::Submit => "Submit",
            Self::Copy(_) => "Copy",
        })
    }
}

pub(in crate::settings) fn security(event: SecurityEvent) -> Message {
    Message::Settings(Event::Security(event))
}

impl Settings {
    pub(in crate::settings) fn security_event(
        &self,
        event: SecurityEvent,
        cx: &mut Cx<'_>,
    ) -> Task<Message> {
        let input = match event {
            SecurityEvent::ListDevices => SecurityInput::ListDevices,
            SecurityEvent::AskLogOut(asking) => SecurityInput::AskLogOut(asking),
            SecurityEvent::ConfirmLogOut => SecurityInput::ConfirmLogOut,
            SecurityEvent::Edit(dialog) => {
                cx.dispatch(Action::Security(SecurityInput::Edit(dialog)));
                cx.touch();
                return match dialog {
                    Some(dialog) => operation::focus(first_field(dialog, cx)),
                    None => Task::none(),
                };
            }
            SecurityEvent::Write(field, written) => {
                let check = field == AccountField::Name
                    && cx
                        .state()
                        .security
                        .editor
                        .as_ref()
                        .is_some_and(|editor| editor.editing == Some(AccountDialog::Username));
                let rested = check.then(|| written.clone());
                cx.dispatch(Action::Security(SecurityInput::Write(field, written)));
                return match rested {
                    Some(written) => {
                        crate::after(CHECK_AFTER, security(SecurityEvent::CheckUsername(written)))
                    }
                    None => Task::none(),
                };
            }
            SecurityEvent::CheckUsername(written) => {
                let still = cx
                    .state()
                    .security
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.name == written);
                if !still {
                    return Task::none();
                }
                SecurityInput::CheckUsername
            }
            SecurityEvent::Submit => SecurityInput::Submit,
            SecurityEvent::Copy(words) => return iced::clipboard::write(words),
        };
        cx.dispatch(Action::Security(input));
        Task::none()
    }
}

/// The field a dialog's first step starts in.
fn first_field(dialog: AccountDialog, cx: &Cx<'_>) -> &'static str {
    let step = cx
        .state()
        .security
        .editor
        .as_ref()
        .and_then(|editor| editor.step);
    match (dialog, step) {
        (AccountDialog::Username, _) => NAME_FIELD,
        (AccountDialog::Email, Some(AccountStep::Details)) => NAME_FIELD,
        (AccountDialog::Email, _) => CODE_FIELD,
        _ => PASSWORD_FIELD,
    }
}

/// A secret as authenticator apps show it, in groups of four.
fn grouped(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    chars
        .chunks(4)
        .map(|group| group.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

impl<'a> Page<'a> {
    /// My Account's account and its security, under the account's card,
    /// for a Discord account.
    pub(in crate::settings) fn account_security(self) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let app = self.cx.app;
        let shown = &self.cx.state.security;
        let tr = backcord_iced_kit::i18n::tr;
        let username = app.own_username();
        let edit = |dialog| Some(security(SecurityEvent::Edit(Some(dialog))));
        let line = |label: &'static str, value: String, button: &'static str, dialog| {
            form::beside(
                column![
                    text(tr(label)).size(12).font(BOLD).color(theme.muted),
                    text(value).size(15).color(theme.text),
                ]
                .spacing(2),
                action(theme, tr(button), Kind::Plain, edit(dialog)),
            )
        };
        let mut parts = vec![form::heading(theme, tr("ACCOUNT"))];
        if let Some(notice) = &shown.notice {
            parts.push(form::note(
                theme,
                backcord_iced_kit::i18n::t(notice.clone()),
            ));
        }
        let shown_name = if username.is_empty() {
            self.cx.account.name.clone()
        } else {
            username
        };
        parts.push(form::card(
            theme,
            column![
                line("USERNAME", shown_name, "Edit", AccountDialog::Username),
                line(
                    "EMAIL",
                    tr("Discord emails you here.").to_owned(),
                    "Edit",
                    AccountDialog::Email
                ),
            ]
            .spacing(16),
        ));
        parts.push(form::heading(theme, tr("PASSWORD AND AUTHENTICATION")));
        let on = app.two_factor_on();
        let mut buttons = vec![
            action(
                theme,
                tr("Change Password"),
                Kind::Primary,
                edit(AccountDialog::Password),
            )
            .into(),
        ];
        let status = if on {
            buttons.push(
                action(
                    theme,
                    tr("View Backup Codes"),
                    Kind::Plain,
                    edit(AccountDialog::BackupCodes),
                )
                .into(),
            );
            buttons.push(
                action(
                    theme,
                    tr("Remove 2FA"),
                    Kind::Danger,
                    edit(AccountDialog::DisableTwoFactor),
                )
                .into(),
            );
            tr(
                "Two-factor authentication is on: signing in and changing what Discord protects asks for a code from your authenticator app.",
            )
        } else {
            buttons.push(
                action(
                    theme,
                    tr("Enable Authenticator App"),
                    Kind::Plain,
                    edit(AccountDialog::EnableTwoFactor),
                )
                .into(),
            );
            tr(
                "Two-factor authentication is off. An authenticator app adds a code to your password, so your account stays yours if your password leaks.",
            )
        };
        parts.push(form::group([
            form::note(theme, status),
            form::buttons(buttons),
        ]));
        parts
    }

    /// The dialog that changes the account, while it's open.
    pub(in crate::settings) fn account_dialog(self) -> Option<Element<'a, Message>> {
        let editor = self.cx.state.security.editor.as_ref()?;
        let (dialog, step) = (editor.editing?, editor.step?);
        let look = self.cx.look;
        let theme = look.theme;
        let tr = backcord_iced_kit::i18n::tr;
        let open = !editor.busy;
        let close = security(SecurityEvent::Edit(None));
        let note = |words: &'a str| form::note(theme, words);
        let entry = |id: &'static str,
                     label: &'static str,
                     value: &'a str,
                     secure: bool,
                     field_of: AccountField|
         -> Element<'a, Message> {
            let mut input = field(theme, id, tr(label), value, secure, open)
                .size(15)
                .padding([8, 10]);
            if open {
                input = input
                    .on_input(move |written| security(SecurityEvent::Write(field_of, written)));
                if editor.ready() {
                    input = input.on_submit(security(SecurityEvent::Submit));
                }
            }
            column![
                text(tr(label)).size(12).font(BOLD).color(theme.muted),
                input
            ]
            .spacing(6)
            .into()
        };
        let password = || {
            entry(
                PASSWORD_FIELD,
                "Current Password",
                editor.password.as_str(),
                true,
                AccountField::Password,
            )
        };
        let (title, mut body, done): (&str, Vec<Element<'a, Message>>, &str) = match (dialog, step)
        {
            (AccountDialog::Username, _) => {
                let mut body = vec![
                    note(tr("Enter a new username and your existing password.")),
                    entry(
                        NAME_FIELD,
                        "Username",
                        editor.name.as_str(),
                        false,
                        AccountField::Name,
                    ),
                ];
                let name = editor.name.trim();
                if editor.problem().is_none()
                    && editor
                        .taken
                        .as_ref()
                        .is_some_and(|(asked, taken)| !taken && asked == name)
                {
                    body.push(
                        text(tr("Username is available!"))
                            .size(13)
                            .color(theme.live)
                            .into(),
                    );
                }
                body.push(password());
                (tr("Change your username"), body, tr("Done"))
            }
            (AccountDialog::Password, _) => (
                tr("Update your password"),
                vec![
                    note(tr("Enter your current password and a new password.")),
                    password(),
                    entry(
                        NEW_PASSWORD_FIELD,
                        "New Password",
                        editor.new_password.as_str(),
                        true,
                        AccountField::NewPassword,
                    ),
                    entry(
                        CONFIRM_FIELD,
                        "Confirm New Password",
                        editor.confirm.as_str(),
                        true,
                        AccountField::Confirm,
                    ),
                ],
                tr("Done"),
            ),
            (AccountDialog::Email, AccountStep::Code) => (
                tr("Verify email address"),
                vec![
                    note(if editor.busy && editor.code.is_empty() {
                        tr("Discord is emailing a code to your current address...")
                    } else {
                        tr(
                            "Discord emailed a verification code to your current address. Enter it below to change it.",
                        )
                    }),
                    entry(
                        CODE_FIELD,
                        "Verification Code",
                        editor.code.as_str(),
                        false,
                        AccountField::Code,
                    ),
                ],
                tr("Next"),
            ),
            (AccountDialog::Email, _) => (
                tr("Enter an email address"),
                vec![
                    note(tr(
                        "Enter a new email address and your existing password. Discord sends a link to verify it.",
                    )),
                    entry(
                        NAME_FIELD,
                        "Email",
                        editor.name.as_str(),
                        false,
                        AccountField::Name,
                    ),
                    password(),
                ],
                tr("Done"),
            ),
            (AccountDialog::EnableTwoFactor, AccountStep::Password | AccountStep::Details) => (
                tr("Enable two-factor authentication"),
                vec![note(tr("Enter your password to continue.")), password()],
                tr("Continue"),
            ),
            (AccountDialog::EnableTwoFactor, AccountStep::Code) => {
                let address = backcord_core::application::totp_address(
                    &editor.secret,
                    &self.cx.app.own_username(),
                );
                let mut body = vec![note(tr(
                    "Scan the QR code with an authenticator app, such as Google Authenticator or Authy, or enter the key below in it. Then enter the 6-digit code it shows.",
                ))];
                let mut shown = row![].spacing(16).align_y(Alignment::Center);
                if let Some((code, side)) = self.s.qr_code(&address) {
                    shown = shown.push(Pic::square(code, side));
                }
                shown = shown.push(
                    column![
                        text(tr("KEY")).size(12).font(BOLD).color(theme.muted),
                        text(grouped(&editor.secret))
                            .size(14)
                            .font(iced::Font::MONOSPACE)
                            .color(theme.text),
                        action(
                            theme,
                            tr("Copy"),
                            Kind::Plain,
                            Some(security(SecurityEvent::Copy(editor.secret.clone()))),
                        ),
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                );
                body.push(shown.into());
                body.push(entry(
                    CODE_FIELD,
                    "6-digit code",
                    editor.code.as_str(),
                    false,
                    AccountField::Code,
                ));
                (tr("Enable two-factor authentication"), body, tr("Activate"))
            }
            (_, AccountStep::Codes) => {
                let mut list = column![].spacing(4);
                for pair in editor.codes.chunks(2) {
                    let mut line = row![].spacing(24);
                    for code in pair {
                        let (words, ink) = if code.consumed {
                            (
                                backcord_iced_kit::i18n::t(format!("{} (used)", code.code)),
                                theme.faint,
                            )
                        } else {
                            (code.code.clone(), theme.text)
                        };
                        line = line.push(
                            text(words)
                                .size(15)
                                .font(iced::Font::MONOSPACE)
                                .color(ink)
                                .width(Length::Fill),
                        );
                    }
                    list = list.push(line);
                }
                let sunken = theme.sunken;
                let codes: Vec<&str> = editor
                    .codes
                    .iter()
                    .filter(|code| !code.consumed)
                    .map(|code| code.code.as_str())
                    .collect();
                (
                    tr("Backup codes"),
                    vec![
                        note(tr(
                            "Each code signs you in once, if you lose your authenticator app. Keep them somewhere safe: anyone with them can get into your account.",
                        )),
                        container(list)
                            .padding(12)
                            .width(Length::Fill)
                            .style(move |_| crate::style::rounded(sunken, 8))
                            .into(),
                        form::buttons(vec![
                            action(
                                theme,
                                tr("Copy codes"),
                                Kind::Plain,
                                Some(security(SecurityEvent::Copy(codes.join("\n")))),
                            )
                            .into(),
                        ]),
                    ],
                    tr("Done"),
                )
            }
            (AccountDialog::DisableTwoFactor, _) => (
                tr("Remove 2FA"),
                vec![note(tr(
                    "Your account will be less safe without two-factor authentication. Discord asks for a code from your authenticator app first.",
                ))],
                tr("Remove 2FA"),
            ),
            (AccountDialog::BackupCodes, AccountStep::Password) => (
                tr("View backup codes"),
                vec![
                    note(tr(
                        "Enter your password. Discord then emails you a key that shows your backup codes.",
                    )),
                    password(),
                ],
                tr("Send Key"),
            ),
            (AccountDialog::BackupCodes, _) => (
                tr("View backup codes"),
                vec![
                    note(tr("Enter the key Discord emailed you.")),
                    entry(
                        CODE_FIELD,
                        "Key",
                        editor.code.as_str(),
                        false,
                        AccountField::Code,
                    ),
                ],
                tr("View Codes"),
            ),
        };
        if let Some(problem) = editor.problem() {
            body.push(form::error(theme, tr(problem)).width(Length::Fill).into());
        }
        if let Some(error) = &editor.error {
            body.push(
                form::error(theme, backcord_iced_kit::i18n::t(error.clone()))
                    .width(Length::Fill)
                    .into(),
            );
        }
        let finished = step == AccountStep::Codes;
        let kind = if dialog == AccountDialog::DisableTwoFactor {
            Kind::Danger
        } else {
            Kind::Primary
        };
        let mut buttons: Vec<Element<'a, Message>> = vec![];
        if !finished {
            buttons.push(
                action(
                    theme,
                    tr("Cancel"),
                    Kind::Ghost,
                    open.then(|| close.clone()),
                )
                .into(),
            );
        }
        let ready = finished || editor.ready();
        buttons.push(
            action(
                theme,
                if editor.busy { tr("Saving...") } else { done },
                kind,
                ready.then(|| security(SecurityEvent::Submit)),
            )
            .into(),
        );
        let mut parts = vec![look.dialog_head(title, close.clone())];
        parts.extend(body);
        parts.push(backcord_iced_kit::dialog::buttons(buttons));
        let width = (look.window.width - 48.0).min(backcord_iced_kit::dialog::CONFIRM_WIDTH + 40.0);
        Some(look.framed(parts, width, if open { close } else { Message::Nothing }))
    }
}
