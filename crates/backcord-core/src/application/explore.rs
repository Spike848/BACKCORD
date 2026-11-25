//! Explore servers, under Add a server on the rail: Italian Discord
//! servers DISBOARD lists, found by name, tag or description and sorted by
//! how many of their members are online or by name. Join enters one in
//! BACKCORD by its public invite, through Join a Server's steps unseen,
//! then opens it; a server without one sends to DISBOARD's own check in
//! the browser, whose invite Add a server takes.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::chat::ExploreServer;

/// How Explore servers sorts its list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExploreSort {
    /// By how many of their members are online, as DISBOARD last counted.
    #[default]
    Online,
    Name,
}

/// How joining one of the listed servers goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExploreJoin {
    /// Asking for its public invite.
    Finding,
    /// Looking the invite up and joining.
    Joining,
    /// Discord asked for a captcha, in the browser.
    Captcha,
    /// It has no public invite: DISBOARD gives one after its check.
    NoInvite,
    Failed(String),
}

/// Explore servers, while it's open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Explore {
    /// The servers found so far, each once, in DISBOARD's order.
    pub servers: Vec<ExploreServer>,
    /// How many of DISBOARD's pages were asked for, and whether one is on
    /// its way.
    pub sources: usize,
    pub loading: bool,
    pub error: Option<String>,
    /// DISBOARD didn't answer, so the list shows servers kept from before.
    pub saved: bool,
    pub search: String,
    pub sort: ExploreSort,
    /// Highest first, for the counts; Z to A, for names.
    pub descending: bool,
    pub joining: HashMap<String, ExploreJoin>,
    /// The servers whose whole description shows.
    pub expanded: HashSet<String>,
}

impl Explore {
    /// Whether DISBOARD has pages BACKCORD hasn't read.
    pub fn more(&self) -> bool {
        self.sources < backcord_sources()
    }

    /// The servers the search finds, in the chosen order.
    pub fn shown(&self) -> Vec<&ExploreServer> {
        let words: Vec<String> = self
            .search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let mut shown: Vec<&ExploreServer> = self
            .servers
            .iter()
            .filter(|server| {
                let name = server.name.to_lowercase();
                let description = server.description.to_lowercase();
                words.iter().all(|word| {
                    name.contains(word.as_str())
                        || description.contains(word.as_str())
                        || server
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(word.as_str()))
                })
            })
            .collect();
        match self.sort {
            ExploreSort::Online => shown.sort_by_key(|server| server.online.unwrap_or(0)),
            ExploreSort::Name => shown.sort_by_cached_key(|server| server.name.to_lowercase()),
        }
        // Counts come highest first, names from A, unless turned around.
        if self.descending {
            shown.reverse();
        }
        shown
    }
}

/// How many of DISBOARD's pages there are to read.
fn backcord_sources() -> usize {
    #[cfg(feature = "discord")]
    {
        backcord_discord::explore::SOURCES.len()
    }
    #[cfg(not(feature = "discord"))]
    {
        0
    }
}

/// How many pages the list reads as it opens.
const FIRST_SOURCES: usize = 2;

impl Application {
    /// Opens Explore servers and reads DISBOARD's first pages.
    pub(super) fn open_explore(&mut self) -> Vec<Effect> {
        if self.state.preview {
            return vec![];
        }
        self.state.explore = Some(Explore {
            descending: true,
            ..Explore::default()
        });
        (0..FIRST_SOURCES.min(backcord_sources()))
            .flat_map(|_| self.explore_more())
            .collect()
    }

