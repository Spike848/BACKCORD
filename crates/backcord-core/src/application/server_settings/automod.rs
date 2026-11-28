//! AutoMod: the server's AutoMod rules, as Discord's client lists them,
//! each with its switch, which goes to Discord at once, Edit and Delete
//! (after asking). Create Rule offers the kinds the server may still add,
//! then the rule's form: what sets it off, and what it does. Discord's
//! refusals show on the page or the form. Exempt roles and channels are
//! kept as the rule has them. See "AutoMod" in docs/discord/server-settings.md.

use crate::chat::{AutomodAction, AutomodAsk, AutomodDid, AutomodKind, AutomodRule};

/// The timeouts the form offers, in seconds, with their words.
pub const AUTOMOD_TIMEOUTS: [(u32, &str); 6] = [
    (60, "60 seconds"),
    (300, "5 minutes"),
    (600, "10 minutes"),
    (3600, "1 hour"),
    (86_400, "1 day"),
    (604_800, "1 week"),
];
/// Discord's keyword lists for a Preset rule, by number.
pub const AUTOMOD_PRESETS: [(u8, &str); 3] =
    [(1, "Profanity"), (2, "Sexual Content"), (3, "Slurs")];
/// Discord's limits on a rule's words and patterns.
pub const KEYWORDS_LIMIT: usize = 1000;
pub const KEYWORD_LENGTH: usize = 60;
pub const REGEX_LIMIT: usize = 10;
pub const REGEX_LENGTH: usize = 260;
pub const MENTION_LIMIT: u8 = 50;
pub const CUSTOM_MESSAGE_LENGTH: usize = 150;
pub const RULE_NAME_LIMIT: usize = 100;
/// The mention limit a new Mention Spam rule starts at.
const DEFAULT_MENTIONS: u8 = 20;

impl AutomodKind {
    /// How many rules of this kind a server may have.
    pub fn limit(self) -> usize {
        match self {
            Self::Keyword => 6,
            Self::Other(_) => 0,
            _ => 1,
        }
    }

    /// The kind in words, as Discord's client names it.
    pub fn words(self) -> &'static str {
        match self {
            Self::Keyword => "Block Custom Words",
            Self::Preset => "Block Commonly Flagged Words",
            Self::Spam => "Block Suspected Spam Content",
            Self::MentionSpam => "Block Mention Spam",
            Self::MemberProfile => "Block Words in Member Profile Names",
            Self::Other(_) => "Other rule",
        }
    }

    /// Whether its rules may time a member out, as Discord allows.
    pub fn times_out(self) -> bool {
        matches!(self, Self::Keyword | Self::MentionSpam)
    }

    /// Whether it looks at messages, which it may block.
    pub fn blocks_messages(self) -> bool {
        !matches!(self, Self::MemberProfile | Self::Other(_))
    }

    /// Whether it has keywords and regex patterns.
    pub fn has_keywords(self) -> bool {
        matches!(self, Self::Keyword | Self::MemberProfile)
    }

    /// Whether it lets some words through, and how many at most.
    pub fn allowed_limit(self) -> Option<usize> {
        match self {
            Self::Keyword | Self::MemberProfile => Some(100),
            Self::Preset => Some(1000),
            _ => None,
        }
    }
}

/// A text of the rule's form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomodText {
    Name,
    /// Separated by commas or lines.
    Keywords,
    /// One a line.
    Regex,
    /// Separated by commas or lines.
    Allowed,
    MentionLimit,
    /// What a blocked message's author is told.
    BlockMessage,
}

/// What's done on AutoMod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutomodInput {
    /// Flip a rule's switch, by its id.
    Enable(String),
    /// Create Rule: the kinds the server may still add.
    New,
    /// A kind chosen, which opens a new rule's form.
    Kind(AutomodKind),
    Edit(String),
    /// Ask before deleting a rule, then delete it.
    Delete(String),
    ConfirmDelete,
    /// Close the kinds, the form or the question.
    Cancel,
    Write(AutomodText, String),
    /// Flip one of a Preset rule's lists.
    Preset(u8),
    RaidProtection,
    /// Flip an action.
    Block,
    Alert,
    Timeout,
    BlockInteraction,
    /// The channel alerts go to, by its id.
    AlertChannel(String),
    /// Seconds, one of [`AUTOMOD_TIMEOUTS`].
    TimeoutFor(u32),
    Save,
}

