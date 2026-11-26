//! Someone's menu and the dialog it leads to: a right click on a member, a
//! friend or Active Now's card, or a profile's More, lists what the core
//! offers for them (`Application::person_choices`) in Discord's order, each
//! row with its icon. Unfriending, blocking, a note, a friend's
//! nickname, a call's question and a server's moderation ask first in the
//! person dialog. Roles, Invite to server, Mute and Apps open submenus
//! beside their rows: the server's roles to tick, the servers to invite
//! them to, how long to mute their DM, and the apps' user commands to run
//! on them.

use std::borrow::Cow;

use backcord_core::application::{
    Action, AppState, BAN_HISTORY, CommandTarget, MenuChoice, OwnershipInput, PermissionsOf,
    PersonAsk, TIMEOUT_TIMES, network_id, network_link,
};
use backcord_core::chat::RelationshipChange;
use iced::{
    Element, Length, Point, Task,
    widget::{Space, column, operation, row, text},
};

use crate::{
    app::{App, Message, NOBODY},
    controls::{Item, Kind, Light, action, field},
    ctx::{Ctx, Cx, Focus, Request},
    dialog::{CONFIRM_WIDTH, dialog},
    fonts::BOLD,
    menus::{Menu, MenuAct, MenuRow},
    pictures::Icon,
};

/// Where Discord's client says a choice in someone's menu came from: a
/// right click, or a profile's More.
pub const CONTEXT_MENU: &str = "ContextMenu";
pub const PROFILE_OVERFLOW: &str = "user profile overflow menu";

/// The dialog's field.
pub const PERSON_FIELD: &str = "person-field";
/// The field for a timeout's own length.
pub const LENGTH_FIELD: &str = "person-length";

/// What's done in the person dialog.
#[derive(Clone, Debug)]
pub enum Event {
    Text(String),
    Choose(usize),
    /// A timeout's own length, as it's written.
    Length(String),
    Tick,
    Answer,
    Close,
}

/// The dialog's own state: what's typed, and Don't ask again.
#[derive(Default)]
pub struct Dialog {
    pub text: String,
    pub ticked: bool,
    /// The core's text it was filled from, so it fills once it comes.
    pub filled: bool,
}

/// Whether someone's menus offer Follow: for another Discord user, while
/// it's on in Extra Features.
pub(crate) fn offers_follow(app: &App, user: &str) -> bool {
    let state = &app.client.app.state;
    app.layout.prefs.follow_voice.unwrap_or(true)
        && state.discord_user.is_some()
        && !user.is_empty()
        && !user.starts_with('@')
        && state.relation(user) != backcord_core::application::Relation::Me
}

/// Whether someone's menu offers Permissions: a member's in a Discord
/// server, while Permissions Viewer is on in Extra Features.
pub(crate) fn offers_permissions(app: &App, user: &str) -> bool {
    let core = &app.client.app;
    app.layout.prefs.permissions_viewer.unwrap_or(true)
        && core
            .member_place(user)
            .is_some_and(|place| core.offers_permissions(place))
}