    /// Reads DISBOARD's next page.
    pub(super) fn explore_more(&mut self) -> Vec<Effect> {
        let Some(explore) = self.state.explore.as_mut().filter(|explore| explore.more()) else {
            return vec![];
        };
        let source = explore.sources;
        explore.sources += 1;
        explore.loading = true;
        explore.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ExplorePage { source },
        ))]
    }

    pub(super) fn explore_page(
        &mut self,
        source: usize,
        result: Result<Vec<ExploreServer>, String>,
        saved: bool,
    ) -> Vec<Effect> {
        let Some(explore) = self.state.explore.as_mut() else {
            return vec![];
        };
        if source + 1 >= explore.sources {
            explore.loading = false;
        }
        match result {
            Ok(found) => {
                explore.saved |= saved;
                for server in found {
                    if !explore
                        .servers
                        .iter()
                        .any(|known| known.guild == server.guild)
                    {
                        explore.servers.push(server);
                    }
                }
            }
            Err(error) => {
                // A later page that fails still shows the first's servers.
                if explore.servers.is_empty() {
                    explore.error = Some(error);
                }
            }
        }
        vec![]
    }

    /// Shows a server's whole description, or only its start again.
    pub(super) fn explore_expand(&mut self, guild: String) {
        if let Some(explore) = self.state.explore.as_mut()
            && !explore.expanded.remove(&guild)
        {
            explore.expanded.insert(guild);
        }
    }

    /// Reads the pages again after an error.
    pub(super) fn explore_again(&mut self) -> Vec<Effect> {
        let Some(explore) = self.state.explore.as_mut() else {
            return vec![];
        };
        explore.sources = 0;
        explore.error = None;
        (0..FIRST_SOURCES.min(backcord_sources()))
            .flat_map(|_| self.explore_more())
            .collect()
    }

    pub(super) fn explore_sort(&mut self, sort: ExploreSort) {
        if let Some(explore) = self.state.explore.as_mut() {
            if explore.sort == sort {
                explore.descending = !explore.descending;
            } else {
                explore.sort = sort;
                // Counts start from the highest, names from A.
                explore.descending = sort == ExploreSort::Online;
            }
        }
    }

    /// The room id of a listed server this account is in.
    pub fn explored_member(&self, guild: &str) -> Option<String> {
        let id = discord::server_id(guild);
        self.state
            .servers
            .iter()
            .any(|server| server.id == id)
            .then_some(id)
    }

    /// Joins a listed server by its public invite, or opens it for a
    /// member.
    pub(super) fn explore_join(&mut self, guild: String) -> Vec<Effect> {
        if let Some(server) = self.explored_member(&guild) {
            self.state.explore = None;
            return self.dispatch(Action::Navigate(Navigation::Server(server)));
        }
        let busy = self.state.join.is_some();
        let Some(explore) = self.state.explore.as_mut() else {
            return vec![];
        };
        if busy
            || matches!(
                explore.joining.get(&guild),
                Some(ExploreJoin::Finding | ExploreJoin::Joining | ExploreJoin::Captcha)
            )
        {
            return vec![];
        }
        if !self.state.discord_online {
            explore.joining.insert(
                guild,
                ExploreJoin::Failed("Discord isn't connected.".into()),
            );
            return vec![];
        }
        explore.joining.insert(guild.clone(), ExploreJoin::Finding);
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ExploreInvite { guild },
        ))]
    }

    pub(super) fn explore_invite(
        &mut self,
        guild: String,
        result: Result<Option<String>, String>,
    ) -> Vec<Effect> {
        let busy = self.state.join.is_some();
        let Some(explore) = self.state.explore.as_mut() else {
            return vec![];
        };
        if explore.joining.get(&guild) != Some(&ExploreJoin::Finding) {
            return vec![];
        }
        let code = match result {
            Ok(Some(code)) if !busy => code,
            Ok(Some(_)) => {
                explore.joining.insert(
                    guild,
                    ExploreJoin::Failed("Another server is joining; try again.".into()),
                );
                return vec![];
            }
            Ok(None) => {
                explore.joining.insert(guild, ExploreJoin::NoInvite);
                return vec![];
            }
            Err(error) => {
                explore.joining.insert(guild, ExploreJoin::Failed(error));
                return vec![];
            }
        };
        explore.joining.insert(guild.clone(), ExploreJoin::Joining);
        // Join a Server's steps, unseen: the invite looked up, then joined
        // at once.
        self.state.join = Some(JoinDialog {
            code: Some(code.clone()),
            from_link: true,
            resolving: true,
            explore: Some(guild),
            ..JoinDialog::default()
        });
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ResolveInvite { code, typed: None },
        ))]
    }

    /// A join Explore servers started stopped: why, on its server's row.
    pub(super) fn explore_join_failed(&mut self, guild: &str, error: String) {
        self.state.join = None;
        if let Some(explore) = self.state.explore.as_mut() {
            explore
                .joining
                .insert(guild.to_owned(), ExploreJoin::Failed(error));
        }
    }

    pub(super) fn explore_captcha(&mut self, guild: &str) {
        if let Some(explore) = self.state.explore.as_mut() {
            explore
                .joining
                .insert(guild.to_owned(), ExploreJoin::Captcha);
        }
    }

    /// DISBOARD's page that gives a server's invite after its check.
    pub(super) fn explore_in_browser(&self, guild: &str) -> Vec<Effect> {
        if !guild.bytes().all(|byte| byte.is_ascii_digit()) {
            return vec![];
        }
        vec![Effect::OpenPage(format!(
            "https://disboard.org/it/server/join/{guild}"
        ))]
    }
}
