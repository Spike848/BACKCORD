//! The dialogs a Discord server's, category's or channel's menu opens: the
//! invite link, Join a Server and a clicked invite's, and making, copying
//! or deleting a channel or category, which asks before leaving a server or
//! group too, and whether a channel dragged into a category takes its
//! permissions. What each shows and does is the core's (`state.invite`,
//! `state.join`, `state.channel_dialog`, `state.leave`, `state.move_sync`);
//! the dialogs keep only what's being chosen or typed in them
//! ([`Dialogs`]), from when the core opens one to when it closes it. Each
//! is drawn in `dialog::dialog`'s frame; Escape or a click outside closes
//! it.

use backcord_core::{
    application::{
        Action, AppState, ChannelAsk, INVITE_AGES, INVITE_USES, InviteDialog, InvitePerson,
    },
    chat::{ChannelKind, ChannelSettings},
};
use backcord_iced_kit::{i18n::tr, widgets::scroll_view};
use iced::{
    Alignment, Element, Length, Task,
    keyboard::key,
    widget::{Id, button, column, container, operation, row, text},
};

use crate::{
    app::{Message, NOBODY},
    controls::{self, Kind, action},
    ctx::{Ctx, Cx, Focus, Request},
    dialog::buttons,
    form,
    style::rounded,
};

/// The dialogs' widths: the invite's and the channel's, and Join a
/// Server's.
const WIDE: f32 = 460.0;
const NARROW: f32 = 440.0;
/// The invite link's box.
const LINK: f32 = 44.0;
/// How tall the invite dialog's list of people is, and how many of each
/// kind it shows at most; the search finds the rest.
const PEOPLE_HEIGHT: f32 = 300.0;
const PEOPLE_SHOWN: usize = 200;
/// How wide the invite link's choices are: the dialog's width inside its
/// padding, so the Italian words fit.
const CHOICES_WIDTH: f32 = WIDE - 40.0;
/// The fields' ids.
const INVITE_SEARCH: &str = "invite-search";
const JOIN_FIELD: &str = "join-invite";
const CHANNEL_FIELD: &str = "channel-name";

/// The choices for a new invite link, as Discord's dialog words them, in
/// the order of `INVITE_AGES` and `INVITE_USES`.
const AGES: [&str; 7] = [
    "30 min", "1 hr", "6 hr", "12 hr", "1 day", "7 days", "Never",
];
const USES: [&str; 7] = ["No limit", "1", "5", "10", "25", "50", "100"];

/// What's being chosen or typed in the open dialogs.
#[derive(Default)]
pub struct Dialogs {
    /// The invite dialog's next link (each an index into its list), and the
    /// link last copied, while it's open.
    invite: Option<InviteChoice>,
    /// Join a Server's field, while it's open.
    join: Option<String>,
    /// The channel dialog's name and kind, while it's open.
    channel: Option<ChannelDraft>,
    /// Whether the leave dialog showed at the last sync.
    leave_shown: bool,
}

struct InviteChoice {
    age: usize,
    uses: usize,
    copied: Option<String>,
    /// What's typed in its search, and whether it shows the choices for a
    /// new link rather than the people.
    search: String,
    editing: bool,
}

struct ChannelDraft {
    name: String,
    voice: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    InviteAge(usize),
    InviteUses(usize),
    InviteSearch(String),
    /// Edit invite link, or Cancel back from its choices.
    EditInvite,
    InvitePerson(String),
    Copy(String),
    Remake,
    CloseInvite,
    JoinLink(String),
    Find,
    /// Join Server's main button: find the invite, join, or open the form.
    Confirm,
    OpenCaptcha,
    CloseJoin,
    ChannelName(String),
    ChannelVoice(bool),
    /// A group's Change icon: the system's dialog, then the file chosen.
    GroupIcon,
    GroupIconPicked(Option<String>),
    SaveChannel,
    CloseChannel,
    Leave,
    ToggleSilent,
    CloseLeave,
    /// Cancella tutto's dialog: delete, or close it.
    DeleteAll,
    CloseDeleteAll,
    /// The moved channel takes its category's permissions (true), keeps
    /// its own (false), or stays where it was (none).
    MoveSync(Option<bool>),
}

fn message(event: Event) -> Message {
    Message::ServerDialog(event)
}

/// Whether one of these dialogs is open, which takes the keys.
pub fn is_open(state: &AppState) -> bool {
    state.invite.is_some()
        || state.join.is_some()
        || state.channel_dialog.is_some()
        || state.leave.is_some()
        || state.delete_all.is_some()
        || state.move_sync.is_some()
}

