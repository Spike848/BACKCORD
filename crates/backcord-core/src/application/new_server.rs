//! Making a Discord server, as Discord's "Create a server" does it, from
//! the "+" at the foot of the server rail: Create My Own, then who it's
//! for, then its name and icon; or one of Discord's templates, or one by
//! its `discord.new` link or code, previewed first. The new server opens
//! once its GUILD_CREATE lists it.

use super::*;
use crate::chat::{NewServerAnswer, NewServerCommand, TemplatePreview};

/// The longest a server's name may be, and the shortest, as Discord takes it.
pub const SERVER_NAME_LIMIT: usize = 100;
const SERVER_NAME_LEAST: usize = 2;

/// Discord's own templates, by their names and codes, as the list of
/// official templates docs.discord.food links to has them.
pub const SERVER_TEMPLATES: [(&str, &str); 6] = [
    ("Friends & Family", "hgM48av5Q69A"),
    ("Study Group", "FbwUwRp4j8Es"),
    ("Campus Clubs & Organizations", "Ctg7PUHcQmZu"),
    ("Local Communities", "64UDvRNCC52Y"),
    ("Creators & Hobbies", "6exdzMgjZgah"),
    ("Global Communities", "4sgbPdCjzAYU"),
];

/// The Create a server dialog, while it's open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewServerDialog {
    pub step: NewServerStep,
    /// The name and the picture file for its icon.
    pub name: String,
    pub icon: Option<String>,
    /// The template it's made from, once looked up.
    pub template: Option<TemplatePreview>,
    /// The code looked up, until Discord answers.
    pub looking_up: Option<String>,
    /// What's typed in the template code field.
    pub code: String,
    pub creating: bool,
    /// The new server, by its room id, until it lists and opens.
    pub made: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewServerStep {
    /// Create My Own, the templates, a code, or joining instead.
    #[default]
    Start,
    /// "Tell us more about your server": for friends or a community.
    About,
    /// A template's preview.
    Template,
    /// Its name and icon.
    Customize,
}

/// What's done in the Create a server dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewServerAction {
    Open,
    Close,
    Back,
    CreateOwn,
    /// For me and my friends (false), or a club or community (true).
    About(bool),
    /// A template by its code, or a `discord.new` link, as typed or chosen.
    Template(String),
    Code(String),
    Name(String),
    Icon(Option<String>),
    Create,
    /// Have an invite already? Join a Server instead.
    Join,
}

impl Application {
    pub(super) fn new_server_action(&mut self, action: NewServerAction) -> Vec<Effect> {
        if action == NewServerAction::Open {
            if self.state.discord_online || self.state.preview {
                self.state.new_server = Some(NewServerDialog::default());
            }
            return vec![];
        }
        let default_name = self.default_server_name();
        let preview = self.state.preview;
        let Some(dialog) = self.state.new_server.as_mut() else {
            return vec![];
        };
        if dialog.creating && action != NewServerAction::Close {
            return vec![];
        }
        match action {
            NewServerAction::Open => {}
            NewServerAction::Close => self.state.new_server = None,
            NewServerAction::Back => {
                dialog.error = None;
                dialog.step = match dialog.step {
                    NewServerStep::Customize if dialog.template.is_some() => {
                        NewServerStep::Template
                    }
                    NewServerStep::Customize => NewServerStep::About,
                    _ => NewServerStep::Start,
                };
                if dialog.step == NewServerStep::Start {
                    dialog.template = None;
                }
            }
            NewServerAction::CreateOwn => {
                dialog.template = None;
                dialog.step = NewServerStep::About;
                dialog.error = None;
            }
            NewServerAction::About(_) => {
                dialog.step = NewServerStep::Customize;
                if dialog.name.is_empty() {
                    dialog.name = default_name;
                }
            }
            NewServerAction::Template(typed) => {
                let Some(code) = template_code(&typed) else {
                    dialog.error = Some(
                        "Paste a template link, such as https://discord.new/hgM48av5Q69A.".into(),
                    );
                    return vec![];
                };
                dialog.error = None;
                dialog.looking_up = Some(code.clone());
                if preview {
                    return vec![];
                }
                return vec![Effect::Backend(Command::Discord(
                    DiscordCommand::NewServer(NewServerCommand::Template { code }),
                ))];
            }
            NewServerAction::Code(code) => dialog.code = code,
            NewServerAction::Name(name) => {
                dialog.name = name.chars().take(SERVER_NAME_LIMIT).collect();
                dialog.error = None;
            }
            NewServerAction::Icon(icon) => dialog.icon = icon,
            NewServerAction::Create => {
                if dialog.step == NewServerStep::Template {
                    dialog.step = NewServerStep::Customize;
                    if dialog.name.is_empty() {
                        dialog.name = default_name;
                    }
                    return vec![];
                }
                let name = dialog.name.trim().to_owned();
                if name.chars().count() < SERVER_NAME_LEAST {
                    dialog.error =
                        Some("A server's name must be between 2 and 100 characters.".into());
                    return vec![];
                }
                if preview {
                    return vec![];
                }
                dialog.creating = true;
                dialog.error = None;
                return vec![Effect::Backend(Command::Discord(
                    DiscordCommand::NewServer(NewServerCommand::Create {
                        name,
                        icon: dialog.icon.clone(),
                        template: dialog
                            .template
                            .as_ref()
                            .map(|template| template.code.clone()),
                    }),
                ))];
            }
            NewServerAction::Join => {
                self.state.new_server = None;
                return self.open_join();
            }
        }
        vec![]
    }

