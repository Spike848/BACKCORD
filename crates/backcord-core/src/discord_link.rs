//! Linking a Discord account by QR code, or by a token the person pastes,
//! beside the chat backend, so a build without Matrix can link before
//! anything else runs. The login goes into the OS credential store; see
//! `backcord_discord::remote_auth`.

use backcord_discord::{
    api::{Api, PasswordLogin, Refused},
    captcha,
    remote_auth::{self, Step, Stopped},
    token,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::runtime::{BackgroundRuntime, BackgroundTask};

/// How linking goes, for [`crate::application::Application::linking`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The code to show, as the address it carries.
    Code(String),
    /// Someone scanned it, by their username, and confirms on their phone.
    Scanned(String),
    /// Discord wants a captcha solved, on the page at this address.
    Captcha(String),
    /// Discord wants a two-factor code first: the ticket to send it with,
    /// and what was wrong with the last code, when one was tried.
    Mfa {
        ticket: String,
        error: Option<String>,
    },
    /// The account is linked and its login saved.
    Linked,
    /// The code expired before anyone confirmed it.
    Expired,
    Failed(String),
}

/// Starts linking an account on `runtime`: by QR code, or with `token`
/// when the person pasted one. Its progress arrives on the receiver until it
/// ends, and dropping the task stops it.
pub fn start(
    runtime: &BackgroundRuntime,
    token: Option<String>,
) -> (BackgroundTask<()>, UnboundedReceiver<Progress>) {
    let (sender, progress) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    let task = match token {
        Some(token) => runtime.spawn(with_token(token, report)),
        None => runtime.spawn(run(report)),
    };
    (task, progress)
}

/// Starts signing in with an email or phone number and a password. The
/// progress arrives as it does for [`start`].
pub fn start_password(
    runtime: &BackgroundRuntime,
    login: String,
    password: String,
) -> (BackgroundTask<()>, UnboundedReceiver<Progress>) {
    let (sender, progress) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    (
        runtime.spawn(with_password(login, password, report)),
        progress,
    )
}

/// Finishes a sign-in that wanted a two-factor code.
pub fn start_mfa(
    runtime: &BackgroundRuntime,
    ticket: String,
    code: String,
) -> (BackgroundTask<()>, UnboundedReceiver<Progress>) {
    let (sender, progress) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    (runtime.spawn(with_mfa(ticket, code, report)), progress)
}

/// Signs in with a password, showing Discord's captcha when it asks for
/// one, then links the account the login belongs to.
async fn with_password(login: String, password: String, progress: impl Fn(Progress)) {
    let api = match Api::anonymous() {
        Ok(api) => api,
        Err(error) => return progress(Progress::Failed(error.to_string())),
    };
    let (api_ref, login_ref, password_ref) = (&api, &login, &password);
    let sent = captcha::solving(
        |solved| async move {
            api_ref
                .password_login(
                    login_ref,
                    password_ref,
                    solved
                        .as_ref()
                        .map(|(captcha, solution)| (captcha, solution.as_str())),
                )
                .await
        },
        |page| progress(Progress::Captcha(page.to_owned())),
    )
    .await;
    match sent {
        Ok(PasswordLogin::Token(token)) => with_token(token, &progress).await,
        Ok(PasswordLogin::Mfa { ticket }) => progress(Progress::Mfa {
            ticket,
            error: None,
        }),
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "could not sign in to Discord");
            let words = error.to_string();
            progress(Progress::Failed(
                if words.contains("or password is invalid") {
                    "E-mail o password errate".to_owned()
                } else {
                    words
                },
            ));
        }
    }
}

/// Sends the two-factor code, then links the account. A code Discord
/// refuses leaves the ticket to try again with.
async fn with_mfa(ticket: String, code: String, progress: impl Fn(Progress)) {
    let sent = async {
        let api = Api::anonymous()?;
        api.mfa_login(&ticket, &code).await
    }
    .await;
    match sent {
        Ok(token) => with_token(token, &progress).await,
        Err(error) => progress(Progress::Mfa {
            ticket,
            error: Some(error.to_string()),
        }),
    }
}

/// Links the account a pasted token belongs to, once Discord accepts it.
async fn with_token(token: String, progress: impl Fn(Progress)) {
    let token = match token::clean(&token) {
        Ok(token) => token,
        Err(error) => return progress(Progress::Failed(error.to_string())),
    };
    let checked = async {
        let api = Api::new(token.clone())?;
        api.learn_build_number().await;
        api.me().await?;
        token::store(&token)
    };
    progress(match checked.await {
        Ok(()) => Progress::Linked,
        Err(error) => match error.downcast_ref::<Refused>() {
            Some(Refused(status)) if status.as_u16() == 401 => {
                Progress::Failed("Discord didn't accept that token.".into())
            }
            _ => {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "could not link Discord by token"
                );
                Progress::Failed(format!("BACKCORD couldn't link Discord: {error}"))
            }
        },
    });
}

/// Links an account, telling `progress` how it goes until it ends.
async fn run(progress: impl Fn(Progress)) {
    let linked = remote_auth::link(|step| {
        progress(match step {
            Step::Code(address) => Progress::Code(address),
            Step::Scanned { name, .. } => Progress::Scanned(name),
            Step::Captcha(page) => Progress::Captcha(page),
        });
    })
    .await;
    progress(match linked.and_then(|login| token::store(&login)) {
        Ok(()) => Progress::Linked,
        Err(error) => match error.downcast_ref::<Stopped>() {
            Some(Stopped::Expired) => Progress::Expired,
            Some(stopped) => Progress::Failed(stopped.to_string()),
            None => {
                tracing::warn!(error = format!("{error:#}"), "could not link Discord");
                Progress::Failed(format!("BACKCORD couldn't link Discord: {error}"))
            }
        },
    });
}
