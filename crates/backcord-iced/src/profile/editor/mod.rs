//! The profile editor: this account's card, each part of it edited in place
//! as Discord's editors allow (the core's `ProfileEditor` says which), and
//! Discord's bar asking to save once something changed. The main profile
//! has what it wears down its left and its tabs on the right where there's
//! room; a server's profile is the card alone, with Nitro's offer over what
//! takes it.
//!
//! What the card shows is a profile view made of what's written, drawn
//! with the profile card's own parts; it's made again only when the editor
//! changes.

use std::{path::PathBuf, sync::Arc};

use backcord_core::{
    application::{Action, AppState, ProfileEditor, ProfileText, ProfileView},
    chat::{self, ProfilePicture},
};
use iced::{
    Alignment, Color, Element, Length, Padding, Task,
    widget::{Id, Space, button, column, container, opaque, operation, row, stack, text},
};

use crate::profile::{Event as ProfileEvent, Mode, Shown, look::Tab};
use crate::{
    app::Message,
    controls::{lights, placed},
    ctx::{Cx, Focus, Request},
    field::{self, Draft},
    fonts::{BOLD, MEDIUM, SEMIBOLD},
    pictures::Icon,
    slot::{Closes, Slot, Why},
    style::{faded, rounded},
    theme::rgb,
    widgets::{Hsv, hsv_rgb, rgb_hsv},
};

mod badges;
mod card;
mod colour;
mod menus;
mod styles;
mod wardrobe;

use colour::parse_colour;
use menus::item;

/// The text fields' ids.
const NAME: &str = "profile-edit-name";
const PRONOUNS: &str = "profile-edit-pronouns";
const BIO: &str = "profile-edit-bio";
const COLOUR: &str = "profile-edit-colour";
const THEME_COLOUR: &str = "profile-edit-theme";
const TAG_SEARCH: &str = "profile-edit-tag";
/// The styles column beside the main profile, and the avatar's size.
const SIDE: f32 = 241.0;
/// A tile's width, two to a row of the styles column.
const TILE: f32 = (SIDE - 32.0 - 8.0) / 2.0;
const AVATAR: f32 = 120.0;
/// The profiles' switch's height above the card, with its gap.
const SWITCH: f32 = 40.0;
/// Round the main profile's preview, which stands apart from the styles
/// and its tabs, with room for its frame, as Discord's editor has it.
const CARD_MARGIN: f32 = 24.0;
/// Where Discord's client sends someone for what only its shop sets.
const SHOP: &str = "https://discord.com/shop";
const CONNECTIONS: &str = "https://discord.com/settings/connections";
/// The banner colours offered without typing.
const SWATCHES: [u32; 9] = [
    0x5865f2, 0x3ba55d, 0xfaa61a, 0xed4245, 0xeb459e, 0x1abc9c, 0x9b59b6, 0x607d8b, 0x2c2f33,
];

/// A text being edited in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    Name,
    Pronouns,
    Bio,
}

/// What opens over the editor: a picture's menu, the server tags, the
/// card's More, a theme colour's code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pop {
    Avatar,
    Banner,
    Tag,
    More,
    Top,
    Bottom,
    /// The profiles to edit: the main one and each server's.
    Profiles,
}

/// What's done in the editor.
#[derive(Clone, Debug)]
pub enum Event {
    Edit(Option<Text>),
    Write(ProfileText, String),
    Bio(field::Event),
    /// The avatar or banner clicked: its menu, the file dialog, or the
    /// banner colour's box.
    Picture(ProfilePicture),
    Pick(ProfilePicture),
    Picked(ProfilePicture, PathBuf),
    Reset(ProfilePicture),
    Pop(Option<Pop>),
    /// A theme colour chosen on its pad.
    Hsv(Hsv),
    Tag(String),
    TagQuery(String),
    Tab(Tab),
    Side(bool),
    Undo,
    Save,
    Nitro,
    CopyId,
    Status,
    Note,
    Close,
    /// Another profile to edit: a server's by its id, or the main one.
    Profile(Option<String>),
    /// In the Display Name Style window.
    Style(crate::profile::name_style::Event),
    /// A style from the text formatting panel, for the bio.
    Format(crate::format::Style),
    /// An emoji's text picked for the bio, in place of its selection.
    Insert(String),
    /// The choice of a collectible opened for a kind, or closed.
    Wardrobe(Option<backcord_core::chat::CollectibleKind>),
    /// More of the shop's collectibles in their choice.
    ShopMore,
    /// The badges' window opened, or closed.
    Badges(bool),
    /// A badge pressed: picked, and with Nitro being dragged.
    BadgeGrab(u32, bool),
    /// The pointer, dragging a badge, came over another.
    BadgeOver(u32),
    /// Let go of the badge being dragged.
    BadgeDrop,
    /// A badge hidden, or shown again.
    BadgeHide(u32),
    /// The badges' confirmation opened, or closed.
    BadgeConfirm(bool),
    BadgeSave,
    /// A collectible chosen for the preview, or with none its kind taken off.
    Choose(backcord_core::chat::CollectibleKind, Option<String>),
    /// The choice's confirmation opened, or closed.
    Confirm(bool),
    /// A collectible owned worn, or with none its kind taken off.
    Wear(backcord_core::chat::CollectibleKind, Option<String>),
}

