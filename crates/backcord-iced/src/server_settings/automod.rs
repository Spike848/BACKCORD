//! AutoMod: the server's rules, each a row with its kind in words, its
//! switch (saved at once), Edit and Delete, which asks first. Create Rule
//! lists the kinds the server may still add; the rule's form takes the
//! page's place: its name, what sets it off, and its actions. The form's
//! lists are texts several lines high, which the screen keeps
//! ([`Local`]) while the form is open.

use backcord_core::{
    application::{
        AUTOMOD_PRESETS, AUTOMOD_TIMEOUTS, AutomodForm, AutomodInput, AutomodPage, AutomodText,
        ServerSettings,
    },
    chat::{AutomodKind, AutomodRule},
};
use backcord_iced_kit::i18n::{t, tr};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use super::{Drop, Event, Screen, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    dialog,
    field::{self, Draft},
    fonts::{BOLD, SEMIBOLD},
    form,
};

/// The form's texts several lines high.
const KEYWORDS: &str = "automod-keywords";
const REGEX: &str = "automod-regex";
const ALLOWED: &str = "automod-allowed";
/// Delete's question's width.
const CONFIRM: f32 = 440.0;

/// The form's lists as they're typed, while a rule's form is open.
#[derive(Default)]
pub struct Local {
    keywords: Draft,
    regex: Draft,
    allowed: Draft,
}

/// Which of the form's lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Keywords,
    Regex,
    Allowed,
}

impl Area {
    pub fn text(self) -> AutomodText {
        match self {
            Self::Keywords => AutomodText::Keywords,
            Self::Regex => AutomodText::Regex,
            Self::Allowed => AutomodText::Allowed,
        }
    }
}

impl Local {
    /// The lists follow the form, as when it opens on a rule.
    pub fn sync(&self, form: Option<&AutomodForm>) {
        let (keywords, regex, allowed) = form.map_or(("", "", ""), |form| {
            (
                form.keywords.as_str(),
                form.regex.as_str(),
                form.allowed.as_str(),
            )
        });
        for (draft, text) in [
            (&self.keywords, keywords),
            (&self.regex, regex),
            (&self.allowed, allowed),
        ] {
            if draft.text() != text {
                draft.reset(text);
            }
        }
    }
}

fn automod(input: AutomodInput) -> Message {
    message(Event::Automod(input))
}

/// What a kind of rule does, under its name in Create Rule's list.
fn about(kind: AutomodKind) -> &'static str {
    match kind {
        AutomodKind::Keyword => {
            "Block messages with words or patterns you choose. A server can have up to 6 of these."
        }
        AutomodKind::Preset => {
            "Block words from Discord's lists of profanity, sexual content and slurs."
        }
        AutomodKind::Spam => "Block messages Discord suspects are spam.",
        AutomodKind::MentionSpam => "Block messages with too many role and user mentions.",
        AutomodKind::MemberProfile => {
            "Keep members whose names have words you choose from talking in the server."
        }
        AutomodKind::Other(_) => "A rule set up in Discord's app.",
    }
}