/// The pictures the dialogs show: the server an invite is to, and the
/// people the invite dialog lists.
pub fn pictures(state: &AppState) -> Vec<String> {
    let people = state.invite.iter().flat_map(|invite| {
        invite
            .people
            .iter()
            .filter_map(|person| person.avatar.clone())
            .take(2 * PEOPLE_SHOWN)
    });
    // A group's picture, in its Edit Group dialog.
    let group = state
        .channel_dialog
        .as_ref()
        .and_then(|dialog| match &dialog.ask {
            ChannelAsk::Group(room) => state.rooms.iter().find(|known| &known.id == room),
            _ => None,
        })
        .and_then(|room| room.avatar.clone());
    state
        .join
        .as_ref()
        .and_then(|join| join.preview.as_ref())
        .and_then(|preview| preview.icon.clone())
        .into_iter()
        .chain(people)
        .chain(group)
        .collect()
}

impl Dialogs {
    /// Takes in the core's dialogs: what's typed in one starts as it opens
    /// (a copy's name from its channel, Join's field empty) and goes when it
    /// closes. An opening dialog's field takes the keys, else nothing under
    /// it does.
    pub fn sync(&mut self, state: &AppState) -> Task<Message> {
        let local = &mut *self;
        let mut focus = None;
        match &state.invite {
            Some(invite) if local.invite.is_none() => {
                let index = |list: &[u32], value| {
                    list.iter().position(|known| *known == value).unwrap_or(0)
                };
                local.invite = Some(InviteChoice {
                    age: index(&INVITE_AGES, invite.max_age),
                    uses: index(&INVITE_USES, invite.max_uses),
                    copied: None,
                    search: String::new(),
                    editing: false,
                });
                focus = Some(NOBODY);
            }
            Some(_) => {}
            None => local.invite = None,
        }
        match &state.join {
            Some(join) if local.join.is_none() => {
                local.join = Some(String::new());
                focus = Some(if join.from_link { NOBODY } else { JOIN_FIELD });
            }
            Some(_) => {}
            None => local.join = None,
        }
        match &state.channel_dialog {
            Some(asked) if local.channel.is_none() => {
                local.channel = Some(ChannelDraft {
                    name: asked
                        .settings
                        .as_ref()
                        .map(|settings| settings.name.clone())
                        .unwrap_or_default(),
                    voice: asked.kind == ChannelKind::Voice,
                });
                let named = !matches!(asked.ask, ChannelAsk::Delete(_));
                focus = Some(if named { CHANNEL_FIELD } else { NOBODY });
            }
            Some(_) => {}
            None => local.channel = None,
        }
        let asking = state.leave.is_some() || state.delete_all.is_some();
        if asking && !local.leave_shown {
            focus = Some(NOBODY);
        }
        local.leave_shown = asking;
        match focus {
            Some(id) => operation::focus(Id::new(id)),
            None => Task::none(),
        }
    }