fn message(event: Event) -> Message {
    Message::Profile(ProfileEvent::Editor(event))
}

/// `inserted` in the bio being written, at its caret.
pub fn insert_in_bio(inserted: String) -> Message {
    message(Event::Insert(inserted))
}

/// `style` applied to the bio being written.
pub fn format_bio(style: crate::format::Style) -> Message {
    message(Event::Format(style))
}

/// The editor on screen: its card, and what's open in it.
pub struct Editor {
    pub(super) shown: Shown,
    editing: Option<Text>,
    colour_open: bool,
    pop: Option<Pop>,
    /// The open theme colour's, as its pad shows it.
    hsv: Hsv,
    tab: Option<Tab>,
    side: bool,
    tag_query: String,
    bio: Draft,
    /// The Display Name Style window, while it's open.
    pub(crate) style: Option<crate::profile::name_style::Window>,
    /// The choice of collectibles, while it's open, by the kind it offers.
    wardrobe: Option<backcord_core::chat::CollectibleKind>,
    /// How many of the shop's the choice shows.
    shop_shown: usize,
    /// The choice asks whether to wear what was chosen.
    confirming: bool,
    /// The badges' window, while it's open.
    badges: Option<badges::Draft>,
    /// The core's editor and lists changes its card was made at.
    made: (u64, u64),
}

impl Editor {
    fn new(shown: Shown) -> Self {
        Self {
            shown,
            editing: None,
            colour_open: false,
            pop: None,
            hsv: [0.0; 3],
            tab: None,
            side: true,
            tag_query: String::new(),
            bio: Draft::default(),
            style: None,
            wardrobe: None,
            shop_shown: wardrobe::SHOP_PAGE,
            confirming: false,
            badges: None,
            made: (0, 0),
        }
    }

    /// Ends what's being edited or open on the card.
    pub(super) fn stop_editing(&mut self) {
        self.editing = None;
        self.colour_open = false;
        self.pop = None;
    }
}

