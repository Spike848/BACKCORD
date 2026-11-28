//! My Account's account security and the Devices page.

use super::*;
use crate::application::{AccountDialog, AccountField, AccountStep, LogOut, SecurityInput};
use crate::chat::{AccountAnswer, AccountChange, BackupCode, Device};

/// `discord_server` with Alice's account, its email verified or not, and
/// this device's session "this".
fn linked(verified: bool) -> Application {
    let mut app = discord_server();
    let mut snapshot = backcord_discord::service::Snapshot::clone(app.discord.as_ref().unwrap());
    snapshot.account = Some(backcord_discord::service::Account {
        username: "alice".into(),
        name: "Alice".into(),
        avatar: String::new(),
        decoration: None,
        nameplate: None,
        premium_type: 0,
        mfa_enabled: false,
        has_email: true,
        nsfw_allowed: false,
        nsfw_commands_in_dms: false,
        email_verified: verified,
        auth_session: Some("this".into()),
    });
    app.apply(discord_event(snapshot));
    app
}

fn input(app: &mut Application, input: SecurityInput) -> Vec<Effect> {
    app.dispatch(Action::Security(input))
}

fn sent(effects: Vec<Effect>) -> Option<DiscordCommand> {
    match <[_; 1]>::try_from(effects).ok()? {
        [Effect::Backend(Command::Discord(command))] => Some(command),
        _ => None,
    }
}

fn change(effects: Vec<Effect>) -> Option<AccountChange> {
    match sent(effects)? {
        DiscordCommand::ChangeAccount(change) => Some(change),
        _ => None,
    }
}

fn device(id: &str) -> Device {
    Device {
        id: id.into(),
        os: Some("Linux".into()),
        platform: Some("Discord Client".into()),
        location: None,
        last_used: None,
        current: false,
    }
}

fn write(app: &mut Application, field: AccountField, text: &str) {
    input(app, SecurityInput::Write(field, text.into()));
}

#[test]
fn devices_list_this_one_first_and_sign_out_after_asking() {
    let mut app = linked(true);
    assert!(matches!(
        sent(input(&mut app, SecurityInput::ListDevices)),
        Some(DiscordCommand::Devices)
    ));
    assert!(
        input(&mut app, SecurityInput::ListDevices).is_empty(),
        "asked once while it loads"
    );
    app.apply(Event::Devices(Ok(vec![
        device("phone"),
        device("this"),
        device("web"),
    ])));
    let devices = &app.state.security.devices;
    let list = devices.list.as_ref().unwrap();
    assert_eq!(list[0].id, "this");
    assert!(list[0].current && !list[1].current);

    // Nothing goes before it's confirmed.
    assert!(input(&mut app, SecurityInput::ConfirmLogOut).is_empty());
    input(
        &mut app,
        SecurityInput::AskLogOut(Some(LogOut::One("phone".into()))),
    );
    assert!(matches!(
        sent(input(&mut app, SecurityInput::ConfirmLogOut)),
        Some(DiscordCommand::LogOutDevices(hashes)) if hashes == ["phone"]
    ));
    assert!(app.state.security.devices.busy);
    app.apply(Event::DevicesLoggedOut {
        hashes: vec!["phone".into()],
        result: Ok(()),
    });
    let devices = &app.state.security.devices;
    assert!(devices.asking.is_none() && !devices.busy);
    assert_eq!(devices.list.as_ref().unwrap().len(), 2);

    // All but this one; a refusal stays said.
    input(&mut app, SecurityInput::AskLogOut(Some(LogOut::Others)));
    assert!(matches!(
        sent(input(&mut app, SecurityInput::ConfirmLogOut)),
        Some(DiscordCommand::LogOutDevices(hashes)) if hashes == ["web"]
    ));
    app.apply(Event::DevicesLoggedOut {
        hashes: vec!["web".into()],
        result: Err("Two-factor authentication was cancelled.".into()),
    });
    let devices = &app.state.security.devices;
    assert_eq!(
        devices.error.as_deref(),
        Some("Two-factor authentication was cancelled.")
    );
    assert_eq!(devices.list.as_ref().unwrap().len(), 2);
}