    pub fn update(&mut self, event: Event, cx: &mut Cx<'_>) -> Task<Message> {
        let local = &mut *self;
        let action = match event {
            Event::InviteAge(age) => {
                if let Some(invite) = &mut local.invite {
                    invite.age = age;
                }
                return Task::none();
            }
            Event::InviteUses(uses) => {
                if let Some(invite) = &mut local.invite {
                    invite.uses = uses;
                }
                return Task::none();
            }
            Event::Copy(link) => {
                if let Some(invite) = &mut local.invite {
                    invite.copied = Some(link.clone());
                }
                return iced::clipboard::write(link);
            }
            Event::InviteSearch(typed) => {
                if let Some(invite) = &mut local.invite {
                    invite.search = typed;
                }
                return Task::none();
            }
            Event::EditInvite => {
                if let Some(invite) = &mut local.invite {
                    invite.editing = !invite.editing;
                }
                return Task::none();
            }
            Event::InvitePerson(user) => Action::InviteFromDialog(user),
            Event::Remake => {
                let Some(invite) = &mut local.invite else {
                    return Task::none();
                };
                invite.editing = false;
                Action::MakeInvite {
                    max_age: INVITE_AGES[invite.age],
                    max_uses: INVITE_USES[invite.uses],
                }
            }
            Event::CloseInvite => Action::CloseInvite,
            Event::JoinLink(link) => {
                local.join = Some(link);
                return Task::none();
            }
            Event::Find => {
                let busy = cx
                    .state()
                    .join
                    .as_ref()
                    .is_some_and(|join| join.resolving || join.joining);
                match &local.join {
                    Some(link) if !busy => Action::FindInvite(link.clone()),
                    _ => return Task::none(),
                }
            }
            Event::Confirm => {
                let Some(join) = cx.state().join.as_ref() else {
                    return Task::none();
                };
                if join.form.is_some() {
                    Action::OpenJoinForm
                } else if join.preview.is_some() {
                    Action::JoinServer
                } else {
                    return self.update(Event::Find, cx);
                }
            }
            Event::OpenCaptcha => Action::OpenJoinCaptcha,
            Event::CloseJoin => Action::CloseJoin,
            Event::ChannelName(name) => {
                if let Some(draft) = &mut local.channel {
                    draft.name = name;
                }
                return Task::none();
            }
            Event::ChannelVoice(voice) => {
                if let Some(draft) = &mut local.channel {
                    draft.voice = voice;
                }
                return Task::none();
            }
            Event::GroupIcon => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Icona del gruppo")
                            .add_filter(
                                "PNG, JPG, GIF, WebP",
                                &["png", "jpg", "jpeg", "gif", "webp"],
                            )
                            .pick_file()
                            .await?;
                        Some(file.path().to_string_lossy().into_owned())
                    },
                    |picked| message(Event::GroupIconPicked(picked)),
                );
            }
            Event::GroupIconPicked(None) => return Task::none(),
            Event::GroupIconPicked(Some(path)) => Action::GroupIcon(Some(path)),
            Event::SaveChannel => {
                let saving = cx
                    .state()
                    .channel_dialog
                    .as_ref()
                    .is_none_or(|asked| asked.saving);
                let Some(draft) = &local.channel else {
                    return Task::none();
                };
                if saving {
                    return Task::none();
                }
                Action::SaveChannel {
                    kind: if draft.voice {
                        ChannelKind::Voice
                    } else {
                        ChannelKind::Text
                    },
                    settings: ChannelSettings {
                        name: draft.name.clone(),
                        ..Default::default()
                    },
                }
            }
            Event::CloseChannel => Action::CloseChannelDialog,
            Event::Leave => Action::Leave,
            Event::ToggleSilent => Action::ToggleSilentLeave,
            Event::CloseLeave => Action::CloseLeave,
            Event::DeleteAll => Action::DeleteAll,
            Event::CloseDeleteAll => Action::CloseDeleteAll,
            Event::MoveSync(sync) => Action::AnswerMoveSync(sync),
        };
        cx.dispatch(action);
        if !is_open(cx.state()) {
            cx.request(Request::Focus(Focus::Default));
        }
        Task::none()
    }

    /// The keys while one is open: Escape closes it, and Enter carries out
    /// a channel dialog or a leave without a field.
    pub fn key(&mut self, named: Option<key::Named>, cx: &mut Cx<'_>) -> Task<Message> {
        let state = cx.state();
        let (close, enter) = if state.move_sync.is_some() {
            (Event::MoveSync(None), Some(Event::MoveSync(Some(true))))
        } else if state.leave.is_some() {
            (Event::CloseLeave, Some(Event::Leave))
        } else if state.delete_all.is_some() {
            (Event::CloseDeleteAll, Some(Event::DeleteAll))
        } else if state.channel_dialog.is_some() {
            (Event::CloseChannel, Some(Event::SaveChannel))
        } else if state.join.is_some() {
            (Event::CloseJoin, None)
        } else if state.invite.is_some() {
            (Event::CloseInvite, None)
        } else {
            return Task::none();
        };
        let busy = state.leave.as_ref().is_some_and(|leave| leave.leaving);
        match (named, enter) {
            (Some(key::Named::Escape), _) => self.update(close, cx),
            (Some(key::Named::Enter), Some(enter)) if !busy => self.update(enter, cx),
            _ => Task::none(),
        }
    }
}

/// The open one of these dialogs, if any.
pub fn view<'a>(dialogs: &'a Dialogs, cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    View { s: dialogs, cx }.view()
}

/// What's typed in the dialogs and what they read, as their parts draw
/// them.
#[derive(Clone, Copy)]
struct View<'a> {
    s: &'a Dialogs,
    cx: Ctx<'a>,
}