/// The card as it would show once saved: what's written over what Discord
/// has.
fn preview(state: &AppState, editor: &ProfileEditor) -> ProfileView {
    let fallback = editor.profile.as_ref().map(|profile| &profile.fallback);
    let text = |text: Option<&String>| text.cloned().unwrap_or_default();
    let written = |written: &str, fallback: Option<&String>| {
        let written = written.trim();
        if written.is_empty() {
            text(fallback)
        } else {
            written.to_owned()
        }
    };
    // A picture just chosen goes by a name of its own, which the card's
    // pictures take from the editor (`Shown::picked`).
    let picture = |kind: ProfilePicture, own: Option<&String>| match editor.picture(kind) {
        (_, Some(_)) => Some(picked_url(kind, editor.picture_edits)),
        (Some(kept), None) => Some(kept.to_owned()),
        (None, None) => own.cloned(),
    };
    // The collectible chosen for the preview, of a kind: none when none
    // was, else the one chosen, or none to take it off.
    use backcord_core::chat::{CollectibleKind, Wardrobe};
    let chosen = |kind: CollectibleKind| -> Option<Option<&chat::Collectible>> {
        let (chosen_kind, sku) = editor.chosen.as_ref()?;
        if *chosen_kind != kind {
            return None;
        }
        let Some(sku) = sku else { return Some(None) };
        let Wardrobe::Loaded { owned, shop } = &editor.wardrobe else {
            return None;
        };
        owned
            .iter()
            .chain(shop)
            .find(|item| &item.sku == sku)
            .map(Some)
    };
    let kept_accent = editor.profile.as_ref().and_then(|profile| profile.accent);
    let accent = if editor.may_write(ProfileText::Colour) {
        editor.accent().unwrap_or(kept_accent)
    } else {
        kept_accent
    };
    let tag = match &editor.chosen_tag {
        None => fallback.and_then(|own| own.tag.clone()),
        Some(None) => None,
        Some(Some(chosen)) => state
            .servers
            .iter()
            .find(|server| server_id(&server.id) == chosen)
            .and_then(|server| server.tag.clone()),
    };
    let profile = chat::Profile {
        // The account's own while Discord's answer is awaited, so the card
        // keeps its own tabs.
        id: fallback
            .map(|own| own.id.clone())
            .or_else(|| state.discord_user.clone())
            .unwrap_or_default(),
        name: written(&editor.name, fallback.map(|own| &own.name)),
        handle: text(fallback.map(|own| &own.handle)),
        avatar: picture(
            ProfilePicture::Avatar,
            fallback.and_then(|own| own.avatar.as_ref()),
        ),
        banner: picture(
            ProfilePicture::Banner,
            fallback.and_then(|own| own.banner.as_ref()),
        ),
        accent: accent.or_else(|| fallback.and_then(|own| own.accent)),
        pronouns: written(&editor.pronouns, fallback.map(|own| &own.pronouns)),
        // Its times as the profile shows them, as Discord's preview does.
        bio: backcord_core::clock::render_timestamps(&written(
            &editor.bio,
            fallback.map(|own| &own.bio),
        )),
        created: fallback.and_then(|own| own.created.clone()),
        joined: fallback.and_then(|own| own.joined.clone()),
        server_name: text(fallback.map(|own| &own.server_name)),
        server_icon: fallback.and_then(|own| own.server_icon.clone()),
        theme: editor.theme().ok().flatten(),
        style: editor.name_style().map(Arc::new),
        widgets: fallback.map(|own| own.widgets.clone()).unwrap_or_default(),
        board_asked: fallback.is_some_and(|own| own.board_asked),
        board_named: fallback.is_some_and(|own| own.board_named),
        wishlist_place: fallback.and_then(|own| own.wishlist_place.clone()),
        wishlist: editor.wishlist.clone(),
        tag,
        // What the editor doesn't change comes from the profile it read.
        decoration: match chosen(CollectibleKind::Decoration) {
            Some(item) => item.and_then(|item| item.picture.as_deref().map(Arc::from)),
            None => fallback.and_then(|own| own.decoration.clone()),
        },
        effect: match &editor.chosen {
            Some((CollectibleKind::Effect, sku)) => sku.clone(),
            _ => fallback.and_then(|own| own.effect.clone()),
        },
        badges: fallback.map(|own| own.badges.clone()).unwrap_or_default(),
        frame: match chosen(CollectibleKind::Frame) {
            Some(item) => item
                .map(|item| Arc::from(item.frame.as_slice()))
                .unwrap_or_default(),
            None => fallback.map(|own| own.frame.clone()).unwrap_or_default(),
        },
        connections: fallback
            .map(|own| own.connections.clone())
            .unwrap_or_default(),
        // The account's own, as the card at the window's foot shows it.
        presence: crate::account::account(state)
            .presence
            .filter(|_| editor.main()),
        activity: fallback.and_then(|own| own.activity.clone()),
        activities: fallback
            .map(|own| own.activities.clone())
            .unwrap_or_default(),
        ..Default::default()
    };
    ProfileView {
        profile,
        ..Default::default()
    }
}

/// A chosen picture's name on the card, new with each choice.
fn picked_url(kind: ProfilePicture, edits: u32) -> String {
    let kind = match kind {
        ProfilePicture::Avatar => "avatar",
        ProfilePicture::Banner => "banner",
    };
    format!("picked:{kind}:{edits}")
}

/// A server's Discord id, from BACKCORD's id for it.
fn server_id(id: &str) -> &str {
    id.rsplit(':').next().unwrap_or(id)
}

impl Editor {
    /// A new editor on the core's, its card made of what's written.
    fn open(state: &AppState, link: u32) -> Self {
        let core = state
            .profile_editor
            .as_ref()
            .expect("the editor opens only while the core's is open");
        let view = Arc::new(preview(state, core));
        Self::new(Shown::new(crate::profile::look::Look::new(
            state,
            &view,
            None,
            &[],
            link,
        )))
    }

    /// Takes in the core's editor: its card is made again only when what it
    /// would show changed.
    fn sync(&mut self, state: &AppState, core: &ProfileEditor, link: u32) {
        // The preview is made from the editor and the servers' tags alone.
        let made = (state.profile_editor_changes, state.list_changes);
        if self.made == made {
            return;
        }
        let view = Arc::new(preview(state, core));
        if *self.shown.look.view != *view {
            self.shown.look = crate::profile::look::Look::new(state, &view, None, &[], link);
        }
        let mut picked = std::collections::HashMap::new();
        for kind in [ProfilePicture::Avatar, ProfilePicture::Banner] {
            if let (_, Some(preview)) = core.picture(kind) {
                picked.insert(picked_url(kind, core.picture_edits), preview.clone());
            }
        }
        *self.shown.picked.borrow_mut() = picked;
        self.made = made;
        // The bio's field follows the core while it isn't being written.
        if self.editing != Some(Text::Bio) && self.bio.text() != core.bio {
            self.bio.reset(&core.bio);
        }
    }