    /// Opens a clicked `discord.new` link's template in the dialog.
    pub(super) fn open_template_link(&mut self, code: String) -> Vec<Effect> {
        let mut effects = self.new_server_action(NewServerAction::Open);
        effects.extend(self.new_server_action(NewServerAction::Template(code)));
        effects
    }

    /// "{name}'s server", as Discord's client names a new one.
    fn default_server_name(&self) -> String {
        match self.state.account_name.as_deref() {
            Some(name) if !name.is_empty() => {
                let name: String = name.chars().take(SERVER_NAME_LIMIT - 9).collect();
                format!("{name}'s server")
            }
            _ => String::new(),
        }
    }

    pub(super) fn new_server_answered(&mut self, answer: NewServerAnswer) -> Vec<Effect> {
        let Some(dialog) = self.state.new_server.as_mut() else {
            return vec![];
        };
        match answer {
            NewServerAnswer::Template { code, result } => {
                if dialog.looking_up.as_ref() != Some(&code) {
                    return vec![];
                }
                dialog.looking_up = None;
                match result {
                    Ok(template) => {
                        dialog.template = Some(template);
                        dialog.step = NewServerStep::Template;
                        dialog.error = None;
                    }
                    Err(_) => {
                        dialog.error = Some("This template is invalid or was deleted.".into());
                    }
                }
            }
            NewServerAnswer::Created { result } => {
                if !dialog.creating {
                    return vec![];
                }
                match result {
                    Ok(guild) => {
                        dialog.made = Some(discord::server_id(&guild));
                        return self.open_made_server();
                    }
                    Err(error) => {
                        dialog.creating = false;
                        dialog.error = Some(error);
                    }
                }
            }
        }
        vec![]
    }

    /// Closes the dialog and opens the server it made, once it lists.
    pub(super) fn open_made_server(&mut self) -> Vec<Effect> {
        let Some(server) = self
            .state
            .new_server
            .as_ref()
            .and_then(|dialog| dialog.made.clone())
            .filter(|made| self.state.servers.iter().any(|server| &server.id == made))
        else {
            return vec![];
        };
        self.state.new_server = None;
        self.dispatch(Action::Navigate(Navigation::Server(server)))
    }
}

/// A template's code in what was typed or clicked.
fn template_code(typed: &str) -> Option<String> {
    #[cfg(feature = "discord")]
    {
        backcord_discord::guild_template::code_of(typed)
    }
    #[cfg(not(feature = "discord"))]
    {
        let _ = typed;
        None
    }
}
