//! Create a server (`state.new_server`), as Discord's Add a Server is
//! laid out: Create My Own and Discord's templates, or a template's link
//! or code; who it's for; a template's preview; then its icon and name.
//! Everything it shows and holds is the core's.

use backcord_core::application::{NewServerAction, NewServerStep, SERVER_TEMPLATES};
use backcord_iced_kit::{
    i18n::{t, tr},
    widgets::scroll_view,
};
use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

use super::{Event, NARROW, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    dialog::buttons,
    fonts::BOLD,
    form,
};

/// The fields' ids.
pub(super) const NAME_FIELD: &str = "new-server-name";
const CODE_FIELD: &str = "new-server-template";
/// A choice's height.
const BUTTON: f32 = 40.0;
/// How tall a template's channels get before they scroll.
const CHANNELS_HEIGHT: f32 = 240.0;

fn act(action: NewServerAction) -> Message {
    message(Event::NewServer(action))
}

/// A choice filling the dialog's width.
fn wide<'a>(
    theme: &backcord_iced_kit::theme::Theme,
    label: impl Into<std::borrow::Cow<'a, str>>,
    on: Message,
) -> Element<'a, Message> {
    action(theme, label, Kind::Plain, Some(on))
        .wide(BUTTON)
        .into()
}

impl<'a> View<'a> {
    pub(super) fn new_server_view(self) -> Option<Element<'a, Message>> {
        let dialog = self.cx.state.new_server.as_ref()?;
        let theme = self.cx.look.theme;
        let close = act(NewServerAction::Close);
        let mut body = Vec::new();
        let mut foot: Vec<Element<'a, Message>> = Vec::new();
        let back = || {
            action(
                theme,
                tr("Back"),
                Kind::Ghost,
                Some(act(NewServerAction::Back)),
            )
            .into()
        };
        match dialog.step {
            NewServerStep::Start => {
                body.extend([
                    self.cx.look.dialog_head(tr("Create Your Server"), close.clone()),
                    form::note(
                        theme,
                        tr("Your server is where you and your friends hang out. Make yours and start talking."),
                    ),
                    wide(theme, tr("Create My Own"), act(NewServerAction::CreateOwn)),
                    form::heading(theme, tr("START FROM A TEMPLATE")),
                ]);
                for (name, code) in SERVER_TEMPLATES {
                    body.push(wide(
                        theme,
                        tr(name),
                        act(NewServerAction::Template(code.to_owned())),
                    ));
                }
                body.extend([
                    form::heading(theme, tr("TEMPLATE LINK")),
                    row![
                        form::field(
                            theme,
                            CODE_FIELD,
                            "https://discord.new/hgM48av5Q69A",
                            &dialog.code
                        )
                        .padding([8.0, 10.0])
                        .on_input(|typed| act(NewServerAction::Code(typed)))
                        .on_submit(act(NewServerAction::Template(dialog.code.clone()))),
                        action(
                            theme,
                            tr("Preview"),
                            Kind::Plain,
                            (dialog.looking_up.is_none() && !dialog.code.trim().is_empty())
                                .then(|| act(NewServerAction::Template(dialog.code.clone()))),
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                ]);
                if dialog.looking_up.is_some() {
                    body.push(form::note(theme, tr("Looking up the template...")));
                }
                body.push(
                    row![
                        text(tr("Have an invite already?"))
                            .size(15)
                            .font(BOLD)
                            .color(theme.text),
                        action(
                            theme,
                            tr("Join a Server"),
                            Kind::Plain,
                            Some(act(NewServerAction::Join))
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .into(),
                );
            }
            NewServerStep::About => {
                body.extend([
                    self.cx.look.dialog_head(tr("Tell Us More About Your Server"), close.clone()),
                    form::note(
                        theme,
                        tr("In order to help you with your setup, is your new server for just a few friends or a larger community?"),
                    ),
                    wide(theme, tr("For me and my friends"), act(NewServerAction::About(false))),
                    wide(theme, tr("For a club or community"), act(NewServerAction::About(true))),
                ]);
                foot.push(back());
            }
            NewServerStep::Template => {
                let template = dialog.template.as_ref()?;
                body.push(self.cx.look.dialog_head(t(&template.name), close.clone()));
                if let Some(description) = &template.description {
                    body.push(form::note(theme, description.as_str()));
                }
                body.push(form::note(
                    theme,
                    t(format!("Used {} times", template.usage_count)),
                ));
                let mut channels = column![].spacing(2);
                for (kind, name) in &template.channels {
                    channels = channels.push(match kind {
                        4 => form::heading(theme, name.to_uppercase()),
                        2 => text(format!("🔊 {name}"))
                            .size(14)
                            .color(theme.secondary)
                            .into(),
                        _ => text(format!("# {name}"))
                            .size(14)
                            .color(theme.secondary)
                            .into(),
                    });
                }
                body.push(
                    container(scroll_view(channels).width(Length::Fill))
                        .max_height(CHANNELS_HEIGHT)
                        .into(),
                );
                if !template.roles.is_empty() {
                    body.push(form::note(
                        theme,
                        t(format!("Roles: {}", template.roles.join(", "))),
                    ));
                }
                foot.push(back());
                foot.push(
                    action(
                        theme,
                        tr("Use This Template"),
                        Kind::Primary,
                        Some(act(NewServerAction::Create)),
                    )
                    .into(),
                );
            }
            NewServerStep::Customize => {
                body.extend([
                    self.cx.look.dialog_head(tr("Customize Your Server"), close.clone()),
                    form::note(
                        theme,
                        tr("Give your new server a personality with a name and an icon. You can always change it later."),
                    ),
                ]);
                let icon = dialog.icon.as_deref().map_or_else(
                    || tr("No icon").to_owned(),
                    |path| {
                        std::path::Path::new(path)
                            .file_name()
                            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
                    },
                );
                body.extend([
                    form::heading(theme, tr("ICON")),
                    row![
                        container(form::note::<Message>(theme, icon)).width(Length::Fill),
                        action(
                            theme,
                            tr("Upload"),
                            Kind::Plain,
                            Some(message(Event::NewServerIcon))
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                    form::heading(theme, tr("SERVER NAME")),
                    form::field(theme, NAME_FIELD, "", &dialog.name)
                        .padding([8.0, 10.0])
                        .on_input(|typed| act(NewServerAction::Name(typed)))
                        .on_submit(act(NewServerAction::Create))
                        .into(),
                ]);
                if dialog.creating {
                    body.push(form::note(theme, tr("Making the server...")));
                }
                foot.push(back());
                foot.push(
                    action(
                        theme,
                        tr("Create"),
                        Kind::Primary,
                        (!dialog.creating).then(|| act(NewServerAction::Create)),
                    )
                    .into(),
                );
            }
        }
        if let Some(error) = &dialog.error {
            body.push(form::note_in(t(error), theme.danger));
        }
        if !foot.is_empty() {
            body.push(buttons(foot));
        }
        Some(self.cx.look.framed(body, NARROW, close))
    }
}