#[test]
fn a_username_is_checked_and_changed_with_the_password() {
    let mut app = linked(true);
    input(&mut app, SecurityInput::Edit(Some(AccountDialog::Username)));
    write(&mut app, AccountField::Name, "Bob");
    let editor = app.state.security.editor.as_ref().unwrap();
    assert!(editor.problem().is_some(), "capitals");
    assert!(input(&mut app, SecurityInput::CheckUsername).is_empty());
    write(&mut app, AccountField::Name, "bob");
    assert!(matches!(
        sent(input(&mut app, SecurityInput::CheckUsername)),
        Some(DiscordCommand::CheckUsername(name)) if name == "bob"
    ));
    app.apply(Event::UsernameChecked {
        username: "bob".into(),
        taken: Ok(true),
    });
    write(&mut app, AccountField::Password, "password");
    let editor = app.state.security.editor.as_ref().unwrap();
    assert!(
        editor
            .problem()
            .unwrap()
            .starts_with("Username is unavailable")
    );
    assert!(!editor.ready());
    write(&mut app, AccountField::Name, "alice_2");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::Username { username, password })
            if username == "alice_2" && password == "password"
    ));
    // A wrong password leaves it open with Discord's words.
    app.apply(Event::AccountChanged(Err(
        "Password: Password does not match.".into(),
    )));
    let editor = app.state.security.editor.as_ref().unwrap();
    assert_eq!(
        editor.error.as_deref(),
        Some("Password: Password does not match.")
    );
    assert!(change(input(&mut app, SecurityInput::Submit)).is_some());
    app.apply(Event::AccountChanged(Ok(AccountAnswer::Done)));
    assert!(
        app.state.security.editor.is_none(),
        "closed, forgetting the password"
    );
    assert_eq!(
        app.state.security.notice.as_deref(),
        Some("Your username changed.")
    );
}

#[test]
fn a_new_password_must_match_and_fit() {
    let mut app = linked(true);
    input(&mut app, SecurityInput::Edit(Some(AccountDialog::Password)));
    write(&mut app, AccountField::Password, "old password");
    write(&mut app, AccountField::NewPassword, "short");
    assert_eq!(
        app.state.security.editor.as_ref().unwrap().problem(),
        Some("Must be between 8 and 72 in length.")
    );
    write(&mut app, AccountField::NewPassword, "long enough");
    write(&mut app, AccountField::Confirm, "long enougH");
    assert_eq!(
        app.state.security.editor.as_ref().unwrap().problem(),
        Some("Passwords do not match!")
    );
    assert!(input(&mut app, SecurityInput::Submit).is_empty());
    write(&mut app, AccountField::Confirm, "long enough");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::Password { current, new })
            if current == "old password" && new == "long enough"
    ));
    app.apply(Event::AccountChanged(Ok(AccountAnswer::Done)));
    assert!(app.state.security.editor.is_none());
}

#[test]
fn a_verified_email_changes_after_the_code_emailed_to_it() {
    let mut app = linked(true);
    assert!(matches!(
        change(input(
            &mut app,
            SecurityInput::Edit(Some(AccountDialog::Email))
        )),
        Some(AccountChange::SendEmailCode)
    ));
    app.apply(Event::AccountChanged(Ok(AccountAnswer::Done)));
    let editor = app.state.security.editor.as_ref().unwrap();
    assert_eq!(editor.step, Some(AccountStep::Code));
    write(&mut app, AccountField::Code, "123456");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::VerifyEmailCode(code)) if code == "123456"
    ));
    app.apply(Event::AccountChanged(Ok(AccountAnswer::EmailToken(
        "t".into(),
    ))));
    assert_eq!(
        app.state.security.editor.as_ref().unwrap().step,
        Some(AccountStep::Details)
    );
    write(&mut app, AccountField::Name, "not an address");
    write(&mut app, AccountField::Password, "password");
    assert!(input(&mut app, SecurityInput::Submit).is_empty());
    write(&mut app, AccountField::Name, "alice@new.example");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::Email { email, token: Some(token), .. })
            if email == "alice@new.example" && token == "t"
    ));

    // An unverified one changes at once.
    let mut app = linked(false);
    assert!(input(&mut app, SecurityInput::Edit(Some(AccountDialog::Email))).is_empty());
    assert_eq!(
        app.state.security.editor.as_ref().unwrap().step,
        Some(AccountStep::Details)
    );
}

