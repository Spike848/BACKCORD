//! My Account's account security and the Devices page, as Discord's
//! settings have them: the devices signed in to the account, each signed
//! out after asking, or all but this one; and dialogs that change the
//! username, the password or the email address, and turn two-factor
//! authentication with an authenticator app on or off or show its backup
//! codes. Passwords and codes typed live here only while their dialog is
//! open. See "Account security" in docs/discord/README.md.

use super::*;
use crate::chat::{AccountAnswer, AccountChange, BackupCode, Device};

/// The shortest and longest password Discord takes.
pub const PASSWORD_LENGTH: std::ops::RangeInclusive<usize> = 8..=72;
/// The shortest and longest username Discord takes.
pub const USERNAME_LENGTH: std::ops::RangeInclusive<usize> = 2..=32;

/// The devices signed in to the account, as the Devices page shows them.
#[derive(Clone, Debug, Default)]
pub struct Devices {
    /// Those Discord listed, this one first; none until listed.
    pub list: Option<Vec<Device>>,
    pub loading: bool,
    pub error: Option<String>,
    /// Signing out being asked about, before it's confirmed.
    pub asking: Option<LogOut>,
    /// Waiting for Discord to sign them out.
    pub busy: bool,
}

/// What signing out signs out: one device by its session's hash, or every
/// one but this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogOut {
    One(String),
    Others,
}

/// The dialogs that change the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    Username,
    Password,
    Email,
    EnableTwoFactor,
    DisableTwoFactor,
    BackupCodes,
}

/// Where a dialog is: asking the password first, a code (emailed, or the
/// authenticator app's), what's changed, or showing the backup codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Password,
    Code,
    Details,
    Codes,
}

/// A field of a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The new username, or the new email address.
    Name,
    Password,
    NewPassword,
    Confirm,
    Code,
}

/// A dialog that changes the account, while it's open.
#[derive(Clone, Default)]
pub struct AccountEditor {
    pub editing: Option<Dialog>,
    pub step: Option<Step>,
    /// The new username or email address, as written.
    pub name: String,
    pub password: String,
    pub new_password: String,
    pub confirm: String,
    pub code: String,
    /// The authenticator app's secret, once the password is checked.
    pub secret: String,
    /// The token the emailed code gave, or the nonce that shows the backup
    /// codes.
    token: Option<String>,
    pub codes: Vec<BackupCode>,
    /// Whether another account has the username written, once Discord
    /// said, and for which.
    pub taken: Option<(String, bool)>,
    pub busy: bool,
    pub error: Option<String>,
}

impl std::fmt::Debug for AccountEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountEditor")
            .field("editing", &self.editing)
            .field("step", &self.step)
            .field("busy", &self.busy)
            .finish_non_exhaustive()
    }
}