impl<'a> View<'a> {
    /// AutoMod: Create Rule (or the kinds it offers), then the rules; the
    /// open form instead, once there is one.
    pub(super) fn automod_page(
        self,
        core: &'a ServerSettings,
        page: &'a AutomodPage,
        screen: &'a Screen,
    ) -> Element<'a, Message> {
        if let Some(rule) = &page.form {
            return self.automod_form(core, rule, screen);
        }
        let theme = self.cx.look.theme;
        let offered = page.offered();
        let mut parts = vec![
            form::title(theme, tr("AutoMod")),
            form::note(
                theme,
                tr(
                    "AutoMod blocks words, spam and mention spam before moderators have to. Each change to a rule is saved at once.",
                ),
            ),
        ];
        if page.picking {
            let mut kinds: Vec<Element<'a, Message>> =
                vec![form::heading(theme, tr("CHOOSE A RULE TYPE"))];
            kinds.extend(offered.into_iter().map(|kind| self.automod_kind(kind)));
            kinds.push(form::buttons([action(
                theme,
                tr("Cancel"),
                Kind::Plain,
                Some(automod(AutomodInput::Cancel)),
            )
            .into()]));
            parts.push(form::group(kinds));
        } else {
            let full = offered.is_empty();
            let mut line: Vec<Element<'a, Message>> = vec![
                action(
                    theme,
                    tr("Create Rule"),
                    Kind::Primary,
                    (!full).then(|| automod(AutomodInput::New)),
                )
                .into(),
            ];
            if full {
                line.push(form::note(theme, tr("Every kind of rule is at its limit.")));
            }
            parts.push(form::buttons(line));
        }
        if let Some(error) = &page.error {
            parts.push(form::error(theme, t(error)).into());
        }
        if page.rules.is_empty() {
            parts.push(form::note(
                theme,
                tr("This server has no AutoMod rules yet."),
            ));
        }
        for rule in &page.rules {
            parts.push(self.automod_row(page, rule));
        }
        form::page(parts)
    }

    /// A kind Create Rule offers: its name and what it does.
    fn automod_kind(self, kind: AutomodKind) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        button(
            column![
                text(tr(kind.words()))
                    .size(16)
                    .font(SEMIBOLD)
                    .color(theme.text),
                text(tr(about(kind))).size(13).color(theme.secondary),
            ]
            .spacing(2),
        )
        .padding(12)
        .width(Length::Fill)
        .on_press(automod(AutomodInput::Kind(kind)))
        .style(crate::controls::lit_style(None, theme.wash(0.06), 8.0))
        .into()
    }

    /// A rule: its name and kind, its switch, Edit (for a kind BACKCORD
    /// sets up) and Delete.
    fn automod_row(self, page: &'a AutomodPage, rule: &'a AutomodRule) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let free = page.sending.is_none();
        let mut line = row![
            column![
                text(rule.name.as_str())
                    .size(16)
                    .font(SEMIBOLD)
                    .color(theme.text)
                    .wrapping(text::Wrapping::None),
                text(tr(rule.kind.words()))
                    .size(13)
                    .color(theme.secondary)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2)
            .width(Length::Fill)
            .clip(true),
            form::toggle(
                theme,
                rule.enabled,
                free.then(|| automod(AutomodInput::Enable(rule.id.clone()))),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if rule.kind.limit() > 0 {
            line = line.push(action(
                theme,
                tr("Edit"),
                Kind::Plain,
                Some(automod(AutomodInput::Edit(rule.id.clone()))),
            ));
        }
        line = line.push(action(
            theme,
            tr("Delete"),
            Kind::Danger,
            free.then(|| automod(AutomodInput::Delete(rule.id.clone()))),
        ));
        let (ground, edge) = (theme.wash(0.02), theme.line);
        container(line)
            .padding(12)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(ground.into()),
                border: crate::style::edge(edge, 8.0),
                ..Default::default()
            })
            .into()
    }

    /// A rule's form: its name, what sets it off for its kind, its
    /// actions, then why it can't be saved yet and Save Rule.
    fn automod_form(
        self,
        core: &'a ServerSettings,
        rule: &'a AutomodForm,
        screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let kind = rule.kind;
        let ready = !rule.saving;
        let write = |field: AutomodText| {
            ready.then_some(move |text: String| automod(AutomodInput::Write(field, text)))
        };
        let mut parts = vec![
            form::title(
                theme,
                if rule.editing.is_some() {
                    tr("Edit Rule")
                } else {
                    tr("Create Rule")
                },
            ),
            form::note(theme, tr(kind.words())),
            form::group([
                form::heading(theme, tr("RULE NAME")),
                form::entry(theme, "automod-name", "", &rule.name)
                    .on_input_maybe(write(AutomodText::Name))
                    .into(),
            ]),
        ];
        if kind.has_keywords() {
            parts.push(form::group([
                form::heading(theme, tr("KEYWORDS")),
                form::note(
                    theme,
                    tr("Separate keywords with commas or new lines. Put * at a word's start or end to match part of a word."),
                ),
                self.automod_area(rule, screen, Area::Keywords, ready),
            ]));
            parts.push(form::group([
                form::heading(theme, tr("REGEX PATTERNS")),
                form::note(theme, tr("One pattern a line, in Rust's regex syntax.")),
                self.automod_area(rule, screen, Area::Regex, ready),
            ]));
        }
        if kind == AutomodKind::Preset {
            let mut lists = vec![form::heading(theme, tr("LISTS"))];
            for (preset, name) in AUTOMOD_PRESETS {
                lists.push(self.automod_check(
                    tr(name),
                    rule.presets.contains(&preset),
                    ready.then(|| automod(AutomodInput::Preset(preset))),
                ));
            }
            parts.push(form::group(lists));
        }
        if kind.allowed_limit().is_some() {
            parts.push(form::group([
                form::heading(theme, tr("ALLOWED WORDS")),
                form::note(
                    theme,
                    tr("Words that never set the rule off, separated with commas or new lines."),
                ),
                self.automod_area(rule, screen, Area::Allowed, ready),
            ]));
        }
        if kind == AutomodKind::MentionSpam {
            parts.push(form::group([
                form::heading(theme, tr("MENTION LIMIT")),
                form::note(
                    theme,
                    tr("The most role and user mentions a message may have, from 1 to 50."),
                ),
                form::entry(theme, "automod-mentions", "", &rule.mention_limit)
                    .on_input_maybe(write(AutomodText::MentionLimit))
                    .into(),
            ]));
            parts.push(
                form::switch(
                    tr("Mention raid protection"),
                    rule.raid_protection,
                    ready.then(|| automod(AutomodInput::RaidProtection)),
                )
                .description(tr(
                    "Also catch members who spread mentions across many messages.",
                ))
                .view(self.cx.look),
            );
        }
        if kind == AutomodKind::Spam {
            parts.push(form::note(
                theme,
                tr("Discord decides what looks like spam; there's nothing more to set."),
            ));
        }
        parts.push(form::group(self.automod_actions(core, rule, screen)));
        if rule.exempts() {
            parts.push(form::note(
                theme,
                tr("The rule's exempt roles and channels stay as they are."),
            ));
        }
        let problem = rule.problem();
        if let Some(problem) = problem {
            parts.push(form::error(theme, tr(problem)).into());
        }
        if let Some(error) = &rule.error {
            parts.push(form::error(theme, t(error)).into());
        }
        parts.push(form::buttons([
            action(
                theme,
                tr("Cancel"),
                Kind::Plain,
                ready.then(|| automod(AutomodInput::Cancel)),
            )
            .into(),
            action(
                theme,
                if ready {
                    tr("Save Rule")
                } else {
                    tr("Saving...")
                },
                Kind::Primary,
                (ready && problem.is_none()).then(|| automod(AutomodInput::Save)),
            )
            .into(),
        ]));
        form::page(parts)
    }

    /// The form's actions, as far as its kind allows them: block the
    /// message (with what its author is told), alert a channel, time the
    /// member out, or block their interactions.
    fn automod_actions(
        self,
        core: &'a ServerSettings,
        rule: &'a AutomodForm,
        screen: &'a Screen,
    ) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let kind = rule.kind;
        let ready = !rule.saving;
        let press = |input: AutomodInput| ready.then(|| automod(input));
        let mut actions = vec![form::heading(theme, tr("ACTIONS"))];
        if kind.blocks_messages() {
            actions.push(self.automod_check(
                tr("Block message"),
                rule.block,
                press(AutomodInput::Block),
            ));
            if rule.block {
                actions.push(
                    form::entry(
                        theme,
                        "automod-message",
                        tr("What the author is told (optional)"),
                        &rule.block_message,
                    )
                    .on_input_maybe(ready.then_some(|text: String| {
                        automod(AutomodInput::Write(AutomodText::BlockMessage, text))
                    }))
                    .into(),
                );
            }
        }
        actions.push(self.automod_check(
            tr("Send an alert to a channel"),
            rule.alert,
            press(AutomodInput::Alert),
        ));
        if rule.alert {
            let channels = self.cx.app.settings_channels(&core.server, false);
            let names: Vec<String> = channels
                .iter()
                .map(|(_, name)| format!("#{name}"))
                .collect();
            let current = rule
                .alert_channel
                .as_ref()
                .and_then(|chosen| channels.iter().position(|(id, _)| id == chosen));
            let ids: Vec<String> = channels.into_iter().map(|(id, _)| id).collect();
            actions.push(
                form::dropdown(
                    names,
                    current,
                    screen.open == Some(Drop::AutomodAlert),
                    message(Event::Drop(Drop::AutomodAlert)),
                    move |index| {
                        automod(AutomodInput::AlertChannel(
                            ids.get(index).cloned().unwrap_or_default(),
                        ))
                    },
                )
                .enabled(ready)
                .view(self.cx.look),
            );
        }
        if kind.times_out() {
            actions.push(self.automod_check(
                tr("Time out the member"),
                rule.timeout,
                press(AutomodInput::Timeout),
            ));
            if rule.timeout {
                let words: Vec<String> = AUTOMOD_TIMEOUTS
                    .iter()
                    .map(|(_, words)| tr(words).to_owned())
                    .collect();
                let current = AUTOMOD_TIMEOUTS
                    .iter()
                    .position(|(seconds, _)| *seconds == rule.timeout_seconds);
                actions.push(
                    form::dropdown(
                        words,
                        current,
                        screen.open == Some(Drop::AutomodTimeout),
                        message(Event::Drop(Drop::AutomodTimeout)),
                        |index| {
                            automod(AutomodInput::TimeoutFor(
                                AUTOMOD_TIMEOUTS
                                    .get(index)
                                    .map_or(60, |(seconds, _)| *seconds),
                            ))
                        },
                    )
                    .enabled(ready)
                    .view(self.cx.look),
                );
            }
        }
        if kind == AutomodKind::MemberProfile {
            actions.push(self.automod_check(
                tr("Block member interactions"),
                rule.block_interaction,
                press(AutomodInput::BlockInteraction),
            ));
            actions.push(form::note(
                theme,
                tr("They can't talk in the server until they change their names."),
            ));
        }
        actions
    }

    /// One of the form's lists, with how many more it may have.
    fn automod_area(
        self,
        rule: &'a AutomodForm,
        screen: &'a Screen,
        area: Area,
        enabled: bool,
    ) -> Element<'a, Message> {
        let local = &screen.automod;
        let (draft, id) = match area {
            Area::Keywords => (&local.keywords, KEYWORDS),
            Area::Regex => (&local.regex, REGEX),
            Area::Allowed => (&local.allowed, ALLOWED),
        };
        let on: fn(field::Event) -> Message = match area {
            Area::Keywords => |event| message(Event::AutomodArea(Area::Keywords, event)),
            Area::Regex => |event| message(Event::AutomodArea(Area::Regex, event)),
            Area::Allowed => |event| message(Event::AutomodArea(Area::Allowed, event)),
        };
        form::area(
            self.cx.look.theme,
            draft,
            id,
            "",
            rule.left(area.text()),
            enabled,
            on,
        )
    }

    /// A box to tick with its label, the whole row taking the click.
    fn automod_check(
        self,
        label: &'a str,
        on: bool,
        press: Option<Message>,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        button(
            row![
                self.cx.look.checkbox(on, false),
                text(label).size(15).color(theme.text)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .on_press_maybe(press)
        .style(crate::controls::lit_style(None, theme.wash(0.06), 6.0))
        .into()
    }

    /// Delete's question, while it's asked.
    pub(super) fn automod_dialog(self, page: &'a AutomodPage) -> Option<Element<'a, Message>> {
        let id = page.deleting.as_ref()?;
        let rule = page.rule(id)?;
        let theme = self.cx.look.theme;
        let deleting = page.sending.as_ref() == Some(id);
        let body = column![
            text(tr("Delete Rule"))
                .size(20)
                .font(BOLD)
                .color(theme.text),
            text(t(format!(
                "Delete the rule {}? This can't be undone.",
                rule.name
            )))
            .size(14)
            .color(theme.secondary),
            row![
                Space::new().width(Length::Fill),
                action(
                    theme,
                    tr("Cancel"),
                    Kind::Plain,
                    (!deleting).then(|| automod(AutomodInput::Cancel)),
                ),
                action(
                    theme,
                    if deleting {
                        tr("Deleting...")
                    } else {
                        tr("Delete Rule")
                    },
                    Kind::Danger,
                    (!deleting).then(|| automod(AutomodInput::ConfirmDelete)),
                ),
            ]
            .spacing(form::TIGHT),
        ]
        .spacing(12)
        .padding(16);
        Some(dialog::panel(
            theme,
            body.into(),
            CONFIRM,
            None,
            automod(AutomodInput::Cancel),
        ))
    }
}