/// AutoMod's rules, and what's open over them.
#[derive(Clone, Debug, Default)]
pub struct AutomodPage {
    pub rules: Vec<AutomodRule>,
    /// Create Rule's kinds are showing.
    pub picking: bool,
    pub form: Option<Box<AutomodForm>>,
    /// The rule Delete asks about, by its id.
    pub deleting: Option<String>,
    /// The rule whose switch or deletion Discord is taking.
    pub sending: Option<String>,
    /// Why Discord didn't take a switch or a deletion.
    pub error: Option<String>,
}

/// A rule being made or changed: the texts as typed, and the actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomodForm {
    /// The rule changed, by its id; none for a new one.
    pub editing: Option<String>,
    pub kind: AutomodKind,
    pub name: String,
    pub keywords: String,
    pub regex: String,
    pub allowed: String,
    pub presets: Vec<u8>,
    pub mention_limit: String,
    pub raid_protection: bool,
    pub block: bool,
    pub block_message: String,
    pub alert: bool,
    pub alert_channel: Option<String>,
    pub timeout: bool,
    pub timeout_seconds: u32,
    pub block_interaction: bool,
    /// What the form doesn't change, kept as the rule has it.
    enabled: bool,
    exempt_roles: Vec<String>,
    exempt_channels: Vec<String>,
    others: Vec<AutomodAction>,
    pub saving: bool,
    pub error: Option<String>,
}

impl AutomodPage {
    pub(super) fn new(rules: Vec<AutomodRule>) -> Self {
        Self {
            rules,
            ..Self::default()
        }
    }

    /// How many rules of a kind the server has.
    pub fn count(&self, kind: AutomodKind) -> usize {
        self.rules.iter().filter(|rule| rule.kind == kind).count()
    }

    /// The kinds the server may still add a rule of.
    pub fn offered(&self) -> Vec<AutomodKind> {
        AutomodKind::ALL
            .into_iter()
            .filter(|&kind| self.count(kind) < kind.limit())
            .collect()
    }

    pub fn rule(&self, id: &str) -> Option<&AutomodRule> {
        self.rules.iter().find(|rule| rule.id == id)
    }

    /// Takes what was done, giving what to ask Discord.
    pub(super) fn input(&mut self, input: AutomodInput) -> Option<AutomodAsk> {
        if let Some(form) = self.form.as_mut().filter(|form| !form.saving) {
            match form.input(&input) {
                Some(true) => {}
                Some(false) => return None,
                None => {
                    // Save, or Cancel.
                    if input == AutomodInput::Cancel {
                        self.form = None;
                        return None;
                    }
                    if form.problem().is_some() {
                        return None;
                    }
                    form.saving = true;
                    form.error = None;
                    let rule = Box::new(form.rule());
                    return Some(if form.editing.is_some() {
                        AutomodAsk::Edit(rule)
                    } else {
                        AutomodAsk::Create(rule)
                    });
                }
            }
        }
        if self.form.is_some() {
            return None;
        }
        match input {
            AutomodInput::Enable(id) => {
                if self.sending.is_some() {
                    return None;
                }
                let rule = self.rules.iter_mut().find(|rule| rule.id == id)?;
                rule.enabled = !rule.enabled;
                let enabled = rule.enabled;
                self.sending = Some(id.clone());
                self.error = None;
                Some(AutomodAsk::Enable { rule: id, enabled })
            }
            AutomodInput::New => {
                self.picking = !self.offered().is_empty();
                None
            }
            AutomodInput::Kind(kind) if self.picking && self.offered().contains(&kind) => {
                self.picking = false;
                self.form = Some(Box::new(AutomodForm::new(kind)));
                None
            }
            AutomodInput::Edit(id) => {
                let rule = self.rule(&id).filter(|rule| rule.kind.limit() > 0)?;
                self.form = Some(Box::new(AutomodForm::edit(rule)));
                self.picking = false;
                None
            }
            AutomodInput::Delete(id) if self.rule(&id).is_some() => {
                self.deleting = Some(id);
                None
            }
            AutomodInput::ConfirmDelete if self.sending.is_none() => {
                let id = self.deleting.clone()?;
                self.sending = Some(id.clone());
                self.error = None;
                Some(AutomodAsk::Delete(id))
            }
            AutomodInput::Cancel => {
                self.picking = false;
                if self.sending.is_none() || self.sending != self.deleting {
                    self.deleting = None;
                }
                None
            }
            _ => None,
        }
    }