impl<'a> View<'a> {
    fn view(self) -> Option<Element<'a, Message>> {
        let state = self.cx.state;
        if state.move_sync.is_some() {
            return self.move_sync_view();
        }
        if state.leave.is_some() {
            return self.leave_view();
        }
        if state.delete_all.is_some() {
            return self.delete_all_view();
        }
        if state.channel_dialog.is_some() {
            return self.channel_dialog_view();
        }
        if state.join.is_some() {
            return self.join_server_view();
        }
        self.invite_view()
    }

    /// The invite dialog, as Discord's: whom it lands in, the people to
    /// send the link to with Invite each, found by the search, then the link
    /// with Copy and how long it lasts; Edit invite link turns it to the
    /// choices for a new one.
    fn invite_view(self) -> Option<Element<'a, Message>> {
        let invite = self.cx.state.invite.as_ref()?;
        let choice = self.s.invite.as_ref()?;
        if choice.editing {
            return Some(self.invite_settings_view(invite, choice));
        }
        let theme = self.cx.look.theme;
        let place = if invite.voice {
            invite.channel.clone()
        } else {
            format!("#{}", invite.channel)
        };
        let mut body = vec![
            self.cx.look.dialog_head(
                format!("{} {}", tr("Invite friends to"), invite.server),
                message(Event::CloseInvite),
            ),
            form::note(theme, format!("{} {place}", tr("Recipients will land in"))),
            form::field(
                theme,
                INVITE_SEARCH,
                tr("Search for friends"),
                &choice.search,
            )
            .padding([8.0, 10.0])
            .on_input(|typed| message(Event::InviteSearch(typed)))
            .into(),
        ];
        let wanted = choice.search.trim().to_lowercase();
        let found = |person: &&InvitePerson| {
            wanted.is_empty()
                || person.name.to_lowercase().contains(&wanted)
                || person
                    .username
                    .as_deref()
                    .is_some_and(|username| username.to_lowercase().contains(&wanted))
        };
        let mut list = column![].spacing(2);
        let mut any = false;
        for (friends, heading) in [(true, "FRIENDS"), (false, "SERVER MEMBERS")] {
            let mut shown = invite
                .people
                .iter()
                .filter(|person| person.friend == friends)
                .filter(found)
                .take(PEOPLE_SHOWN)
                .peekable();
            if shown.peek().is_none() {
                continue;
            }
            any = true;
            list = list.push(form::heading(theme, tr(heading)));
            for person in shown {
                list = list.push(self.invite_row(invite, person));
            }
        }
        if !any {
            list = list.push(form::note(theme, tr("No one found")));
        }
        body.push(
            container(
                scroll_view(list.padding(iced::Padding {
                    right: 10.0,
                    ..iced::Padding::ZERO
                }))
                .width(Length::Fill)
                .style(backcord_iced_kit::style::scrollbar(theme)),
            )
            .height(PEOPLE_HEIGHT)
            .into(),
        );
        body.extend([
            text(tr("Or, send a server invite link to a friend"))
                .size(15)
                .font(backcord_iced_kit::fonts::BOLD)
                .color(theme.text)
                .width(Length::Fill)
                .into(),
            self.link_box(invite, choice),
        ]);
        if let Some(error) = &invite.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
        }
        let edit = button(text(tr("Edit invite link.")).size(13).color(theme.link))
            .padding(0)
            .on_press(message(Event::EditInvite))
            .style(|_, _| button::Style::default());
        let mut note = row![].spacing(4).align_y(Alignment::Center);
        if invite.link.is_some() {
            note = note.push(form::note(
                theme,
                format!("{}{}.", lasts(invite.max_age), works(invite.max_uses)),
            ));
        }
        body.push(
            iced::widget::Column::new()
                .push(note.push(edit).wrap())
                .width(Length::Fill)
                .into(),
        );
        Some(self.cx.look.framed(body, WIDE, message(Event::CloseInvite)))
    }

    /// Someone the invite dialog lists: their picture, name and username,
    /// and Invite, or Sent once it went.
    fn invite_row(
        self,
        invite: &'a InviteDialog,
        person: &'a InvitePerson,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let sent = invite.sent.contains(&person.id);
        let sending = invite.sending.contains(&person.id);
        let mut words = column![
            text(person.name.as_str())
                .size(15)
                .font(backcord_iced_kit::fonts::BOLD)
                .color(theme.text)
                .wrapping(text::Wrapping::None),
        ];
        if let Some(username) = &person.username {
            words = words.push(
                text(username.as_str())
                    .size(12)
                    .color(theme.muted)
                    .wrapping(text::Wrapping::None),
            );
        }
        let button_ = action(
            theme,
            if sent { tr("Sent") } else { tr("Invite") },
            Kind::Plain,
            (!sent && !sending).then(|| message(Event::InvitePerson(person.id.clone()))),
        );
        row![
            self.cx
                .avatar(person.avatar.as_deref(), &person.name, 32.0, 16.0, 1.0),
            container(words).width(Length::Fill).clip(true),
            button_,
        ]
        .spacing(10)
        .padding([4, 0])
        .align_y(Alignment::Center)
        .into()
    }

    /// The link with Copy, or Copied once it was.
    fn link_box(self, invite: &'a InviteDialog, choice: &'a InviteChoice) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let link = invite.link.as_deref().unwrap_or_default();
        let copied = !link.is_empty() && choice.copied.as_deref() == Some(link);
        let shown = if invite.making {
            text(tr("Making a link...")).color(theme.muted)
        } else {
            text(link).color(theme.text)
        };
        let sunken = theme.sunken;
        let copy = action(
            theme,
            if copied { tr("Copied") } else { tr("Copy") },
            if copied { Kind::Plain } else { Kind::Primary },
            (!link.is_empty()).then(|| message(Event::Copy(link.to_owned()))),
        );
        container(
            row![
                container(shown.size(15).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .clip(true),
                copy
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(iced::Padding {
            left: 12.0,
            right: 4.0,
            ..iced::Padding::ZERO
        })
        .height(LINK)
        .align_y(Alignment::Center)
        .style(move |_| rounded(sunken, 8))
        .into()
    }

    /// The choices for a new link: how long it lasts and how often it
    /// works, with Cancel back and Generate a new link.
    fn invite_settings_view(
        self,
        invite: &'a InviteDialog,
        choice: &'a InviteChoice,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let ages = AGES.map(tr);
        let uses = USES.map(tr);
        let mut body = vec![
            self.cx
                .look
                .dialog_head(tr("Invite link settings"), message(Event::CloseInvite)),
            form::heading(theme, tr("EXPIRE AFTER")),
            controls::choices_within(theme, &ages, choice.age, CHOICES_WIDTH, |index| {
                message(Event::InviteAge(index))
            }),
            form::heading(theme, tr("MAX NUMBER OF USES")),
            controls::choices_within(theme, &uses, choice.uses, CHOICES_WIDTH, |index| {
                message(Event::InviteUses(index))
            }),
        ];
        if let Some(error) = &invite.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
        }
        body.push(buttons([
            action(
                theme,
                tr("Cancel"),
                Kind::Ghost,
                Some(message(Event::EditInvite)),
            )
            .into(),
            action(
                theme,
                tr("Generate a new link"),
                Kind::Primary,
                (!invite.making).then(|| message(Event::Remake)),
            )
            .into(),
        ]));
        self.cx.look.framed(body, WIDE, message(Event::CloseInvite))
    }

    /// Join a Server, or the dialog a clicked invite link opens: the field
    /// (Join's alone), the server once found, how joining goes, and its
    /// buttons.
    fn join_server_view(self) -> Option<Element<'a, Message>> {
        let join = self.cx.state.join.as_ref()?;
        let typed = self.s.join.as_deref()?;
        let theme = self.cx.look.theme;
        let preview = join.preview.as_ref();
        let busy = join.resolving || join.joining;
        let title = if join.from_link {
            backcord_iced_kit::i18n::tr("You've been invited to join a server")
        } else {
            backcord_iced_kit::i18n::tr("Join a Server")
        };
        let mut body = vec![
            self.cx
                .look
                .dialog_head(title.to_owned(), message(Event::CloseJoin)),
        ];
        if !join.from_link {
            body.extend([
                form::note(
                    theme,
                    backcord_iced_kit::i18n::tr(
                        "Paste an invite link or code to join a server you were invited to.",
                    ),
                ),
                form::heading(theme, backcord_iced_kit::i18n::tr("INVITE LINK")),
                form::field(theme, JOIN_FIELD, "https://discord.gg/backdoor666", typed)
                    .padding([8.0, 10.0])
                    .on_input(|link| message(Event::JoinLink(link)))
                    .on_submit(message(Event::Find))
                    .into(),
            ]);
        }
        if let Some(preview) = preview {
            let detail = match (preview.members, preview.online) {
                (Some(members), Some(online)) if members >= 5 || online > 0 => {
                    backcord_iced_kit::i18n::t(format!("{online} online, {members} members"))
                }
                _ if preview.voice => preview.channel.clone(),
                _ => format!("#{}", preview.channel),
            };
            body.push(
                self.cx.look.found_card(
                    Some(
                        self.cx
                            .avatar(preview.icon.as_deref(), &preview.name, 48.0, 24.0, 1.0),
                    ),
                    preview.name.clone(),
                    [detail],
                ),
            );
        }
        let status = if join.form.is_some() {
            backcord_iced_kit::i18n::tr(
                "You joined. This server asks new members to fill in a form before they can talk, which BACKCORD can't show yet. Finish it in Discord's web app.",
            )
        } else if join.captcha.is_some() {
            backcord_iced_kit::i18n::tr(
                "Discord asks for a captcha first. Solve it in your browser.",
            )
        } else if join.joining {
            backcord_iced_kit::i18n::tr("Joining...")
        } else if join.resolving {
            backcord_iced_kit::i18n::tr("Looking up the invite...")
        } else if join.member {
            backcord_iced_kit::i18n::tr("You're already in this server.")
        } else {
            ""
        };
        if !status.is_empty() {
            body.push(form::note(theme, backcord_iced_kit::i18n::t(status)));
        }
        if let Some(error) = &join.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
        }
        let confirm = if join.form.is_some() {
            backcord_iced_kit::i18n::tr("Open in browser")
        } else if join.member {
            backcord_iced_kit::i18n::tr("Open Server")
        } else if preview.is_some() {
            backcord_iced_kit::i18n::tr("Join Server")
        } else if join.from_link {
            ""
        } else {
            backcord_iced_kit::i18n::tr("Find Invite")
        };
        let mut row_ = vec![
            action(
                theme,
                if join.form.is_some() || confirm.is_empty() {
                    backcord_iced_kit::i18n::tr("Close")
                } else {
                    backcord_iced_kit::i18n::tr("Cancel")
                },
                Kind::Ghost,
                Some(message(Event::CloseJoin)),
            )
            .into(),
        ];
        if join.captcha.is_some() {
            row_.push(
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Open the captcha again"),
                    Kind::Plain,
                    Some(message(Event::OpenCaptcha)),
                )
                .into(),
            );
        }
        if !confirm.is_empty() {
            row_.push(
                action(
                    theme,
                    confirm,
                    Kind::Primary,
                    (!busy).then(|| message(Event::Confirm)),
                )
                .into(),
            );
        }
        body.push(buttons(row_));
        Some(self.cx.look.framed(body, NARROW, message(Event::CloseJoin)))
    }

    /// Making, copying or deleting a channel or category: where it goes or
    /// what deleting does, a new channel's kind, the name, and the button.
    fn channel_dialog_view(self) -> Option<Element<'a, Message>> {
        let asked = self.cx.state.channel_dialog.as_ref()?;
        let draft = self.s.channel.as_ref()?;
        let theme = self.cx.look.theme;
        let category = asked.kind == ChannelKind::Category;
        let thing = if category { "category" } else { "channel" };
        let named = |name: &str| {
            if asked.kind == ChannelKind::Text {
                format!("#{name}")
            } else {
                name.to_owned()
            }
        };
        let (title, detail, confirm) = match &asked.ask {
            ChannelAsk::Create { .. } => (
                backcord_iced_kit::i18n::t(format!("Create {thing}")),
                if asked.name.is_empty() {
                    format!("In {}", asked.server_name)
                } else {
                    format!("In {}", asked.name)
                },
                backcord_iced_kit::i18n::t(format!("Create {thing}")),
            ),
            ChannelAsk::Duplicate(_) => (
                backcord_iced_kit::i18n::tr("Duplicate channel").to_owned(),
                backcord_iced_kit::i18n::t(format!(
                    "A copy of {}, with its settings and permissions, in the same category.",
                    named(&asked.name)
                )),
                backcord_iced_kit::i18n::tr("Create channel").to_owned(),
            ),
            ChannelAsk::Status(_) => (
                "Imposta stato del canale".to_owned(),
                format!(
                    "Cosa sta succedendo in {}? Compare sotto il nome del canale finché ci sono persone dentro.",
                    named(&asked.name)
                ),
                backcord_iced_kit::i18n::tr("Imposta stato").to_owned(),
            ),
            ChannelAsk::Group(_) => (
                backcord_iced_kit::i18n::tr("Edit Group").to_owned(),
                "Cambia il nome e l'icona del gruppo. Senza nome prende quelli dei membri."
                    .to_owned(),
                backcord_iced_kit::i18n::tr("Save").to_owned(),
            ),
            ChannelAsk::Delete(_) => (
                backcord_iced_kit::i18n::t(format!("Delete {thing}")),
                backcord_iced_kit::i18n::t(format!(
                    "Are you sure you want to delete {}? This cannot be undone.",
                    named(&asked.name)
                )),
                backcord_iced_kit::i18n::t(format!("Delete {thing}")),
            ),
        };
        let deleting = matches!(asked.ask, ChannelAsk::Delete(_));
        let ready = !asked.saving;
        let mut body = vec![
            self.cx
                .look
                .dialog_head(title, message(Event::CloseChannel)),
            form::note(theme, detail),
        ];
        if matches!(asked.ask, ChannelAsk::Create { .. }) && !category {
            body.extend([
                form::heading(theme, backcord_iced_kit::i18n::tr("CHANNEL TYPE")),
                controls::choices(
                    theme,
                    &[
                        backcord_iced_kit::i18n::tr("Text"),
                        backcord_iced_kit::i18n::tr("Voice"),
                    ],
                    usize::from(draft.voice),
                    |index| message(Event::ChannelVoice(index == 1)),
                ),
            ]);
        }
        if let ChannelAsk::Group(room) = &asked.ask {
            body.push(self.group_icon_row(room, asked.icon.as_deref()));
        }
        if !deleting {
            let (field, placeholder) = if matches!(asked.ask, ChannelAsk::Status(_)) {
                ("STATO DEL CANALE", "Imposta uno stato del canale")
            } else if matches!(asked.ask, ChannelAsk::Group(_)) {
                ("NOME DEL GRUPPO", asked.name.as_str())
            } else if category {
                (
                    backcord_iced_kit::i18n::tr("CATEGORY NAME"),
                    backcord_iced_kit::i18n::tr("New Category"),
                )
            } else {
                (backcord_iced_kit::i18n::tr("CHANNEL NAME"), "new-channel")
            };
            body.extend([
                form::heading(theme, field),
                form::field(theme, CHANNEL_FIELD, placeholder, &draft.name)
                    .padding([8.0, 10.0])
                    .on_input(|name| message(Event::ChannelName(name)))
                    .on_submit(message(Event::SaveChannel))
                    .into(),
            ]);
        }
        if let Some(error) = &asked.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
        }
        body.push(buttons([
            action(
                theme,
                backcord_iced_kit::i18n::tr("Cancel"),
                Kind::Ghost,
                Some(message(Event::CloseChannel)),
            )
            .into(),
            action(
                theme,
                confirm,
                if deleting {
                    Kind::Danger
                } else {
                    Kind::Primary
                },
                ready.then(|| message(Event::SaveChannel)),
            )
            .into(),
        ]));
        Some(
            self.cx
                .look
                .framed(body, WIDE, message(Event::CloseChannel)),
        )
    }

    /// A group's icon as it is, or the file chosen for it, and the button
    /// that chooses one.
    fn group_icon_row(self, room: &str, chosen: Option<&'a str>) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let group = self.cx.state.rooms.iter().find(|known| known.id == room);
        let (avatar, name) = group.map_or((None, ""), |group| {
            (group.avatar.as_deref(), group.name.as_str())
        });
        let face = self.cx.avatar(avatar, name, 64.0, 32.0, 1.0);
        let said = match chosen {
            Some(path) => std::path::Path::new(path)
                .file_name()
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_owned()),
            None => "Nessuna nuova icona scelta".to_owned(),
        };
        row![
            face,
            column![
                action(
                    theme,
                    "Cambia icona",
                    Kind::Primary,
                    Some(message(Event::GroupIcon)),
                ),
                text(said).size(12).color(theme.muted),
            ]
            .spacing(6)
            .width(Length::Fill),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    }

    /// Asks before leaving a server or group DM, in red; a group's offers
    /// to leave without telling its other members.
    fn leave_view(self) -> Option<Element<'a, Message>> {
        let leave = self.cx.state.leave.as_ref()?;
        let theme = self.cx.look.theme;
        let what = if leave.group { "group" } else { "server" };
        let ready = !leave.leaving;
        let mut body = vec![
            self.cx.look.dialog_head(
                backcord_iced_kit::i18n::t(format!("Leave '{}'", leave.name)),
                message(Event::CloseLeave),
            ),
            form::note(
                theme,
                backcord_iced_kit::i18n::t(format!(
                    "Are you sure you want to leave {}? You won't be able to rejoin this {what} unless you are re-invited.",
                    leave.name
                )),
            ),
        ];
        if leave.group {
            body.push(
                button(
                    row![
                        container(self.cx.look.checkbox(leave.silent, false)).padding(
                            iced::Padding {
                                top: 1.0,
                                ..iced::Padding::ZERO
                            }
                        ),
                        text(backcord_iced_kit::i18n::tr(
                            "Leave without notifying other members"
                        ))
                        .size(14)
                        .color(theme.secondary)
                        .width(Length::Fill)
                    ]
                    .spacing(10),
                )
                .padding(0)
                .width(Length::Fill)
                .on_press_maybe(ready.then(|| message(Event::ToggleSilent)))
                .style(|_, _| button::Style::default())
                .into(),
            );
        }
        if let Some(error) = &leave.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
        }
        body.push(buttons([
            action(
                theme,
                backcord_iced_kit::i18n::tr("Cancel"),
                Kind::Ghost,
                Some(message(Event::CloseLeave)),
            )
            .into(),
            action(
                theme,
                backcord_iced_kit::i18n::t(format!("Leave {what}")),
                Kind::Danger,
                ready.then(|| message(Event::Leave)),
            )
            .into(),
        ]));
        Some(self.cx.look.framed(body, WIDE, message(Event::CloseLeave)))
    }
}