    /// What's done in the editor.
    pub fn update(&mut self, event: Event, cx: &mut Cx<'_>) -> Task<Message> {
        let editor = self;
        match event {
            Event::Style(event) => return crate::profile::name_style::update(editor, event, cx),
            Event::Edit(field) => {
                editor.editing = field;
                editor.colour_open = false;
                editor.pop = None;
                let id = match field {
                    Some(Text::Name) => NAME,
                    Some(Text::Pronouns) => PRONOUNS,
                    Some(Text::Bio) => BIO,
                    None => return operation::focus(crate::app::NOBODY),
                };
                return operation::focus(Id::new(id));
            }
            Event::Write(field, text) => {
                // A theme colour's code typed moves its pad there.
                if matches!(field, ProfileText::ThemeTop | ProfileText::ThemeBottom)
                    && let Some(rgb) = parse_colour(&text)
                {
                    editor.hsv = rgb_hsv(rgb);
                }
                cx.dispatch(Action::WriteProfile { field, text });
            }
            Event::Hsv(hsv) => {
                let field = match editor.pop {
                    Some(Pop::Top) => ProfileText::ThemeTop,
                    Some(Pop::Bottom) => ProfileText::ThemeBottom,
                    _ => return Task::none(),
                };
                editor.hsv = hsv;
                cx.dispatch(Action::WriteProfile {
                    field,
                    text: format!("#{:06x}", hsv_rgb(hsv)),
                });
            }
            Event::Insert(inserted) => {
                let bio = &editor.bio;
                let text = bio.text();
                let at = bio.selected().unwrap_or_else(|| {
                    let caret = bio.caret().min(text.len());
                    caret..caret
                });
                let written = format!("{}{inserted}{}", &text[..at.start], &text[at.end..]);
                let caret = at.start + inserted.len();
                bio.replace(&written, caret);
                editor.editing = Some(Text::Bio);
                cx.dispatch(Action::WriteProfile {
                    field: ProfileText::Bio,
                    text: written,
                });
                return operation::focus(Id::new(BIO));
            }
            Event::Format(style) => {
                let bio = &editor.bio;
                let (written, selected) =
                    crate::format::apply(style, &bio.text(), bio.selected(), bio.caret());
                bio.replace_selecting(&written, selected);
                editor.editing = Some(Text::Bio);
                cx.dispatch(Action::WriteProfile {
                    field: ProfileText::Bio,
                    text: written,
                });
                return operation::focus(Id::new(BIO));
            }
            Event::Bio(event) => match event {
                field::Event::Edited(text, _) => {
                    cx.dispatch(Action::WriteProfile {
                        field: ProfileText::Bio,
                        text,
                    });
                }
                // Enter breaks the line; Escape comes as a key, which
                // `editor_escape` takes.
                _ => return Task::none(),
            },
            Event::Picture(kind) => {
                editor.editing = None;
                let state = cx.state();
                let Some(core) = state.profile_editor.as_ref() else {
                    return Task::none();
                };
                if kind == ProfilePicture::Banner && core.may_write(ProfileText::Colour) {
                    editor.colour_open = !editor.colour_open;
                    editor.pop = None;
                    return if editor.colour_open {
                        operation::focus(Id::new(COLOUR))
                    } else {
                        Task::none()
                    };
                }
                if core.resettable(kind) {
                    editor.pop = Some(match kind {
                        ProfilePicture::Avatar => Pop::Avatar,
                        ProfilePicture::Banner => Pop::Banner,
                    });
                    return Task::none();
                }
                return editor.update(Event::Pick(kind), cx);
            }
            Event::Pick(kind) => {
                editor.pop = None;
                let title = match kind {
                    ProfilePicture::Avatar => backcord_iced_kit::i18n::tr("Change Avatar"),
                    ProfilePicture::Banner => backcord_iced_kit::i18n::tr("Change Banner"),
                };
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .add_filter(
                                backcord_iced_kit::i18n::tr("Pictures"),
                                &["png", "jpg", "jpeg", "gif", "webp"],
                            )
                            .pick_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    move |path| match path {
                        Some(path) => message(Event::Picked(kind, path)),
                        None => Message::Nothing,
                    },
                );
            }
            Event::Picked(kind, path) => {
                cx.dispatch(Action::PickProfilePicture { kind, path });
            }
            Event::Reset(kind) => {
                editor.pop = None;
                cx.dispatch(Action::ResetProfilePicture(kind));
            }
            Event::Pop(pop) => {
                editor.pop = if editor.pop == pop { None } else { pop };
                editor.editing = None;
                editor.tag_query.clear();
                match editor.pop {
                    Some(Pop::Tag) => return operation::focus(Id::new(TAG_SEARCH)),
                    Some(pop @ (Pop::Top | Pop::Bottom)) => {
                        let core = cx.state().profile_editor.as_ref();
                        let code = core.map_or("", |core| match pop {
                            Pop::Top => core.theme_top.as_str(),
                            _ => core.theme_bottom.as_str(),
                        });
                        editor.hsv = rgb_hsv(parse_colour(code).unwrap_or(0x5865f2));
                        return operation::focus(Id::new(THEME_COLOUR));
                    }
                    _ => return Task::none(),
                }
            }
            Event::Tag(server) => {
                editor.pop = None;
                cx.dispatch(Action::ChooseServerTag(server));
            }
            Event::TagQuery(query) => {
                editor.tag_query = query;
                return Task::none();
            }
            Event::Tab(tab) => {
                editor.tab = Some(tab);
            }
            Event::Side(side) => editor.side = side,
            Event::Wardrobe(kind) => {
                editor.wardrobe = kind;
                editor.shop_shown = wardrobe::SHOP_PAGE;
                editor.confirming = false;
                editor.pop = None;
                // What was chosen and not worn goes with the choice.
                cx.dispatch(Action::ChooseCollectible(None));
                if kind.is_some() {
                    cx.dispatch(Action::OpenWardrobe);
                }
            }
            Event::ShopMore => editor.shop_shown += wardrobe::SHOP_PAGE,
            Event::Badges(open) => {
                editor.pop = None;
                editor.badges = open.then(badges::Draft::default);
                if open {
                    cx.dispatch(Action::OpenBadges);
                }
            }
            Event::BadgeGrab(id, drag) => {
                if let Some(draft) = editor.badges.as_mut() {
                    draft.selected = Some(id);
                    draft.dragging = drag.then_some(id);
                }
            }
            Event::BadgeDrop => {
                if let Some(draft) = editor.badges.as_mut() {
                    draft.dragging = None;
                }
            }
            Event::BadgeHide(id) | Event::BadgeOver(id) => {
                let listed = cx.state().profile_editor.as_ref().map(|core| &core.badges);
                if let (Some(draft), Some(backcord_core::chat::OwnBadges::Loaded(listed))) =
                    (editor.badges.as_mut(), listed)
                {
                    match event {
                        Event::BadgeOver(_) => draft.drag_over(listed, id),
                        _ => draft.hide(listed, id),
                    }
                }
            }
            Event::BadgeConfirm(open) => {
                if let Some(draft) = editor.badges.as_mut() {
                    draft.confirming = open;
                }
            }
            Event::BadgeSave => {
                let listed = cx.state().profile_editor.as_ref().map(|core| &core.badges);
                if let (Some(draft), Some(backcord_core::chat::OwnBadges::Loaded(listed))) =
                    (editor.badges.as_mut(), listed)
                {
                    draft.confirming = false;
                    let (order, hidden) = draft.current(listed);
                    cx.dispatch(Action::ArrangeBadges { order, hidden });
                }
            }
            Event::Choose(kind, sku) => {
                editor.confirming = false;
                cx.dispatch(Action::ChooseCollectible(Some((kind, sku))));
            }
            Event::Confirm(open) => editor.confirming = open,
            Event::Wear(kind, sku) => {
                editor.confirming = false;
                cx.dispatch(Action::Wear { kind, sku });
            }
            Event::Undo => {
                editor.editing = None;
                editor.colour_open = false;
                cx.dispatch(Action::UndoProfileEdits);
            }
            Event::Save => {
                editor.editing = None;
                cx.dispatch(Action::SaveProfile);
            }
            Event::Nitro => cx.dispatch(Action::UnlockNitro),
            Event::CopyId => {
                editor.pop = None;
                let id = editor.shown.look.profile().id.clone();
                return iced::clipboard::write(id);
            }
            Event::Status => cx.dispatch(Action::OpenStatusEditor),
            Event::Note => {
                let Some(user) = cx.state().discord_user.clone() else {
                    return Task::none();
                };
                cx.dispatch(Action::AskPerson {
                    user,
                    ask: backcord_core::application::PersonAsk::Note,
                    location: "ContextMenu",
                });
            }
            // The slot closes as the core's editor goes.
            Event::Close => cx.dispatch(Action::CloseProfileEditor),
            Event::Profile(server) => {
                editor.pop = None;
                match server {
                    Some(server) => cx.dispatch(Action::EditServerProfile(server)),
                    None => cx.dispatch(Action::EditProfile),
                }
                // The new profile's editor, from what it starts with.
                let link = rgb(cx.theme.link);
                if cx.state().profile_editor.is_some() {
                    *editor = Editor::open(cx.state(), link);
                }
            }
        }
        cx.touch();
        Task::none()
    }

    /// Escape in the editor: what's being edited or open first, then the
    /// editor.
    pub fn escape(&mut self, cx: &mut Cx<'_>) -> Task<Message> {
        if crate::profile::name_style::escape(self) {
            return Task::none();
        }
        if let Some(draft) = self.badges.as_mut() {
            if draft.confirming {
                draft.confirming = false;
            } else {
                self.badges = None;
            }
            return Task::none();
        }
        if self.confirming {
            self.confirming = false;
            return Task::none();
        }
        if self.wardrobe.is_some() {
            return self.update(Event::Wardrobe(None), cx);
        }
        let editor = self;
        if editor.editing.is_some() || editor.colour_open || editor.pop.is_some() {
            editor.editing = None;
            editor.colour_open = false;
            editor.pop = None;
            return operation::focus(crate::app::NOBODY);
        }
        editor.update(Event::Close, cx)
    }
}