/// Opens `user`'s menu at `at`, said to come from `location`.
pub fn open_person_menu(
    app: &mut App,
    user: &str,
    at: Point,
    location: &'static str,
) -> Task<Message> {
    let core = &app.client.app;
    let state = &core.state;
    let mut choices = core.person_choices(user);
    if app.layout.prefs.view_icons.unwrap_or(true)
        && state
            .person(user)
            .is_some_and(|person| person.avatar.is_some())
    {
        choices.push(MenuChoice::ViewAvatar);
    }
    if offers_follow(app, user) {
        choices.push(MenuChoice::FollowVoice);
    }
    if offers_permissions(app, user) {
        choices.push(MenuChoice::Permissions);
    }
    choices.sort_by_key(|choice| discord_rank(*choice));
    let name = state
        .person(user)
        .map_or_else(|| "them".to_owned(), |person| person.name);
    let in_call = state
        .rooms
        .iter()
        .any(|room| room.direct && room.person.as_deref() == Some(user) && !room.people.is_empty());
    let developer = app.layout.developer_mode;
    let ticked = voice_ticks(core, user);
    let mut group = None;
    let rows: Vec<MenuRow> = choices
        .into_iter()
        .filter(|choice| developer || *choice != MenuChoice::CopyId)
        .map(|choice| {
            let label: Cow<'static, str> = match choice {
                MenuChoice::Timeout => {
                    backcord_iced_kit::i18n::t(format!("Time out {name}")).into()
                }
                MenuChoice::Kick => backcord_iced_kit::i18n::t(format!("Kick {name}")).into(),
                MenuChoice::Ban => backcord_iced_kit::i18n::t(format!("Ban {name}")).into(),
                MenuChoice::Call if in_call => backcord_iced_kit::i18n::tr("Join call").into(),
                choice => crate::place_menus::label(choice).into(),
            };
            let starts = crate::place_menus::group_of(choice);
            let separated = group.replace(starts).is_some_and(|last| last != starts);
            let tick = ticked(choice);
            let mut row = MenuRow {
                icon: Some(crate::place_menus::icon(choice)),
                danger: danger(choice),
                separated,
                subtitle: (choice == MenuChoice::AddNote)
                    .then_some(backcord_iced_kit::i18n::tr("Only visible to you").into()),
                check: tick.map_or(crate::controls::Check::None, crate::controls::Check::Box),
                ..MenuRow::plain(MenuAct::Person(choice), label)
            };
            let selected = state.selected.as_deref();
            submenu(core, &mut row, choice, Some(user), selected, None);
            row.disabled = !(acts(choice)
                || tick.is_some()
                || matches!(
                    choice,
                    MenuChoice::Disconnect | MenuChoice::EditServerProfile
                )
                || !row.submenu.is_empty());
            row
        })
        .collect();
    if rows.is_empty() {
        return Task::none();
    }
    app.win.menu = Some(Menu {
        place: Some(user.to_owned()),
        location,
        ..Menu::new(at, rows).width(crate::menus::FIELD_WIDTH)
    });
    // Its Apps follow the permissions as they are now.
    app.client.dispatch(Action::AskAppCommands);
    Task::batch([operation::focus(NOBODY), app.sync()])
}

/// Whether each of the voice rows someone's menu offers is ticked, in the
/// voice channel their menu is in: muted or their soundboard muted for
/// this account, this account's own mute and deafen, and the server's
/// mute and deafen of them.
fn voice_ticks(
    core: &backcord_core::application::Application,
    user: &str,
) -> impl Fn(MenuChoice) -> Option<bool> + use<> {
    let voice = &core.state.voice;
    let listed = core
        .member_place(user)
        .and_then(|place| core.voice_person(place, user));
    let (server_muted, server_deafened) = listed.map_or((false, false), |person| {
        (person.server_muted, person.server_deafened)
    });
    let (muted, deafened) = (voice.muted, voice.deafened);
    let followed = core.state.following.as_deref() == Some(user);
    let user = user.to_owned();
    move |choice| match choice {
        MenuChoice::FollowVoice => Some(followed),
        MenuChoice::LocalMute => Some(backcord_core::volume::muted(&user)),
        MenuChoice::SoundboardMute => Some(backcord_core::volume::soundboard_muted(&user)),
        MenuChoice::BypassEffects => Some(backcord_core::volume::effects_bypassed(&user)),
        MenuChoice::SelfMute => Some(muted),
        MenuChoice::SelfDeafen => Some(deafened),
        MenuChoice::ServerMute => Some(server_muted),
        MenuChoice::ServerDeafen => Some(server_deafened),
        _ => None,
    }
}