impl AccountEditor {
    /// Why what's written can't be sent yet, if it can't, as Discord's
    /// client says before sending; none when it's empty.
    pub fn problem(&self) -> Option<&'static str> {
        let editing = self.editing?;
        match (editing, self.step?) {
            (Dialog::Username, Step::Details) => {
                let name = self.name.trim();
                if !name.is_empty() && !USERNAME_LENGTH.contains(&name.chars().count()) {
                    Some("Must be between 2 and 32 in length.")
                } else if name.chars().any(|c| {
                    !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
                }) {
                    Some(
                        "Usernames can only use lowercase letters, numbers, underscores and periods.",
                    )
                } else if self
                    .taken
                    .as_ref()
                    .is_some_and(|(asked, taken)| *taken && asked == name)
                {
                    Some(
                        "Username is unavailable. Try adding numbers, letters, underscores _ , or periods.",
                    )
                } else {
                    None
                }
            }
            (Dialog::Password, Step::Details) => {
                if !self.new_password.is_empty()
                    && !PASSWORD_LENGTH.contains(&self.new_password.chars().count())
                {
                    Some("Must be between 8 and 72 in length.")
                } else if !self.confirm.is_empty() && self.confirm != self.new_password {
                    Some("Passwords do not match!")
                } else {
                    None
                }
            }
            (Dialog::Email, Step::Details) => {
                let email = self.name.trim();
                let well_formed = email
                    .split_once('@')
                    .is_some_and(|(user, host)| !user.is_empty() && host.contains('.'));
                (!email.is_empty() && !well_formed).then_some("Not a well formed email address.")
            }
            _ => None,
        }
    }

    /// Whether the step can be sent now.
    pub fn ready(&self) -> bool {
        let (Some(editing), Some(step)) = (self.editing, self.step) else {
            return false;
        };
        if self.busy || self.problem().is_some() {
            return false;
        }
        let digits = |code: &str| {
            let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
            code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
        };
        match (editing, step) {
            (_, Step::Password) => !self.password.is_empty(),
            (Dialog::Username, Step::Details) => {
                !self.name.trim().is_empty() && !self.password.is_empty()
            }
            (Dialog::Password, Step::Details) => {
                !self.password.is_empty()
                    && !self.new_password.is_empty()
                    && self.confirm == self.new_password
            }
            (Dialog::Email, Step::Details) => {
                !self.name.trim().is_empty() && !self.password.is_empty()
            }
            (Dialog::Email, Step::Code) => !self.code.trim().is_empty(),
            (Dialog::EnableTwoFactor, Step::Code) => digits(&self.code),
            (Dialog::BackupCodes, Step::Code) => !self.code.trim().is_empty(),
            (Dialog::DisableTwoFactor, Step::Details) => true,
            _ => false,
        }
    }

    /// The change the step sends.
    fn change(&self) -> Option<AccountChange> {
        Some(match (self.editing?, self.step?) {
            (Dialog::Username, Step::Details) => AccountChange::Username {
                username: self.name.trim().to_owned(),
                password: self.password.clone(),
            },
            (Dialog::Password, Step::Details) => AccountChange::Password {
                current: self.password.clone(),
                new: self.new_password.clone(),
            },
            (Dialog::Email, Step::Code) => AccountChange::VerifyEmailCode(self.code.clone()),
            (Dialog::Email, Step::Details) => AccountChange::Email {
                email: self.name.trim().to_owned(),
                password: self.password.clone(),
                token: self.token.clone(),
            },
            (Dialog::EnableTwoFactor, Step::Password) => {
                AccountChange::CheckTotpPassword(self.password.clone())
            }
            (Dialog::EnableTwoFactor, Step::Code) => AccountChange::EnableTotp {
                password: self.password.clone(),
                secret: self.secret.clone(),
                code: self.code.clone(),
            },
            (Dialog::DisableTwoFactor, Step::Details) => AccountChange::DisableTotp,
            (Dialog::BackupCodes, Step::Password) => {
                AccountChange::BackupCodesChallenge(self.password.clone())
            }
            (Dialog::BackupCodes, Step::Code) => AccountChange::BackupCodes {
                key: self.code.clone(),
                nonce: self.token.clone()?,
            },
            _ => return None,
        })
    }
}

/// The account's security as My Account and Devices show it.
#[derive(Clone, Debug, Default)]
pub struct Security {
    pub devices: Devices,
    /// The dialog open, if one is.
    pub editor: Option<AccountEditor>,
    /// What the last change did, said under the account's card.
    pub notice: Option<String>,
}

/// Something done on My Account's security or on Devices.
pub enum SecurityInput {
    /// The Devices page showed: its list is asked for again.
    ListDevices,
    /// Asks before signing a device out, or all but this one; none stops
    /// asking.
    AskLogOut(Option<LogOut>),
    ConfirmLogOut,
    /// Opens a dialog, or with none closes the one open.
    Edit(Option<Dialog>),
    Write(Field, String),
    /// The username written stopped changing: Discord is asked whether
    /// it's taken.
    CheckUsername,
    /// Sends the dialog's step.
    Submit,
}