    /// What Discord did with what was asked.
    pub(super) fn done(
        &mut self,
        did: AutomodDid,
        result: Result<Option<Box<AutomodRule>>, String>,
    ) {
        match did {
            AutomodDid::Created | AutomodDid::Edited(_) => {
                let Some(form) = self.form.as_mut().filter(|form| form.saving) else {
                    // Closed meanwhile: the list still takes the rule.
                    if let Ok(Some(rule)) = result {
                        self.put(*rule);
                    }
                    return;
                };
                match result {
                    Ok(rule) => {
                        if let Some(rule) = rule {
                            self.put(*rule);
                        }
                        self.form = None;
                    }
                    Err(error) => {
                        form.saving = false;
                        form.error = Some(error);
                    }
                }
            }
            AutomodDid::Enabled(id) => {
                if self.sending.as_ref() == Some(&id) {
                    self.sending = None;
                }
                match result {
                    Ok(Some(rule)) => self.put(*rule),
                    Ok(None) => {}
                    Err(error) => {
                        if let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == id) {
                            rule.enabled = !rule.enabled;
                        }
                        self.error = Some(error);
                    }
                }
            }
            AutomodDid::Deleted(id) => {
                if self.sending.as_ref() == Some(&id) {
                    self.sending = None;
                }
                if self.deleting.as_ref() == Some(&id) {
                    self.deleting = None;
                }
                match result {
                    Ok(_) => self.rules.retain(|rule| rule.id != id),
                    Err(error) => self.error = Some(error),
                }
            }
        }
    }

    /// A rule as Discord has it, in its place or after the others.
    fn put(&mut self, rule: AutomodRule) {
        match self.rules.iter_mut().find(|known| known.id == rule.id) {
            Some(known) => *known = rule,
            None => self.rules.push(rule),
        }
    }
}

impl AutomodForm {
    /// A new rule's form, with Discord's defaults: messages blocked, or a
    /// member's interactions for a Member Profile rule.
    pub fn new(kind: AutomodKind) -> Self {
        Self {
            editing: None,
            kind,
            name: kind.words().to_owned(),
            keywords: String::new(),
            regex: String::new(),
            allowed: String::new(),
            presets: AUTOMOD_PRESETS.iter().map(|(preset, _)| *preset).collect(),
            mention_limit: DEFAULT_MENTIONS.to_string(),
            raid_protection: false,
            block: kind.blocks_messages(),
            block_message: String::new(),
            alert: false,
            alert_channel: None,
            timeout: false,
            timeout_seconds: AUTOMOD_TIMEOUTS[0].0,
            block_interaction: kind == AutomodKind::MemberProfile,
            enabled: true,
            exempt_roles: Vec::new(),
            exempt_channels: Vec::new(),
            others: Vec::new(),
            saving: false,
            error: None,
        }
    }