#[test]
fn two_factor_turns_on_with_the_password_then_the_apps_code() {
    let mut app = linked(true);
    input(
        &mut app,
        SecurityInput::Edit(Some(AccountDialog::EnableTwoFactor)),
    );
    write(&mut app, AccountField::Password, "password");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::CheckTotpPassword(password)) if password == "password"
    ));
    app.apply(Event::AccountChanged(Ok(AccountAnswer::PasswordChecked {
        secret: "SECRET".into(),
    })));
    let editor = app.state.security.editor.as_ref().unwrap();
    assert_eq!(
        (editor.step, editor.secret.as_str()),
        (Some(AccountStep::Code), "SECRET")
    );
    write(&mut app, AccountField::Code, "12345");
    assert!(
        input(&mut app, SecurityInput::Submit).is_empty(),
        "six digits"
    );
    write(&mut app, AccountField::Code, "123 456");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::EnableTotp { secret, .. }) if secret == "SECRET"
    ));
    let codes = vec![BackupCode {
        code: "k3m9x2qa".into(),
        consumed: false,
    }];
    app.apply(Event::AccountChanged(Ok(AccountAnswer::BackupCodes(
        codes.clone(),
    ))));
    let editor = app.state.security.editor.as_ref().unwrap();
    assert_eq!(editor.step, Some(AccountStep::Codes));
    assert_eq!(editor.codes, codes);
    assert!(
        editor.password.is_empty() && editor.secret.is_empty(),
        "forgotten"
    );
    assert!(input(&mut app, SecurityInput::Submit).is_empty());
    assert!(app.state.security.editor.is_none(), "Done closes it");
}

#[test]
fn backup_codes_show_with_the_key_emailed() {
    let mut app = linked(true);
    input(
        &mut app,
        SecurityInput::Edit(Some(AccountDialog::BackupCodes)),
    );
    write(&mut app, AccountField::Password, "password");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::BackupCodesChallenge(_))
    ));
    app.apply(Event::AccountChanged(Ok(AccountAnswer::BackupNonce(
        "n".into(),
    ))));
    write(&mut app, AccountField::Code, "backupkey");
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::BackupCodes { key, nonce }) if key == "backupkey" && nonce == "n"
    ));
}

#[test]
fn turning_two_factor_off_asks_nothing_more_here() {
    let mut app = linked(true);
    input(
        &mut app,
        SecurityInput::Edit(Some(AccountDialog::DisableTwoFactor)),
    );
    assert!(matches!(
        change(input(&mut app, SecurityInput::Submit)),
        Some(AccountChange::DisableTotp)
    ));
    // Another doesn't open while Discord answers.
    input(&mut app, SecurityInput::Edit(Some(AccountDialog::Password)));
    assert_eq!(
        app.state.security.editor.as_ref().unwrap().editing,
        Some(AccountDialog::DisableTwoFactor)
    );
    app.apply(Event::AccountChanged(Ok(AccountAnswer::Done)));
    assert_eq!(
        app.state.security.notice.as_deref(),
        Some("Two-factor authentication is off.")
    );

    // Closed while Discord answers, as Settings close, it's forgotten, and
    // the answer changes nothing.
    input(
        &mut app,
        SecurityInput::Edit(Some(AccountDialog::BackupCodes)),
    );
    app.state.security.notice = None;
    write(&mut app, AccountField::Password, "password");
    assert!(change(input(&mut app, SecurityInput::Submit)).is_some());
    input(&mut app, SecurityInput::Edit(None));
    assert!(app.state.security.editor.is_none());
    app.apply(Event::AccountChanged(Ok(AccountAnswer::BackupNonce(
        "n".into(),
    ))));
    assert!(app.state.security.editor.is_none());
}

#[test]
fn the_apps_address_names_the_account() {
    assert_eq!(
        crate::application::totp_address("ABC", "alice b"),
        "otpauth://totp/Discord:alice%20b?secret=ABC&issuer=Discord"
    );
}