/// A row of someone's menu was chosen; `ticked` is a checkbox row's new
/// state where the menu showed one.
pub fn choose_person(
    app: &mut App,
    user: String,
    location: &'static str,
    choice: MenuChoice,
    ticked: Option<bool>,
) -> Task<Message> {
    let state = app.client.state();
    // Their DM, for what acts on the conversation.
    let room = state
        .rooms
        .iter()
        .find(|room| room.direct && room.person.as_deref() == Some(user.as_str()))
        .map(|room| room.id.clone());
    let place = app
        .client
        .app
        .member_place(&user)
        .map(str::to_owned)
        .or_else(|| state.selected.clone());
    let ask = |ask| Action::AskPerson {
        user: user.clone(),
        ask,
        location,
    };
    let change = |change| Action::ChangeRelationship {
        user: user.clone(),
        change,
        location,
    };
    let ask_member = |ask| {
        place.clone().map(|place| Action::AskMember {
            place,
            user: user.clone(),
            ask,
        })
    };
    // The voice rows act in the voice channel the menu is in, as its
    // call's own menu does. A checkbox's new state is the one its row
    // shows: the core's own changes only once Discord answers, so a second
    // click before then would send the first again. Chosen from elsewhere,
    // it's the other of what the core holds.
    if let Some(room) = place.clone().filter(|_| voice_row(choice)) {
        let ticked =
            ticked.or_else(|| voice_ticks(&app.client.app, &user)(choice).map(|ticked| !ticked));
        return crate::call::popups::person::choose_voice(app, room, user, choice, ticked);
    }
    let action = match choice {
        MenuChoice::Profile => Some(Action::ShowProfile {
            user: user.clone(),
            large: true,
        }),
        MenuChoice::Message => Some(Action::MessagePerson(user.clone())),
        MenuChoice::FollowVoice => {
            let follow = ticked.unwrap_or(state.following.as_deref() != Some(user.as_str()));
            Some(Action::FollowVoice(follow.then(|| user.clone())))
        }
        MenuChoice::Call => Some(ask(PersonAsk::Call)),
        MenuChoice::AddFriend | MenuChoice::AcceptFriend => Some(Action::Befriend(user.clone())),
        MenuChoice::CancelFriendRequest => Some(Action::RemoveFriendRequest(user.clone())),
        MenuChoice::AddNote => Some(ask(PersonAsk::Note)),
        MenuChoice::AddNickname | MenuChoice::EditNickname => Some(ask(PersonAsk::Nickname)),
        MenuChoice::Unfriend => Some(ask(PersonAsk::Unfriend)),
        MenuChoice::Block => Some(ask(PersonAsk::Block)),
        MenuChoice::Ignore => Some(change(RelationshipChange::Ignore)),
        MenuChoice::Unignore => Some(change(RelationshipChange::Unignore)),
        MenuChoice::Unblock => Some(change(RelationshipChange::Unblock)),
        MenuChoice::Pin | MenuChoice::Unpin => room.map(|room| Action::Pin {
            room,
            pinned: choice == MenuChoice::Pin,
        }),
        MenuChoice::CloseDm => room.map(Action::CloseDm),
        MenuChoice::Unmute => room.map(|room| Action::MuteDm {
            room,
            muting: backcord_core::chat::Muting::Off,
        }),
        MenuChoice::EditServerProfile => place.map(Action::EditServerProfile),
        MenuChoice::Permissions => place.map(|place| {
            Action::ShowPermissions(PermissionsOf::Member {
                place,
                user: user.clone(),
            })
        }),
        MenuChoice::ServerNickname => ask_member(PersonAsk::ServerNickname),
        MenuChoice::Timeout => ask_member(PersonAsk::Timeout),
        MenuChoice::Kick => ask_member(PersonAsk::Kick),
        MenuChoice::Ban => ask_member(PersonAsk::Ban),
        // Offered only on Members, whose settings show the dialog.
        MenuChoice::TransferOwnership => {
            Some(Action::Ownership(OwnershipInput::Transfer(user.clone())))
        }
        // In the open group DM, whose member list the menu is in.
        MenuChoice::RemoveFromGroup | MenuChoice::MakeGroupOwner => {
            state.selected.clone().map(|room| Action::AskGroupMember {
                room,
                user: user.clone(),
                remove: choice == MenuChoice::RemoveFromGroup,
            })
        }
        MenuChoice::RemoveTimeout => place.map(|place| Action::ChangeMember {
            place,
            user: user.clone(),
            change: backcord_core::chat::MemberChange::Timeout {
                until_ms: None,
                reason: String::new(),
            },
        }),
        MenuChoice::Mention => {
            app.client.dispatch(Action::MentionPerson(user));
            let draft = app.client.state().draft().to_owned();
            app.composer.draft.reset(&draft);
            app.composer.draft_known = draft;
            return Task::batch([app.sync(), operation::focus(crate::composer::COMPOSER)]);
        }
        MenuChoice::CopyLink => return iced::clipboard::write(network_link(&user)),
        MenuChoice::CopyId => return iced::clipboard::write(network_id(&user).to_owned()),
        MenuChoice::ViewAvatar => {
            let Some(url) = app
                .client
                .state()
                .person(&user)
                .and_then(|person| person.avatar)
            else {
                return Task::none();
            };
            return app.icon_event(crate::icon_viewer::Event::Open {
                url,
                title: backcord_iced_kit::i18n::tr("Avatar").to_owned(),
            });
        }
        _ => None,
    };
    let Some(action) = action else {
        return Task::none();
    };
    app.client.dispatch(action);
    keep_pins(app);
    let focus = app.person.focus(app.client.state());
    Task::batch([app.sync(), focus])
}