impl Closes for Editor {
    /// The keys go back to where they were, unless the window went.
    fn close(&mut self, cx: &mut Cx<'_>, why: Why) {
        if why != Why::Window {
            cx.request(Request::Focus(Focus::Default));
        }
    }
}

/// The editor follows the core's: made as it opens, closed as it closes,
/// its card made again as what it shows changes. Settings close as it
/// opens, so closing it goes back to the conversation.
pub fn follow(slot: &mut Slot<Editor>, cx: &mut Cx<'_>) {
    let link = rgb(cx.theme.link);
    let open = cx.state().profile_editor.is_some().then_some(());
    let Some(editor) = slot.follow(
        open,
        cx,
        |_, _| true,
        |(), cx| {
            cx.request(Request::Close(crate::ctx::Screen::Settings));
            Editor::open(cx.state(), link)
        },
    ) else {
        return;
    };
    let state = cx.state();
    if let Some(core) = state.profile_editor.as_ref() {
        editor.sync(state, core, link);
    }
}

/// Whether the card has room for the tabs beside the middle.
fn editor_tabs_shown(card_width: f32, main: bool) -> bool {
    main && card_width >= 880.0
}

/// A section's heading on the card: the main profile's in its card's
/// words, a server's in capitals.
fn heading<'a>(
    main: bool,
    words: &'a str,
    inks: &crate::profile::look::Inks,
) -> Element<'a, Message> {
    if main {
        text(words).size(13).font(MEDIUM).color(inks.ink).into()
    } else {
        text(words.to_uppercase())
            .size(12)
            .font(BOLD)
            .color(inks.ink3)
            .into()
    }
}