impl<'a> View<'a> {
    /// Asks before Cancella tutto in a DM or server, in red.
    fn delete_all_view(self) -> Option<Element<'a, Message>> {
        let asked = self.cx.state.delete_all.as_ref()?;
        let theme = self.cx.look.theme;
        let tr = backcord_iced_kit::i18n::tr;
        let note = if asked.server {
            tr(
                "Every message you sent in {name}, in all of its channels, will be deleted, one at a time. This can't be undone.",
            )
        } else {
            tr(
                "Every message you sent in your DM with {name} will be deleted, one at a time. This can't be undone.",
            )
        };
        let body = vec![
            self.cx.look.dialog_head(
                tr("Delete all in '{name}'").replace("{name}", &asked.name),
                message(Event::CloseDeleteAll),
            ),
            form::note(theme, note.replace("{name}", &asked.name)),
            buttons([
                action(
                    theme,
                    tr("Cancel"),
                    Kind::Ghost,
                    Some(message(Event::CloseDeleteAll)),
                )
                .into(),
                action(
                    theme,
                    tr("Delete all"),
                    Kind::Danger,
                    Some(message(Event::DeleteAll)),
                )
                .into(),
            ]),
        ];
        Some(
            self.cx
                .look
                .framed(body, WIDE, message(Event::CloseDeleteAll)),
        )
    }

    /// Asks whether a channel dragged into a category takes its
    /// permissions, as Discord's client does (the core's `MoveSync`):
    /// Sync Permissions, or Keep Current Permissions; closing it leaves
    /// the channel where it was.
    fn move_sync_view(self) -> Option<Element<'a, Message>> {
        let asked = self.cx.state.move_sync.as_ref()?;
        let theme = self.cx.look.theme;
        let body = vec![
            self.cx.look.dialog_head(
                backcord_iced_kit::i18n::tr("Sync permissions?").to_owned(),
                message(Event::MoveSync(None)),
            ),
            form::note(
                theme,
                backcord_iced_kit::i18n::t(format!(
                    "Would you like to sync {} permissions with {}?",
                    asked.channel, asked.category
                )),
            ),
            buttons([
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Keep Current Permissions"),
                    Kind::Ghost,
                    Some(message(Event::MoveSync(Some(false)))),
                )
                .into(),
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Sync Permissions"),
                    Kind::Primary,
                    Some(message(Event::MoveSync(Some(true)))),
                )
                .into(),
            ]),
        ];
        Some(
            self.cx
                .look
                .framed(body, WIDE, message(Event::MoveSync(None))),
        )
    }
}

/// How long an invite link lasts, as the dialog says it.
fn lasts(age: u32) -> &'static str {
    tr(match age {
        0 => "Your invite link never expires",
        1800 => "Your invite link expires in 30 minutes",
        3600 => "Your invite link expires in 1 hour",
        21_600 => "Your invite link expires in 6 hours",
        43_200 => "Your invite link expires in 12 hours",
        86_400 => "Your invite link expires in 1 day",
        _ => "Your invite link expires in 7 days",
    })
}

/// How often it works, after how long it lasts.
fn works(uses: u32) -> &'static str {
    tr(match uses {
        0 => "",
        1 => " and works once",
        5 => " and works 5 times",
        10 => " and works 10 times",
        25 => " and works 25 times",
        50 => " and works 50 times",
        _ => " and works 100 times",
    })
}