    /// A rule's form, as the rule stands.
    pub fn edit(rule: &AutomodRule) -> Self {
        let mut form = Self::new(rule.kind);
        form.editing = Some(rule.id.clone());
        form.name.clone_from(&rule.name);
        form.keywords = rule.keywords.join(", ");
        form.regex = rule.regex.join("\n");
        form.allowed = rule.allowed.join(", ");
        form.presets.clone_from(&rule.presets);
        if rule.kind == AutomodKind::MentionSpam {
            form.mention_limit = rule.mention_limit.to_string();
        }
        form.raid_protection = rule.raid_protection;
        form.block = false;
        form.block_interaction = false;
        for action in &rule.actions {
            match action {
                AutomodAction::Block { message } => {
                    form.block = true;
                    form.block_message = message.clone().unwrap_or_default();
                }
                AutomodAction::Alert { channel } => {
                    form.alert = true;
                    form.alert_channel = Some(channel.clone());
                }
                AutomodAction::Timeout { seconds } => {
                    form.timeout = true;
                    form.timeout_seconds = *seconds;
                }
                AutomodAction::BlockInteraction => form.block_interaction = true,
                AutomodAction::Other { .. } => form.others.push(action.clone()),
            }
        }
        form.enabled = rule.enabled;
        form.exempt_roles.clone_from(&rule.exempt_roles);
        form.exempt_channels.clone_from(&rule.exempt_channels);
        form
    }

    /// Whether the rule leaves some roles or channels alone, which the
    /// form keeps.
    pub fn exempts(&self) -> bool {
        !self.exempt_roles.is_empty() || !self.exempt_channels.is_empty()
    }

    /// Takes a change: whether it was the form's to take (none for Save
    /// and Cancel, which are the page's).
    fn input(&mut self, input: &AutomodInput) -> Option<bool> {
        let kind = self.kind;
        match input {
            AutomodInput::Save | AutomodInput::Cancel => return None,
            AutomodInput::Write(field, text) => {
                let text = text.clone();
                match field {
                    AutomodText::Name => self.name = text,
                    AutomodText::Keywords => self.keywords = text,
                    AutomodText::Regex => self.regex = text,
                    AutomodText::Allowed => self.allowed = text,
                    AutomodText::MentionLimit => {
                        if text.chars().all(|c| c.is_ascii_digit()) && text.len() <= 2 {
                            self.mention_limit = text;
                        }
                    }
                    AutomodText::BlockMessage => self.block_message = text,
                }
            }
            AutomodInput::Preset(preset) if kind == AutomodKind::Preset => {
                if let Some(at) = self.presets.iter().position(|known| known == preset) {
                    self.presets.remove(at);
                } else if AUTOMOD_PRESETS.iter().any(|(known, _)| known == preset) {
                    self.presets.push(*preset);
                    self.presets.sort_unstable();
                }
            }
            AutomodInput::RaidProtection if kind == AutomodKind::MentionSpam => {
                self.raid_protection = !self.raid_protection;
            }
            AutomodInput::Block if kind.blocks_messages() => self.block = !self.block,
            AutomodInput::Alert => self.alert = !self.alert,
            AutomodInput::Timeout if kind.times_out() => self.timeout = !self.timeout,
            AutomodInput::BlockInteraction if kind == AutomodKind::MemberProfile => {
                self.block_interaction = !self.block_interaction;
            }
            AutomodInput::AlertChannel(channel) => {
                self.alert_channel = Some(channel.clone());
                self.alert = true;
            }
            AutomodInput::TimeoutFor(seconds)
                if kind.times_out()
                    && AUTOMOD_TIMEOUTS.iter().any(|(known, _)| known == seconds) =>
            {
                self.timeout_seconds = *seconds;
                self.timeout = true;
            }
            _ => return Some(false),
        }
        Some(true)
    }