/// Words that do something when clicked.
fn link_text<'a>(words: &'a str, ink: Color, pressed: Message) -> Element<'a, Message> {
    button(text(words).size(14).font(MEDIUM).color(ink))
        .padding(0)
        .on_press(pressed)
        .style(|_, _| button::Style::default())
        .into()
}

/// What says a text can be changed: an outline around it while pointed at,
/// as Discord's editor draws it (1 px of 17% white, 7 px round).
fn hover_edit<'a>(
    content: impl Into<Element<'a, Message>>,
    enabled: bool,
    pressed: Message,
) -> Element<'a, Message> {
    let outline = Color {
        a: 0.17,
        ..Color::WHITE
    };
    button(content)
        .padding([4, 7])
        .on_press_maybe(enabled.then_some(pressed))
        .style(move |_, status| button::Style {
            background: None,
            border: crate::style::edge(
                if enabled && lights(status) {
                    outline
                } else {
                    Color::TRANSPARENT
                },
                7.0,
            ),
            ..Default::default()
        })
        .into()
}

/// The pictures the editor's card shows.
pub(super) fn editor_pictures(ctx: crate::ctx::Ctx<'_>) -> Vec<String> {
    let Some(editor) = ctx.profiles.editor.get() else {
        return Vec::new();
    };
    let profile = editor.shown.look.profile();
    let cosmetics = ctx.state.cosmetics;
    let mut urls: Vec<String> = [
        profile.avatar.clone(),
        profile.banner.clone(),
        profile.banner.as_deref().and_then(chat::moving_banner),
    ]
    .into_iter()
    .flatten()
    .filter(|url| !url.starts_with("picked:"))
    .collect();
    urls.extend(
        cosmetics
            .decoration(profile.decoration.as_deref())
            .map(str::to_owned),
    );
    urls.extend(profile.badges.iter().map(|badge| badge.icon.clone()));
    urls.extend(profile.server_icon.clone());
    if let Some(kind) = editor.wardrobe {
        urls.extend(wardrobe::pictures(ctx, kind, editor.shop_shown));
    }
    if editor.badges.is_some() {
        urls.extend(badges::pictures(ctx));
    }
    urls.extend(
        cosmetics
            .tag(profile.tag.as_ref())
            .and_then(|tag| tag.badge.as_deref())
            .map(str::to_owned),
    );
    // The effect's still, on its tile.
    urls.extend(crate::profile::cosmetic::effect_pictures(
        ctx.state,
        &editor.shown,
        false,
    ));
    // The frame's layers, round the preview and on its tile.
    urls.extend(crate::profile::cosmetic::frame_pictures(
        ctx.state,
        &editor.shown,
    ));
    // The board's games, their covers and icons, and what's going on now,
    // on the preview's tabs.
    for widget in &profile.widgets {
        for game in &widget.games {
            urls.extend(game.cover.clone());
            urls.extend(game.icon.clone());
        }
    }
    for activity in &editor.shown.look.activities {
        urls.extend(activity.large.clone());
        urls.extend(activity.small.clone());
    }
    urls
}

