//! The account's security, as Discord's My Account and Devices settings
//! change it: listing and signing out devices, the username, email and
//! password, and two-factor authentication with an authenticator app.
//! Discord may hold any of them back for two-factor authentication, which
//! asks the person first ([`Discord::stepping_up`]). Passwords and codes
//! go to Discord and nowhere else: never into a log. See "Account security"
//! in docs/discord/README.md.

use backcord_discord::security;

use super::*;
use crate::chat::{AccountAnswer, AccountChange, BackupCode, Device};

/// Carries out a command about the account's security, or gives back any
/// other.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    let work = match command {
        Command::Discord(DiscordCommand::Devices) => Work::Devices,
        Command::Discord(DiscordCommand::LogOutDevices(hashes)) => Work::LogOut(hashes),
        Command::Discord(DiscordCommand::CheckUsername(username)) => Work::Check(username),
        Command::Discord(DiscordCommand::ChangeAccount(change)) => Work::Change(change),
        other => return Some(other),
    };
    let discord = match discord {
        Ok(discord) => discord.clone(),
        Err(why) => {
            let _ = events.send(work.failed(why.to_owned()));
            return None;
        }
    };
    let events = events.clone();
    tokio::spawn(async move {
        let event = work.run(&discord).await;
        let _ = events.send(event);
    });
    None
}

enum Work {
    Devices,
    LogOut(Vec<String>),
    Check(String),
    Change(AccountChange),
}

impl Work {
    fn failed(self, why: String) -> Event {
        match self {
            Work::Devices => Event::Devices(Err(why)),
            Work::LogOut(hashes) => Event::DevicesLoggedOut {
                hashes,
                result: Err(why),
            },
            Work::Check(username) => Event::UsernameChecked {
                username,
                taken: Err(why),
            },
            Work::Change(_) => Event::AccountChanged(Err(why)),
        }
    }

    async fn run(self, discord: &Discord) -> Event {
        let words = |error: anyhow::Error| error.to_string();
        match self {
            Work::Devices => Event::Devices(
                discord
                    .devices()
                    .await
                    .map(|devices| devices.into_iter().map(device).collect())
                    .map_err(|error| format!("Couldn't list your devices: {error}")),
            ),
            Work::LogOut(hashes) => {
                let result = discord
                    .stepping_up(|| discord.log_out_devices(&hashes))
                    .await
                    .map_err(words);
                if let Err(error) = &result {
                    tracing::info!(error, "Discord kept the devices signed in");
                }
                Event::DevicesLoggedOut { hashes, result }
            }
            Work::Check(username) => {
                let taken = discord.username_taken(&username).await.map_err(words);
                Event::UsernameChecked { username, taken }
            }
            Work::Change(change) => {
                let result = change_account(discord, change).await.map_err(words);
                if let Err(error) = &result {
                    tracing::info!(error, "Discord kept the account as it was");
                }
                Event::AccountChanged(result)
            }
        }
    }
}

async fn change_account(discord: &Discord, change: AccountChange) -> anyhow::Result<AccountAnswer> {
    let done = |()| AccountAnswer::Done;
    Ok(match change {
        AccountChange::Username { username, password } => discord
            .stepping_up(|| discord.change_username(&username, &password))
            .await
            .map(done)?,
        AccountChange::Password { current, new } => discord
            .stepping_up(|| discord.change_password(&current, &new))
            .await
            .map(done)?,
        AccountChange::SendEmailCode => discord.send_email_code().await.map(done)?,
        AccountChange::VerifyEmailCode(code) => {
            AccountAnswer::EmailToken(discord.verify_email_code(code.trim()).await?)
        }
        AccountChange::Email {
            email,
            password,
            token,
        } => discord
            .stepping_up(|| discord.change_email(email.trim(), &password, token.as_deref()))
            .await
            .map(done)?,
        AccountChange::CheckTotpPassword(password) => {
            discord.check_totp_password(&password).await?;
            AccountAnswer::PasswordChecked {
                secret: security::new_totp_secret(),
            }
        }
        AccountChange::EnableTotp {
            password,
            secret,
            code,
        } => {
            let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
            let codes = discord.enable_totp(&password, &secret, &code).await?;
            AccountAnswer::BackupCodes(codes.into_iter().map(backup_code).collect())
        }
        AccountChange::DisableTotp => discord
            .stepping_up(|| discord.disable_totp())
            .await
            .map(done)?,
        AccountChange::BackupCodesChallenge(password) => {
            AccountAnswer::BackupNonce(discord.backup_codes_challenge(&password).await?)
        }
        AccountChange::BackupCodes { key, nonce } => {
            let codes = discord.backup_codes(key.trim(), &nonce).await?;
            AccountAnswer::BackupCodes(codes.into_iter().map(backup_code).collect())
        }
    })
}

fn device(device: security::Device) -> Device {
    Device {
        id: device.id_hash,
        os: device.os,
        platform: device.platform,
        location: device.location,
        last_used: device
            .last_used
            .as_deref()
            .and_then(backcord_discord::message::iso_ms),
        current: false,
    }
}

fn backup_code(code: security::BackupCode) -> BackupCode {
    BackupCode {
        code: code.code,
        consumed: code.consumed,
    }
}

#[cfg(test)]
mod tests {
    use super::super::commands::NOT_DISCORD;
    use super::*;

    #[test]
    fn changes_never_show_their_secrets() {
        let change = AccountChange::Password {
            current: "hunter2".into(),
            new: "hunter3".into(),
        };
        assert_eq!(format!("{change:?}"), "Password");
        let answer = AccountAnswer::EmailToken("token".into());
        assert_eq!(format!("{answer:?}"), "EmailToken");
    }

    #[tokio::test]
    async fn without_discord_each_fails_with_why() {
        let (events, mut received) = tokio::sync::mpsc::unbounded_channel();
        let rest = command(
            Command::Discord(DiscordCommand::Devices),
            Err(NOT_DISCORD),
            &events,
        );
        assert!(rest.is_none());
        assert!(matches!(
            received.recv().await,
            Some(Event::Devices(Err(why))) if why == NOT_DISCORD
        ));
    }
}