/// DMs pinned to the top, and the DM list's folders, are kept in
/// `layout.json`.
pub fn keep_pins(app: &mut App) {
    let state = app.client.state();
    if state.pinned != app.layout.pinned {
        app.layout.pinned = state.pinned.clone();
        crate::layout::store(app.client.root(), "pinned", &state.pinned);
    }
    if state.dm_folders != app.layout.dm_folders {
        app.layout.dm_folders = state.dm_folders.clone();
        crate::layout::store(app.client.root(), "dm_folders", &state.dm_folders);
    }
}

impl Dialog {
    /// The dialog opens: its field takes the keys when it has one, else
    /// nothing does, so Enter and Escape reach the dialog.
    pub fn focus(&mut self, state: &AppState) -> Task<Message> {
        let Some(dialog) = &state.person_dialog else {
            return Task::none();
        };
        self.filled = dialog.text.is_some();
        self.text = dialog.text.clone().unwrap_or_default();
        self.ticked = false;
        if prompt(state, dialog).field.is_empty() {
            operation::focus(NOBODY)
        } else {
            operation::focus(PERSON_FIELD)
        }
    }

    /// What's done in the person dialog.
    pub fn update(&mut self, event: Event, cx: &mut Cx<'_>) -> Task<Message> {
        match event {
            Event::Text(text) => self.text = text,
            Event::Choose(index) => cx.dispatch(Action::ChoosePersonOption(index)),
            Event::Length(length) => cx.dispatch(Action::PersonLength(length)),
            Event::Tick => self.ticked = !self.ticked,
            Event::Answer => {
                let Some(dialog) = &cx.state().person_dialog else {
                    return Task::none();
                };
                if dialog.text.is_none() || dialog.saving {
                    return Task::none();
                }
                if dialog.ask == PersonAsk::Call && self.ticked {
                    cx.dispatch(Action::SkipCallPrompt(true));
                    cx.keep("skip_call_prompt", true);
                }
                let text = std::mem::take(&mut self.text);
                cx.dispatch(Action::AnswerPerson(text));
            }
            Event::Close => cx.dispatch(Action::ClosePersonDialog),
        }
        if cx.state().person_dialog.is_none() {
            cx.request(Request::Focus(Focus::Default));
        }
        Task::none()
    }

    /// The note or nickname the core was waiting for came: the field takes
    /// it, once.
    pub fn sync(&mut self, state: &AppState) {
        match &state.person_dialog {
            Some(dialog) if !self.filled => {
                if let Some(text) = &dialog.text {
                    self.text.clone_from(text);
                    self.filled = true;
                }
            }
            Some(_) => {}
            None => *self = Dialog::default(),
        }
    }

    /// The person dialog's keys: Enter answers one without a field, Escape
    /// closes it.
    pub fn key(
        &mut self,
        named: Option<iced::keyboard::key::Named>,
        cx: &mut Cx<'_>,
    ) -> Task<Message> {
        use iced::keyboard::key::Named;
        match named {
            Some(Named::Escape) => self.update(Event::Close, cx),
            Some(Named::Enter) => self.update(Event::Answer, cx),
            _ => Task::none(),
        }
    }
}

/// The person dialog over the dimmed window: its title and what it
/// explains, a timeout's or a ban's choices, the field, Don't ask again,
/// why it failed, Cancel and its button.
pub fn view<'a>(dialog: &'a Dialog, cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    View { s: dialog, cx }.view()
}

/// The dialog's state and what it reads, as its parts draw them.
#[derive(Clone, Copy)]
struct View<'a> {
    s: &'a Dialog,
    cx: Ctx<'a>,
}