/// The editor over the window dimmed by 60%; a click outside the card
/// closes it.
/// The editor's card's width: the main profile's with the styles
/// column or without, a server profile's.
fn editor_card_width(ctx: crate::ctx::Ctx<'_>, editor: &Editor, main: bool) -> f32 {
    let width = ctx.look.window.width;
    if main {
        if editor.side {
            (width - 8.0).min(1199.0)
        } else {
            (width - 96.0).min(958.0)
        }
    } else {
        (width - 48.0).min(600.0)
    }
}

/// The middle column's width: what the styles column (and its line)
/// leave of the card, halved with the tabs, which fill as it does.
fn editor_middle_width(ctx: crate::ctx::Ctx<'_>, editor: &Editor, main: bool) -> f32 {
    let card = editor_card_width(ctx, editor, main);
    let rest = card - if main && editor.side { SIDE + 1.0 } else { 0.0 };
    // Beside the tabs, its margins come out of theirs; alone, out of its own.
    if editor_tabs_shown(card, main) {
        rest / 2.0
    } else if main {
        rest - 2.0 * CARD_MARGIN
    } else {
        rest
    }
}

pub fn profile_editor_view<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<Element<'a, Message>> {
    let editor = ctx.profiles.editor.get()?;
    let core = ctx.state.profile_editor.as_ref()?;
    let theme = ctx.look.theme;
    let main = core.main();
    let height = ctx.look.window.height;
    let inks = crate::profile::inks(ctx, &editor.shown.look, Mode::Side);
    let card_width = editor_card_width(ctx, editor, main);
    // Room above it for the profiles' switch.
    let card_height = (height - 48.0 - SWITCH).min(if main { 798.0 } else { 620.0 });
    let middle = crate::profile::editor::card::editor_middle(ctx, editor, core, &inks);
    let mut parts = row![];
    if main && editor.side {
        let line = theme.line;
        parts = parts
            .push(crate::profile::editor::styles::customize(ctx, editor, core))
            .push(
                container(Space::new())
                    .width(1)
                    .height(Length::Fill)
                    .style(move |_| rounded(line, 0)),
            );
    }
    if main {
        // The preview is a card of its own, its frame round it.
        let width = editor_middle_width(ctx, editor, main);
        let preview = container(middle)
            .width(width)
            .height(Length::Fill)
            .clip(true)
            .style(crate::profile::frame_style(inks, theme.edge, 12.0, true));
        let dressed = crate::profile::cosmetic::dressed(
            ctx,
            &editor.shown,
            Mode::Side,
            width,
            preview.into(),
        );
        // Room above for its frame's top, which reaches past the card.
        let above =
            crate::profile::cosmetic::frame_room_above(ctx, &editor.shown, Mode::Side, width);
        parts = parts.push(
            container(dressed)
                .padding(Padding {
                    top: CARD_MARGIN.max(above.ceil() + 8.0),
                    ..Padding::new(CARD_MARGIN)
                })
                .width(width + 2.0 * CARD_MARGIN)
                .height(Length::Fill),
        );
    } else {
        parts = parts.push(container(middle).width(Length::Fill).height(Length::Fill));
    }
    if editor_tabs_shown(card_width, main) {
        parts = parts.push(editor_tabs(ctx, editor, &inks));
    }
    let mut card = stack![
        container(parts)
            .width(card_width)
            .height(card_height)
            .clip(true)
            .style(crate::profile::frame_style(inks, theme.edge, 12.0, true))
    ];
    if main {
        card = card.push(
            container(
                ctx.look
                    .icon_button(
                        Icon::X,
                        backcord_iced_kit::i18n::tr("Close"),
                        message(Event::Close),
                    )
                    .size(36.0, 18)
                    .below(),
            )
            .width(card_width)
            .align_x(Alignment::End)
            .padding(12),
        );
        if !editor.side {
            card = card.push(placed(
                crate::profile::editor::styles::show_styles(ctx),
                0.0,
                40.0,
            ));
        }
    }
    // The profiles' switch above its top left corner, outside it.
    let card = column![menus::profile_switch(ctx, editor, core), card]
        .spacing(SWITCH - 32.0)
        .width(card_width);
    // Over the window dimmed as under a dialog.
    let mut layers = stack![
        crate::controls::dimmed(message(Event::Close), crate::controls::DIM),
        container(crate::widgets::motion::grow(opaque(card))).center(Length::Fill)
    ];
    if let Some(window) = &editor.style {
        layers = layers.push(crate::profile::name_style::name_style_view(
            ctx,
            window,
            &editor.shown,
        ));
    }
    if let Some(draft) = &editor.badges {
        layers = layers.push(badges::view(ctx, draft));
    }
    if let Some(kind) = editor.wardrobe {
        layers = layers.push(wardrobe::view(
            ctx,
            &editor.shown,
            kind,
            worn(ctx, kind),
            editor.shop_shown,
            editor.confirming,
        ));
    }
    Some(opaque(layers))
}