/// The address an authenticator app's QR code carries for `secret`, named
/// for the account as Discord's client names it.
pub fn totp_address(secret: &str, username: &str) -> String {
    let name: String = username
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect();
    format!("otpauth://totp/Discord:{name}?secret={secret}&issuer=Discord")
}

/// The devices of the preview, which signs them out without Discord.
fn preview_devices() -> Vec<Device> {
    let device = |id: &str, os: &str, platform: &str, location: Option<&str>, current| Device {
        id: id.to_owned(),
        os: Some(os.to_owned()),
        platform: Some(platform.to_owned()),
        location: location.map(str::to_owned),
        // 2026-11-20 at 08:30 UTC.
        last_used: Some(1_795_163_400_000),
        current,
    };
    vec![
        device("this", "Linux", "BACKCORD", Some("Rome, Italy"), true),
        device(
            "phone",
            "Android",
            "Discord Android",
            Some("Milan, Italy"),
            false,
        ),
        device("browser", "Windows", "Chrome", None, false),
    ]
}

impl Application {
    /// Whether the Discord account has two-factor authentication on.
    pub fn two_factor_on(&self) -> bool {
        #[cfg(feature = "discord")]
        return self
            .discord
            .as_ref()
            .and_then(|snapshot| snapshot.account.as_ref())
            .is_some_and(|account| account.mfa_enabled);
        #[cfg(not(feature = "discord"))]
        false
    }

    /// The Discord account's username, for its authenticator app's entry.
    pub fn own_username(&self) -> String {
        #[cfg(feature = "discord")]
        return self
            .discord
            .as_ref()
            .and_then(|snapshot| snapshot.account.as_ref())
            .map(|account| account.username.clone())
            .unwrap_or_default();
        #[cfg(not(feature = "discord"))]
        String::new()
    }

    /// Whether the Discord account's email address is verified, so changing
    /// it starts with a code Discord emails there.
    fn email_verified(&self) -> bool {
        #[cfg(feature = "discord")]
        return self
            .discord
            .as_ref()
            .and_then(|snapshot| snapshot.account.as_ref())
            .is_some_and(|account| account.email_verified);
        #[cfg(not(feature = "discord"))]
        false
    }

    /// This device's session's hash, as READY gave it.
    fn own_session(&self) -> Option<String> {
        #[cfg(feature = "discord")]
        return self
            .discord
            .as_ref()
            .and_then(|snapshot| snapshot.account.as_ref())
            .and_then(|account| account.auth_session.clone());
        #[cfg(not(feature = "discord"))]
        None
    }

    pub(super) fn security_input(&mut self, input: SecurityInput) -> Vec<Effect> {
        match input {
            SecurityInput::ListDevices => self.list_devices(),
            SecurityInput::AskLogOut(asking) => {
                let devices = &mut self.state.security.devices;
                if !devices.busy {
                    devices.asking = asking;
                    devices.error = None;
                }
                vec![]
            }
            SecurityInput::ConfirmLogOut => self.log_out_devices(),
            SecurityInput::Edit(editing) => self.edit_account(editing),
            SecurityInput::Write(field, text) => {
                if let Some(editor) = self.state.security.editor.as_mut().filter(|e| !e.busy) {
                    let slot = match field {
                        Field::Name => &mut editor.name,
                        Field::Password => &mut editor.password,
                        Field::NewPassword => &mut editor.new_password,
                        Field::Confirm => &mut editor.confirm,
                        Field::Code => &mut editor.code,
                    };
                    *slot = text;
                    editor.error = None;
                }
                vec![]
            }
            SecurityInput::CheckUsername => {
                let Some(editor) = self
                    .state
                    .security
                    .editor
                    .as_ref()
                    .filter(|editor| editor.editing == Some(Dialog::Username))
                else {
                    return vec![];
                };
                let name = editor.name.trim();
                let asked = editor
                    .taken
                    .as_ref()
                    .is_some_and(|(asked, _)| asked == name);
                if self.state.preview
                    || asked
                    || !USERNAME_LENGTH.contains(&name.chars().count())
                    || editor.problem().is_some()
                {
                    return vec![];
                }
                vec![Effect::Backend(Command::Discord(
                    DiscordCommand::CheckUsername(name.to_owned()),
                ))]
            }
            SecurityInput::Submit => self.submit_account(),
        }
    }