impl<'a> View<'a> {
    /// The person dialog over the dimmed window: its title and what it
    /// explains, a timeout's or a ban's choices, the field, Don't ask
    /// again, why it failed, Cancel and its button.
    fn view(self) -> Option<Element<'a, Message>> {
        let dialog = self.cx.state.person_dialog.as_ref()?;
        let prompt = prompt(self.cx.state, dialog);
        let theme = self.cx.look.theme;
        let close = Message::Person(Event::Close);
        let mut body = column![self.cx.look.dialog_head(prompt.title, close.clone())]
            .spacing(12)
            .padding(20);
        if !prompt.text.is_empty() {
            body = body.push(text(prompt.text).size(15).color(theme.secondary));
        }
        if !prompt.options.is_empty() {
            let mut options =
                column![text(prompt.heading).size(12).font(BOLD).color(theme.muted)].spacing(2);
            for (index, option) in prompt.options.into_iter().enumerate() {
                let chosen = index == dialog.choice;
                options = options.push(self.choice_item(option, chosen, index));
            }
            body = body.push(options);
        }
        // A timeout's own length, which writing chooses.
        if dialog.ask == PersonAsk::Timeout {
            body = body.push(
                field(
                    theme,
                    LENGTH_FIELD,
                    backcord_iced_kit::i18n::tr("e.g. 60s, 90m, 2h 30m, 3d (up to 28 days)"),
                    &dialog.length,
                    false,
                    true,
                )
                .size(15)
                .padding(10)
                .on_input(|length| Message::Person(Event::Length(length)))
                .on_submit(Message::Person(Event::Answer)),
            );
        }
        let loading = dialog.text.is_none();
        if !prompt.field.is_empty() {
            let placeholder = if loading {
                backcord_iced_kit::i18n::tr("Loading...")
            } else {
                prompt.field
            };
            let mut input = field(
                theme,
                PERSON_FIELD,
                placeholder,
                &self.s.text,
                false,
                !loading,
            )
            .size(15)
            .padding(10);
            if !loading {
                input = input
                    .on_input(|text| Message::Person(Event::Text(text)))
                    .on_submit(Message::Person(Event::Answer));
            }
            body = body.push(input);
        }
        if dialog.ask == PersonAsk::Call {
            body = body.push(self.cx.look.menu_item(
                Item {
                    picture: None,
                    label: backcord_iced_kit::i18n::tr("Don't ask again").into(),
                    icon: Some(if self.s.ticked {
                        Icon::Check
                    } else {
                        Icon::Square
                    }),
                    danger: false,
                    slot: true,
                    disabled: false,
                    subtitle: None,
                    check: crate::controls::Check::None,
                },
                Light::Pointer,
                Message::Person(Event::Tick),
            ));
        }
        if let Some(error) = &dialog.error {
            body = body.push(crate::form::error(theme, backcord_iced_kit::i18n::t(error)));
        }
        let kind = if prompt.danger {
            Kind::Danger
        } else {
            Kind::Primary
        };
        let answer = (!loading && !dialog.saving).then_some(Message::Person(Event::Answer));
        body = body.push(
            row![
                Space::new().width(Length::Fill),
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Cancel"),
                    Kind::Ghost,
                    Some(close.clone())
                ),
                action(theme, prompt.confirm, kind, answer),
            ]
            .spacing(8),
        );
        Some(dialog_frame(theme, body.into(), close))
    }

    /// One of a timeout's or a ban's choices, ticked when chosen.
    fn choice_item(self, label: String, chosen: bool, index: usize) -> Element<'a, Message> {
        self.cx.look.menu_item(
            Item {
                picture: None,
                label: label.into(),
                icon: chosen.then_some(Icon::Check),
                danger: false,
                slot: true,
                disabled: false,
                subtitle: None,
                check: crate::controls::Check::None,
            },
            Light::Pointer,
            Message::Person(Event::Choose(index)),
        )
    }
}

fn dialog_frame<'a>(
    theme: &crate::theme::Theme,
    body: Element<'a, Message>,
    close: Message,
) -> Element<'a, Message> {
    dialog(theme, body, CONFIRM_WIDTH, close)
}

/// What the person dialog says.
struct Prompt {
    title: String,
    text: String,
    heading: &'static str,
    options: Vec<String>,
    field: &'static str,
    confirm: &'static str,
    danger: bool,
}