/// The SKU of the collectible of `kind` the main profile wears, found among
/// those owned by the pictures it shows; empty for one worn but not found,
/// none for none.
fn worn(ctx: crate::ctx::Ctx<'_>, kind: backcord_core::chat::CollectibleKind) -> Option<String> {
    use backcord_core::chat::{CollectibleKind, Wardrobe};
    // What Discord has, not the preview, which wears what's chosen.
    let profile = &ctx
        .state
        .profile_editor
        .as_ref()?
        .profile
        .as_ref()?
        .fallback;
    let marks: Vec<&str> = match kind {
        CollectibleKind::Effect => return profile.effect.clone(),
        CollectibleKind::Decoration => profile.decoration.as_deref().into_iter().collect(),
        CollectibleKind::Nameplate => ctx
            .state
            .account_nameplate
            .as_ref()
            .map(|plate| &*plate.picture)
            .into_iter()
            .collect(),
        CollectibleKind::Frame => profile
            .frame
            .iter()
            .map(|layer| layer.src.as_str())
            .collect(),
    };
    if marks.is_empty() {
        return None;
    }
    // A decoration's address names its asset, not its SKU.
    let asset = |url: &str| {
        let (_, rest) = url.split_once("avatar-decoration-presets/")?;
        rest.split(['.', '?']).next().map(str::to_owned)
    };
    let owned = match ctx.state.profile_editor.as_ref().map(|core| &core.wardrobe) {
        Some(Wardrobe::Loaded { owned, .. }) => owned.as_slice(),
        _ => &[],
    };
    let found = owned.iter().filter(|item| item.kind == kind).find(|item| {
        marks.iter().any(|mark| {
            mark.contains(item.sku.as_str())
                || item
                    .picture
                    .as_deref()
                    .and_then(asset)
                    .is_some_and(|asset| mark.contains(asset.as_str()))
        })
    });
    Some(found.map(|item| item.sku.clone()).unwrap_or_default())
}

/// The board's note while the profile being edited loads, or after it
/// failed to.
fn board_note(ctx: crate::ctx::Ctx<'_>) -> Option<&'static str> {
    let core = ctx.state.profile_editor.as_ref()?;
    if core.loading() {
        Some(backcord_iced_kit::i18n::tr("Loading the board..."))
    } else if core.profile.is_none() {
        Some(backcord_iced_kit::i18n::tr("Couldn't load the board."))
    } else {
        None
    }
}

/// The tabs beside the main profile: its Board and Activity,
/// as the profile will show them, in the editor's words.
fn editor_tabs<'a>(
    ctx: crate::ctx::Ctx<'a>,
    editor: &'a Editor,
    inks: &crate::profile::look::Inks,
) -> Element<'a, Message> {
    let look = &editor.shown.look;
    let tab = editor
        .tab
        .filter(|tab| look.tabs.contains(tab))
        .unwrap_or(look.tabs[0]);
    let mut heads = row![].spacing(24);
    for entry in &look.tabs {
        let label = match entry {
            Tab::Board => backcord_iced_kit::i18n::tr("Board"),
            Tab::Activity => backcord_iced_kit::i18n::tr("Activity"),
            Tab::Friends | Tab::Servers => continue,
        };
        heads = heads.push(crate::controls::tab(
            crate::controls::Tab {
                label,
                chosen: *entry == tab,
                ink: inks.ink,
                rest: inks.ink2,
                bar: inks.ink,
                height: 26.0,
                rest_font: SEMIBOLD,
            },
            message(Event::Tab(*entry)),
        ));
    }
    let rule_ink = if inks.themed {
        faded(inks.ink, 0.12)
    } else {
        ctx.look.theme.line
    };
    column![
        heads,
        container(Space::new())
            .width(Length::Fill)
            .height(1)
            .style(move |_| rounded(rule_ink, 0)),
        crate::profile::full::editor_tab_page(ctx, &editor.shown, tab, inks, board_note(ctx)),
    ]
    .spacing(16)
    .padding(Padding {
        top: 58.0,
        right: 32.0,
        bottom: 22.0,
        left: 8.0,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