    /// Why the rule can't be saved as it is, in Discord's limits.
    pub fn problem(&self) -> Option<&'static str> {
        let kind = self.kind;
        let name = self.name.trim();
        if name.is_empty() {
            return Some("Give the rule a name.");
        }
        if name.chars().count() > RULE_NAME_LIMIT {
            return Some("A rule's name can be up to 100 characters.");
        }
        if kind.has_keywords() {
            let (keywords, regex) = (self.keyword_list(), self.regex_list());
            if keywords.is_empty() && regex.is_empty() {
                return Some("Add a keyword or a regex pattern.");
            }
            if keywords.len() > KEYWORDS_LIMIT {
                return Some("A rule can have up to 1000 keywords.");
            }
            if keywords
                .iter()
                .any(|word| word.chars().count() > KEYWORD_LENGTH)
            {
                return Some("Each keyword can be up to 60 characters.");
            }
            if regex.len() > REGEX_LIMIT {
                return Some("A rule can have up to 10 regex patterns.");
            }
            if regex
                .iter()
                .any(|pattern| pattern.chars().count() > REGEX_LENGTH)
            {
                return Some("Each regex pattern can be up to 260 characters.");
            }
        }
        if let Some(limit) = kind.allowed_limit() {
            let allowed = self.allowed_list();
            if allowed.len() > limit {
                return Some(if limit == 100 {
                    "A rule can allow up to 100 words."
                } else {
                    "A rule can allow up to 1000 words."
                });
            }
            if allowed
                .iter()
                .any(|word| word.chars().count() > KEYWORD_LENGTH)
            {
                return Some("Each allowed word can be up to 60 characters.");
            }
        }
        if kind == AutomodKind::Preset && self.presets.is_empty() {
            return Some("Choose at least one list.");
        }
        if kind == AutomodKind::MentionSpam && self.mentions().is_none() {
            return Some("The mention limit is from 1 to 50.");
        }
        if self.block
            && kind.blocks_messages()
            && self.block_message.chars().count() > CUSTOM_MESSAGE_LENGTH
        {
            return Some("The custom message can be up to 150 characters.");
        }
        if self.alert && self.alert_channel.is_none() {
            return Some("Choose a channel for the alerts.");
        }
        if self.actions().is_empty() {
            return Some("Choose at least one action.");
        }
        None
    }

    /// How many more a list may have, or characters a text, below none
    /// once past Discord's limit.
    pub fn left(&self, field: AutomodText) -> i64 {
        let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
        let (limit, used) = match field {
            AutomodText::Name => (RULE_NAME_LIMIT, self.name.trim().chars().count()),
            AutomodText::Keywords => (KEYWORDS_LIMIT, self.keyword_list().len()),
            AutomodText::Regex => (REGEX_LIMIT, self.regex_list().len()),
            AutomodText::Allowed => (
                self.kind.allowed_limit().unwrap_or(0),
                self.allowed_list().len(),
            ),
            AutomodText::BlockMessage => {
                (CUSTOM_MESSAGE_LENGTH, self.block_message.chars().count())
            }
            AutomodText::MentionLimit => (0, 0),
        };
        count(limit) - count(used)
    }

    fn keyword_list(&self) -> Vec<String> {
        items(&self.keywords, true)
    }

    fn regex_list(&self) -> Vec<String> {
        items(&self.regex, false)
    }

    fn allowed_list(&self) -> Vec<String> {
        items(&self.allowed, true)
    }

    fn mentions(&self) -> Option<u8> {
        self.mention_limit
            .trim()
            .parse()
            .ok()
            .filter(|limit| (1..=MENTION_LIMIT).contains(limit))
    }

    /// The actions chosen, as far as the kind allows them, then those
    /// BACKCORD doesn't know.
    fn actions(&self) -> Vec<AutomodAction> {
        let kind = self.kind;
        let mut actions = Vec::new();
        if self.block && kind.blocks_messages() {
            let message = self.block_message.trim();
            actions.push(AutomodAction::Block {
                message: (!message.is_empty()).then(|| message.to_owned()),
            });
        }
        if self.alert
            && let Some(channel) = &self.alert_channel
        {
            actions.push(AutomodAction::Alert {
                channel: channel.clone(),
            });
        }
        if self.timeout && kind.times_out() {
            actions.push(AutomodAction::Timeout {
                seconds: self.timeout_seconds,
            });
        }
        if self.block_interaction && kind == AutomodKind::MemberProfile {
            actions.push(AutomodAction::BlockInteraction);
        }
        actions.extend(self.others.iter().cloned());
        actions
    }

    /// The rule as the form would save it.
    pub fn rule(&self) -> AutomodRule {
        let kind = self.kind;
        let keywords = kind.has_keywords();
        AutomodRule {
            id: self.editing.clone().unwrap_or_default(),
            name: self.name.trim().to_owned(),
            kind,
            enabled: self.enabled,
            keywords: if keywords {
                self.keyword_list()
            } else {
                Vec::new()
            },
            regex: if keywords {
                self.regex_list()
            } else {
                Vec::new()
            },
            allowed: if kind.allowed_limit().is_some() {
                self.allowed_list()
            } else {
                Vec::new()
            },
            presets: if kind == AutomodKind::Preset {
                self.presets.clone()
            } else {
                Vec::new()
            },
            mention_limit: if kind == AutomodKind::MentionSpam {
                self.mentions().unwrap_or(DEFAULT_MENTIONS)
            } else {
                0
            },
            raid_protection: kind == AutomodKind::MentionSpam && self.raid_protection,
            actions: self.actions(),
            exempt_roles: self.exempt_roles.clone(),
            exempt_channels: self.exempt_channels.clone(),
        }
    }
}