fn prompt(state: &AppState, dialog: &backcord_core::application::PersonDialog) -> Prompt {
    let name = &dialog.name;
    let me = state.relation(&dialog.user) == backcord_core::application::Relation::Me;
    let (title, text, field, confirm, danger) = match dialog.ask {
        PersonAsk::Note => (
            backcord_iced_kit::i18n::t(format!("Note on {name}")),
            backcord_iced_kit::i18n::tr("Only you see your note, in Discord too.").to_owned(),
            backcord_iced_kit::i18n::tr("Note"),
            backcord_iced_kit::i18n::tr("Save"),
            false,
        ),
        PersonAsk::Call => (
            backcord_iced_kit::i18n::tr("Ready to start a call?").to_owned(),
            backcord_iced_kit::i18n::tr("Enjoy this moment. Then jump in when you're ready.")
                .to_owned(),
            "",
            backcord_iced_kit::i18n::tr("Start Call"),
            false,
        ),
        PersonAsk::Nickname => (
            backcord_iced_kit::i18n::t(format!("Nickname for {name}")),
            backcord_iced_kit::i18n::tr(
                "Only you see the nickname you give a friend. Leave it empty to remove it.",
            )
            .to_owned(),
            backcord_iced_kit::i18n::tr("Nickname"),
            backcord_iced_kit::i18n::tr("Save"),
            false,
        ),
        PersonAsk::Unfriend => (
            backcord_iced_kit::i18n::t(format!("Remove {name} as a friend?")),
            backcord_iced_kit::i18n::t(format!(
                "You'll need to send {name} a friend request to be friends again."
            )),
            "",
            backcord_iced_kit::i18n::tr("Remove friend"),
            true,
        ),
        PersonAsk::Block => (
            backcord_iced_kit::i18n::t(format!("Block {name}?")),
            backcord_iced_kit::i18n::t(format!(
                "Blocking {name} also removes them as a friend. Their messages show collapsed, and they can't message you."
            )),
            "",
            backcord_iced_kit::i18n::tr("Block"),
            true,
        ),
        PersonAsk::ServerNickname if me => (
            backcord_iced_kit::i18n::tr("Change your nickname").to_owned(),
            backcord_iced_kit::i18n::tr(
                "Everyone in this server sees it. Leave it empty to go by your name.",
            )
            .to_owned(),
            backcord_iced_kit::i18n::tr("Nickname"),
            backcord_iced_kit::i18n::tr("Save"),
            false,
        ),
        PersonAsk::ServerNickname => (
            backcord_iced_kit::i18n::t(format!("Change {name}'s nickname")),
            backcord_iced_kit::i18n::tr(
                "Everyone in this server sees it. Leave it empty to reset it to their name.",
            )
            .to_owned(),
            backcord_iced_kit::i18n::tr("Nickname"),
            backcord_iced_kit::i18n::tr("Save"),
            false,
        ),
        PersonAsk::Timeout => (
            backcord_iced_kit::i18n::t(format!("Time out {name}")),
            backcord_iced_kit::i18n::t(format!(
                "Until it ends, {name} can read the server but can't send messages, react, or join voice. The reason goes in the server's audit log."
            )),
            backcord_iced_kit::i18n::tr("Reason (optional)"),
            backcord_iced_kit::i18n::tr("Time out"),
            true,
        ),
        PersonAsk::Kick => (
            backcord_iced_kit::i18n::t(format!("Kick {name}?")),
            backcord_iced_kit::i18n::t(format!(
                "{name} can join again with a new invite. The reason goes in the server's audit log."
            )),
            backcord_iced_kit::i18n::tr("Reason (optional)"),
            backcord_iced_kit::i18n::tr("Kick"),
            true,
        ),
        PersonAsk::Ban => (
            backcord_iced_kit::i18n::t(format!("Ban {name}?")),
            backcord_iced_kit::i18n::t(format!(
                "{name} can't join again until unbanned. The reason goes in the server's audit log."
            )),
            backcord_iced_kit::i18n::tr("Reason (optional)"),
            backcord_iced_kit::i18n::tr("Ban"),
            true,
        ),
    };
    let (heading, options) = match dialog.ask {
        PersonAsk::Timeout => (
            backcord_iced_kit::i18n::tr("DURATION"),
            TIMEOUT_TIMES
                .map(span)
                .into_iter()
                .chain([backcord_iced_kit::i18n::tr("Custom length").to_owned()])
                .collect(),
        ),
        PersonAsk::Ban => (
            backcord_iced_kit::i18n::tr("DELETE THEIR MESSAGES FROM"),
            // As Discord's: hours up to a day, then days.
            BAN_HISTORY
                .map(|seconds| match seconds / 3600 {
                    0 => backcord_iced_kit::i18n::tr("Don't delete any").to_owned(),
                    1 => backcord_iced_kit::i18n::tr("The last hour").to_owned(),
                    hours if hours <= 24 => {
                        backcord_iced_kit::i18n::t(format!("The last {hours} hours"))
                    }
                    hours => backcord_iced_kit::i18n::t(format!("The last {} days", hours / 24)),
                })
                .to_vec(),
        ),
        _ => ("", Vec::new()),
    };
    Prompt {
        title,
        text,
        heading,
        options,
        field,
        confirm,
        danger,
    }
}