    /// Asks Discord for the devices again, as the page shows; the preview
    /// lists its own.
    fn list_devices(&mut self) -> Vec<Effect> {
        let devices = &mut self.state.security.devices;
        devices.error = None;
        if self.state.preview {
            if devices.list.is_none() {
                devices.list = Some(preview_devices());
            }
            return vec![];
        }
        if devices.loading {
            return vec![];
        }
        devices.loading = true;
        vec![Effect::Backend(Command::Discord(DiscordCommand::Devices))]
    }

    fn log_out_devices(&mut self) -> Vec<Effect> {
        let devices = &mut self.state.security.devices;
        let Some(asking) = devices.asking.clone().filter(|_| !devices.busy) else {
            return vec![];
        };
        let list = devices.list.as_deref().unwrap_or_default();
        let hashes: Vec<String> = match &asking {
            LogOut::One(hash) => vec![hash.clone()],
            LogOut::Others => list
                .iter()
                .filter(|device| !device.current)
                .map(|device| device.id.clone())
                .collect(),
        };
        // Discord takes up to 64 at a time.
        let hashes: Vec<String> = hashes.into_iter().take(64).collect();
        if hashes.is_empty() {
            devices.asking = None;
            return vec![];
        }
        if self.state.preview {
            devices.asking = None;
            if let Some(list) = devices.list.as_mut() {
                list.retain(|device| !hashes.contains(&device.id));
            }
            return vec![];
        }
        devices.busy = true;
        devices.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::LogOutDevices(hashes),
        ))]
    }

    pub(super) fn devices_listed(&mut self, result: Result<Vec<Device>, String>) {
        let own = self.own_session();
        let devices = &mut self.state.security.devices;
        devices.loading = false;
        match result {
            Ok(mut list) => {
                for device in &mut list {
                    device.current = own.as_deref() == Some(device.id.as_str());
                }
                // This one first, as Discord's page shows it apart.
                list.sort_by_key(|device| !device.current);
                devices.list = Some(list);
                devices.error = None;
            }
            Err(error) => devices.error = Some(error),
        }
    }

    pub(super) fn devices_logged_out(&mut self, hashes: Vec<String>, result: Result<(), String>) {
        let devices = &mut self.state.security.devices;
        devices.busy = false;
        match result {
            Ok(()) => {
                devices.asking = None;
                if let Some(list) = devices.list.as_mut() {
                    list.retain(|device| !hashes.contains(&device.id));
                }
            }
            Err(error) => devices.error = Some(error),
        }
    }

    /// Opens a dialog at its first step: a verified email starts with the
    /// code Discord emails it, sent at once. Closing forgets what was
    /// written, even while Discord answers (as Settings close), whose
    /// answer then changes nothing; another opens only once it's answered.
    fn edit_account(&mut self, editing: Option<Dialog>) -> Vec<Effect> {
        let security = &mut self.state.security;
        let Some(editing) = editing else {
            security.editor = None;
            return vec![];
        };
        if security.editor.as_ref().is_some_and(|editor| editor.busy) {
            return vec![];
        }
        let verified = self.email_verified();
        let step = match editing {
            Dialog::Username | Dialog::Password | Dialog::DisableTwoFactor => Step::Details,
            Dialog::Email if verified => Step::Code,
            Dialog::Email => Step::Details,
            Dialog::EnableTwoFactor | Dialog::BackupCodes => Step::Password,
        };
        let mut editor = AccountEditor {
            editing: Some(editing),
            step: Some(step),
            ..AccountEditor::default()
        };
        let mut effects = vec![];
        if editing == Dialog::Email && verified && !self.state.preview {
            editor.busy = true;
            effects.push(Effect::Backend(Command::Discord(
                DiscordCommand::ChangeAccount(AccountChange::SendEmailCode),
            )));
        }
        let security = &mut self.state.security;
        security.editor = Some(editor);
        security.notice = None;
        effects
    }

    fn submit_account(&mut self) -> Vec<Effect> {
        let preview = self.state.preview;
        let Some(editor) = self.state.security.editor.as_mut() else {
            return vec![];
        };
        if editor.step == Some(Step::Codes) {
            self.state.security.editor = None;
            return vec![];
        }
        if !editor.ready() {
            return vec![];
        }
        let Some(change) = editor.change() else {
            return vec![];
        };
        if preview {
            editor.error = Some("The preview can't change a Discord account.".into());
            return vec![];
        }
        editor.busy = true;
        editor.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ChangeAccount(change),
        ))]
    }

    pub(super) fn username_checked(&mut self, username: String, taken: Result<bool, String>) {
        if let (Some(editor), Ok(taken)) = (self.state.security.editor.as_mut(), taken)
            && editor.editing == Some(Dialog::Username)
        {
            editor.taken = Some((username, taken));
        }
    }

    /// What Discord answered a dialog's step with: the next step, or the
    /// dialog closes with what changed said under the card.
    pub(super) fn account_changed(&mut self, result: Result<AccountAnswer, String>) {
        let security = &mut self.state.security;
        let Some(editor) = security.editor.as_mut() else {
            return;
        };
        editor.busy = false;
        let answer = match result {
            Ok(answer) => answer,
            Err(error) => {
                editor.error = Some(error);
                return;
            }
        };
        let (editing, step) = (editor.editing, editor.step);
        match (editing, step, answer) {
            // The email with the code to change the address went.
            (Some(Dialog::Email), Some(Step::Code), AccountAnswer::Done) => {}
            (Some(Dialog::Email), Some(Step::Code), AccountAnswer::EmailToken(token)) => {
                editor.token = Some(token);
                editor.code.clear();
                editor.step = Some(Step::Details);
            }
            (Some(Dialog::EnableTwoFactor), _, AccountAnswer::PasswordChecked { secret }) => {
                editor.secret = secret;
                editor.step = Some(Step::Code);
            }
            (Some(Dialog::BackupCodes), _, AccountAnswer::BackupNonce(nonce)) => {
                editor.token = Some(nonce);
                editor.step = Some(Step::Code);
            }
            (Some(editing), _, AccountAnswer::BackupCodes(codes)) => {
                // What was typed goes; the codes stay until it closes.
                editor.password.clear();
                editor.code.clear();
                editor.secret.clear();
                editor.token = None;
                editor.codes = codes;
                editor.step = Some(Step::Codes);
                if editing == Dialog::EnableTwoFactor {
                    security.notice = Some("Two-factor authentication is on.".into());
                }
            }
            (Some(editing), _, AccountAnswer::Done) => {
                security.editor = None;
                security.notice = Some(
                    match editing {
                        Dialog::Username => "Your username changed.",
                        Dialog::Password => {
                            "Your password changed. Your other devices were signed out."
                        }
                        Dialog::Email => {
                            "Your email changed. Discord sends a link to verify the new address."
                        }
                        Dialog::DisableTwoFactor => "Two-factor authentication is off.",
                        Dialog::EnableTwoFactor | Dialog::BackupCodes => return,
                    }
                    .into(),
                );
                // A new password signed every other device out.
                if editing == Dialog::Password {
                    security.devices.list = None;
                }
            }
            _ => {}
        }
    }
}