/// A list typed as text: split at lines, and at commas when `commas`,
/// each trimmed, the empty and repeated left out.
fn items(text: &str, commas: bool) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for item in text
        .split(|c: char| c == '\n' || (commas && c == ','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if !found.iter().any(|known| known == item) {
            found.push(item.to_owned());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, kind: AutomodKind) -> AutomodRule {
        AutomodRule {
            id: id.into(),
            name: format!("rule {id}"),
            kind,
            enabled: true,
            keywords: vec![],
            regex: vec![],
            allowed: vec![],
            presets: vec![],
            mention_limit: 0,
            raid_protection: false,
            actions: vec![AutomodAction::Block { message: None }],
            exempt_roles: vec![],
            exempt_channels: vec![],
        }
    }

    fn write(page: &mut AutomodPage, field: AutomodText, text: &str) {
        assert_eq!(page.input(AutomodInput::Write(field, text.into())), None);
    }

    #[test]
    fn create_rule_offers_only_the_kinds_under_their_limit() {
        let mut rules: Vec<AutomodRule> = (0..5)
            .map(|n| rule(&n.to_string(), AutomodKind::Keyword))
            .collect();
        rules.push(rule("10", AutomodKind::Spam));
        rules.push(rule("11", AutomodKind::Other(7)));
        let mut page = AutomodPage::new(rules);
        assert_eq!(
            page.offered(),
            [
                AutomodKind::Keyword,
                AutomodKind::Preset,
                AutomodKind::MentionSpam,
                AutomodKind::MemberProfile
            ]
        );
        page.rules.push(rule("5", AutomodKind::Keyword));
        assert!(
            !page.offered().contains(&AutomodKind::Keyword),
            "six at most"
        );
        page.input(AutomodInput::New);
        assert!(page.picking);
        assert_eq!(page.input(AutomodInput::Kind(AutomodKind::Spam)), None);
        assert!(page.form.is_none(), "the server has its spam rule");
        page.input(AutomodInput::Kind(AutomodKind::Preset));
        assert_eq!(
            page.form.as_ref().map(|form| form.kind),
            Some(AutomodKind::Preset)
        );
        assert!(!page.picking);
        // A kind BACKCORD doesn't set up isn't edited.
        page.input(AutomodInput::Cancel);
        page.input(AutomodInput::Edit("11".into()));
        assert!(page.form.is_none());
    }

    #[test]
    fn a_keyword_rule_is_saved_with_its_lists_and_actions() {
        let mut page = AutomodPage::new(vec![]);
        page.input(AutomodInput::New);
        page.input(AutomodInput::Kind(AutomodKind::Keyword));
        assert_eq!(
            page.form.as_ref().and_then(|form| form.problem()),
            Some("Add a keyword or a regex pattern.")
        );
        write(&mut page, AutomodText::Name, " No spoilers ");
        write(
            &mut page,
            AutomodText::Keywords,
            "spoiler, ending\n*leak*, spoiler",
        );
        write(&mut page, AutomodText::Regex, "a{1,3}b\n\n");
        write(&mut page, AutomodText::Allowed, "spoilerfree");
        page.input(AutomodInput::Alert);
        assert_eq!(
            page.form.as_ref().and_then(|form| form.problem()),
            Some("Choose a channel for the alerts.")
        );
        page.input(AutomodInput::AlertChannel("30".into()));
        page.input(AutomodInput::TimeoutFor(600));
        write(&mut page, AutomodText::BlockMessage, "Not here.");
        let Some(AutomodAsk::Create(rule)) = page.input(AutomodInput::Save) else {
            panic!("not created");
        };
        assert_eq!(rule.name, "No spoilers");
        assert_eq!(rule.keywords, ["spoiler", "ending", "*leak*"]);
        assert_eq!(rule.regex, ["a{1,3}b"], "a pattern keeps its commas");
        assert_eq!(rule.allowed, ["spoilerfree"]);
        assert_eq!(
            rule.actions,
            [
                AutomodAction::Block {
                    message: Some("Not here.".into())
                },
                AutomodAction::Alert {
                    channel: "30".into()
                },
                AutomodAction::Timeout { seconds: 600 },
            ]
        );
        assert!(rule.enabled, "on, as Discord's client makes them");
        assert_eq!(page.input(AutomodInput::Save), None, "once");

        // Discord's refusal shows on the form, which stays to try again.
        page.done(AutomodDid::Created, Err("Invalid regex".into()));
        let form = page.form.as_ref().unwrap();
        assert!(!form.saving && form.error.as_deref() == Some("Invalid regex"));
        page.input(AutomodInput::Save);
        let mut made = *rule;
        made.id = "40".into();
        page.done(AutomodDid::Created, Ok(Some(Box::new(made))));
        assert!(page.form.is_none());
        assert_eq!(page.count(AutomodKind::Keyword), 1);
    }

    #[test]
    fn each_kind_saves_only_what_it_has() {
        let form = |kind| {
            let mut form = AutomodForm::new(kind);
            form.keywords = "word".into();
            form.allowed = "fine".into();
            form.alert = true;
            form.alert_channel = Some("30".into());
            form.timeout = true;
            form.raid_protection = true;
            form
        };
        // Spam: nothing more, and no timeout.
        let spam = form(AutomodKind::Spam).rule();
        assert!(spam.keywords.is_empty() && spam.allowed.is_empty());
        assert!(
            !spam
                .actions
                .contains(&AutomodAction::Timeout { seconds: 60 })
        );
        assert!(!spam.raid_protection);
        // Presets: the lists and allowed words.
        let mut preset = form(AutomodKind::Preset);
        assert_eq!(preset.presets, [1, 2, 3]);
        preset.input(&AutomodInput::Preset(2));
        assert_eq!(preset.rule().presets, [1, 3]);
        assert_eq!(preset.rule().allowed, ["fine"]);
        assert!(preset.rule().keywords.is_empty());
        // Mentions: the limit, from 1 to 50, and raid protection.
        let mut mentions = form(AutomodKind::MentionSpam);
        assert_eq!(mentions.rule().mention_limit, 20);
        assert!(mentions.rule().raid_protection);
        mentions.mention_limit = "51".into();
        assert_eq!(
            mentions.problem(),
            Some("The mention limit is from 1 to 50.")
        );
        mentions.mention_limit = "0".into();
        assert!(mentions.problem().is_some());
        // A member's profile: interactions blocked, never a message, nor a
        // timeout.
        let profile = form(AutomodKind::MemberProfile).rule();
        assert_eq!(
            profile.actions,
            [
                AutomodAction::Alert {
                    channel: "30".into()
                },
                AutomodAction::BlockInteraction
            ]
        );
        assert_eq!(profile.keywords, ["word"]);
        let mut none = AutomodForm::new(AutomodKind::MemberProfile);
        none.keywords = "word".into();
        none.block_interaction = false;
        assert_eq!(none.problem(), Some("Choose at least one action."));
    }

    #[test]
    fn limits_are_discords() {
        let mut form = AutomodForm::new(AutomodKind::Keyword);
        form.keywords = "x".repeat(61);
        assert_eq!(
            form.problem(),
            Some("Each keyword can be up to 60 characters.")
        );
        form.keywords = (0..1001)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(form.problem(), Some("A rule can have up to 1000 keywords."));
        form.keywords = "ok".into();
        form.regex = (0..11)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            form.problem(),
            Some("A rule can have up to 10 regex patterns.")
        );
        form.regex.clear();
        form.allowed = (0..101)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(form.problem(), Some("A rule can allow up to 100 words."));
        let mut preset = AutomodForm::new(AutomodKind::Preset);
        preset.allowed = form.allowed.clone();
        assert_eq!(preset.problem(), None, "a Preset rule allows 1000");
        form.allowed.clear();
        form.block_message = "x".repeat(151);
        assert_eq!(
            form.problem(),
            Some("The custom message can be up to 150 characters.")
        );
        form.block_message.clear();
        form.name = "  ".into();
        assert_eq!(form.problem(), Some("Give the rule a name."));
    }

    #[test]
    fn editing_keeps_what_the_form_doesnt_show() {
        let mut kept = rule("7", AutomodKind::Keyword);
        kept.keywords = vec!["a".into(), "b".into()];
        kept.regex = vec!["x,y".into(), "z".into()];
        kept.enabled = false;
        kept.exempt_roles = vec!["20".into()];
        kept.exempt_channels = vec!["30".into()];
        kept.actions.push(AutomodAction::Other {
            kind: 9,
            metadata: serde_json::json!({"x": 1}),
        });
        let mut page = AutomodPage::new(vec![kept.clone()]);
        page.input(AutomodInput::Edit("7".into()));
        let form = page.form.as_ref().unwrap();
        assert_eq!(
            (form.keywords.as_str(), form.regex.as_str()),
            ("a, b", "x,y\nz")
        );
        assert!(form.exempts());
        let Some(AutomodAsk::Edit(edited)) = page.input(AutomodInput::Save) else {
            panic!("not edited");
        };
        assert_eq!(*edited, kept, "unchanged, it saves the rule as it was");
    }

    #[test]
    fn the_switch_shows_at_once_and_goes_back_if_refused_and_delete_asks_first() {
        let mut page = AutomodPage::new(vec![rule("7", AutomodKind::Spam)]);
        assert_eq!(
            page.input(AutomodInput::Enable("7".into())),
            Some(AutomodAsk::Enable {
                rule: "7".into(),
                enabled: false
            })
        );
        assert!(!page.rules[0].enabled);
        assert_eq!(
            page.input(AutomodInput::Enable("7".into())),
            None,
            "one at a time"
        );
        page.done(
            AutomodDid::Enabled("7".into()),
            Err("Missing Permissions".into()),
        );
        assert!(page.rules[0].enabled && page.sending.is_none());
        assert_eq!(page.error.as_deref(), Some("Missing Permissions"));

        assert_eq!(page.input(AutomodInput::Delete("7".into())), None);
        assert_eq!(page.deleting.as_deref(), Some("7"));
        assert_eq!(
            page.input(AutomodInput::ConfirmDelete),
            Some(AutomodAsk::Delete("7".into()))
        );
        page.done(AutomodDid::Deleted("7".into()), Ok(None));
        assert!(page.rules.is_empty() && page.deleting.is_none());
    }
}