/// A timeout's length in words: "60 seconds", "1 hour", "1 week".
fn span(seconds: u32) -> String {
    let (count, unit) = match seconds {
        seconds if seconds % (7 * 86400) == 0 => (seconds / (7 * 86400), "week"),
        seconds if seconds % 86400 == 0 => (seconds / 86400, "day"),
        seconds if seconds % 3600 == 0 => (seconds / 3600, "hour"),
        seconds if seconds % 60 == 0 && seconds > 60 => (seconds / 60, "minute"),
        seconds => (seconds, "second"),
    };
    let plural = if count == 1 { "" } else { "s" };
    backcord_iced_kit::i18n::t(format!("{count} {unit}{plural}"))
}

/// A row a voice channel's call offers for someone in it.
fn voice_row(choice: MenuChoice) -> bool {
    matches!(
        choice,
        MenuChoice::LocalMute
            | MenuChoice::SoundboardMute
            | MenuChoice::BypassEffects
            | MenuChoice::SelfMute
            | MenuChoice::SelfDeafen
            | MenuChoice::ServerMute
            | MenuChoice::ServerDeafen
            | MenuChoice::Disconnect
    )
}

/// A destructive choice, in the danger colours.
pub(crate) fn danger(choice: MenuChoice) -> bool {
    matches!(
        choice,
        MenuChoice::Unfriend
            | MenuChoice::Block
            | MenuChoice::Timeout
            | MenuChoice::Kick
            | MenuChoice::Ban
            | MenuChoice::ServerMute
            | MenuChoice::ServerDeafen
            | MenuChoice::Disconnect
            | MenuChoice::DeleteChannel
            | MenuChoice::LeaveServer
            | MenuChoice::LeaveGroup
            | MenuChoice::DeleteAll
            | MenuChoice::TransferOwnership
            | MenuChoice::RemoveFromGroup
    )
}

/// Whether this client acts on a choice of someone's menu yet; what opens
/// a submenu and a call's controls show greyed.
pub(crate) fn acts(choice: MenuChoice) -> bool {
    matches!(
        choice,
        MenuChoice::Profile
            | MenuChoice::Message
            | MenuChoice::Call
            | MenuChoice::AddFriend
            | MenuChoice::AcceptFriend
            | MenuChoice::CancelFriendRequest
            | MenuChoice::AddNote
            | MenuChoice::AddNickname
            | MenuChoice::EditNickname
            | MenuChoice::Unfriend
            | MenuChoice::Block
            | MenuChoice::Ignore
            | MenuChoice::Unignore
            | MenuChoice::Unblock
            | MenuChoice::Pin
            | MenuChoice::Unpin
            | MenuChoice::CloseDm
            | MenuChoice::Unmute
            | MenuChoice::ServerNickname
            | MenuChoice::Timeout
            | MenuChoice::Kick
            | MenuChoice::Ban
            | MenuChoice::RemoveTimeout
            | MenuChoice::TransferOwnership
            | MenuChoice::RemoveFromGroup
            | MenuChoice::MakeGroupOwner
            | MenuChoice::Mention
            | MenuChoice::Permissions
            | MenuChoice::CopyLink
            | MenuChoice::CopyId
    )
}

/// Gives a row of someone's menu, or a conversation's, the submenu its
/// choice opens, built as the menu opens: the roles `user` may be given in
/// the open channel's server, the servers to invite them to, how long to
/// mute their DM for, the user commands `apps` (a room) offers, the voice
/// channels to move them to from `voice`'s. A row that has none to offer
/// keeps none.
pub(crate) fn submenu(
    app: &backcord_core::application::Application,
    row: &mut MenuRow,
    choice: MenuChoice,
    user: Option<&str>,
    apps: Option<&str>,
    voice: Option<(&str, &str)>,
) {
    let state = &app.state;
    row.submenu = match choice {
        MenuChoice::Roles => match (state.selected.as_deref(), user) {
            (Some(place), Some(user)) => app
                .role_choices(place, user)
                .into_iter()
                // As Discord's list: a role this account can't give or take
                // shows only while the member holds it, and can't be changed.
                .filter(|choice| choice.grantable || choice.held)
                .map(|choice| {
                    // A role's emoji leads its name, as its picture does.
                    let label = match &choice.role.emoji {
                        Some(emoji) if choice.role.icon.is_none() => {
                            format!("{emoji} {}", choice.role.name)
                        }
                        _ => choice.role.name,
                    };
                    MenuRow {
                        check: crate::controls::Check::Box(choice.held),
                        disabled: !choice.grantable,
                        picture: choice.role.icon,
                        ..MenuRow::plain(MenuAct::Role, label).key(choice.role.id)
                    }
                })
                .collect(),
            _ => Vec::new(),
        },
        MenuChoice::InviteToServer => user
            .map(|user| {
                app.invite_servers(user)
                    .into_iter()
                    .map(|server| {
                        MenuRow::plain(MenuAct::InviteTo, server.name.clone()).key(&server.id)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        MenuChoice::AddToGroup => user
            .map(|user| {
                app.group_choices(user)
                    .into_iter()
                    .map(|group| MenuRow {
                        subtitle: group
                            .in_call
                            .then(|| backcord_iced_kit::i18n::tr("In the call").into()),
                        ..MenuRow::plain(MenuAct::AddTo, group.name).key(group.room)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        MenuChoice::Mute => {
            row.click = Some(MenuAct::MuteDm(backcord_core::chat::Muting::Always));
            crate::place_menus::mute_times(MenuAct::MuteDm)
        }
        MenuChoice::Apps => apps
            .map(|room| {
                app.commands_for_target(room, CommandTarget::User)
                    .into_iter()
                    .map(|command| MenuRow {
                        subtitle: Some(command.app_name.into()),
                        ..MenuRow::plain(MenuAct::App, command.name).key(command.command)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        MenuChoice::MoveTo => voice
            .map(|(room, user)| {
                app.move_targets(room, user)
                    .into_iter()
                    .map(|(room, name)| MenuRow::plain(MenuAct::MoveTo, name).key(room))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
}

/// Where a choice goes in Discord's member menu: Profile, Mention, Message,
/// Start call and Add note; then Invite to server and how this account
/// stands with them; then the server's nickname and roles; then what the
/// call and moderation offer; Copy ID last. What a DM adds as a
/// conversation comes before them all.
pub(crate) fn discord_rank(choice: MenuChoice) -> u8 {
    match choice {
        MenuChoice::MarkRead | MenuChoice::JoinVoice | MenuChoice::OpenChat => 0,
        MenuChoice::Profile => 1,
        MenuChoice::Mention => 2,
        MenuChoice::Message => 3,
        MenuChoice::Call | MenuChoice::FollowVoice => 4,
        MenuChoice::AddNote => 5,
        MenuChoice::AddNickname | MenuChoice::EditNickname => 6,
        MenuChoice::EditServerProfile => 7,
        MenuChoice::RemoveFromGroup => 8,
        MenuChoice::MakeGroupOwner => 9,
        MenuChoice::Apps => 10,
        MenuChoice::InviteToServer | MenuChoice::AddToGroup => 11,
        MenuChoice::AddFriend
        | MenuChoice::AcceptFriend
        | MenuChoice::CancelFriendRequest
        | MenuChoice::Unfriend => 12,
        MenuChoice::Ignore | MenuChoice::Unignore => 13,
        MenuChoice::Block | MenuChoice::Unblock => 14,
        MenuChoice::ServerNickname => 20,
        MenuChoice::Roles => 21,
        MenuChoice::Permissions => 22,
        MenuChoice::CopyId => 100,
        MenuChoice::CopyLink => 99,
        MenuChoice::ViewAvatar => 98,
        _ => 50,
    }
}
