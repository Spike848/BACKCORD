//! Permissions Viewer: what a member of a Discord server may do there, or
//! in one of its channels or categories, each permission with what gives
//! or takes it (the owner, Administrator, a role, an overwrite, a
//! timeout), or what a role gives on its own. The window opens it from a
//! member's menu, a server's, a channel's or a category's, and a role's
//! card, while Extra Features turn it on. The answer is the service's
//! (`backcord_discord::ready::Server::explain`), worked out once as it
//! opens; the groups and titles are Discord's, as a role's and a
//! channel's Permissions pages show them.

use super::*;

/// Whose permissions to show, and where: a server by its id, or a channel
/// or category of it by its room id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionsOf {
    /// A member's, by user id.
    Member { place: String, user: String },
    /// This account's own.
    Mine(String),
    /// What a role gives on its own, in the server, by role id.
    Role { place: String, role: String },
}

/// The Permissions window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionsView {
    /// Whose: the member's name, or the role's with its @.
    pub name: String,
    /// The RGB colour of the role, or of the member's name.
    pub color: Option<u32>,
    /// Where: the server's name, or the channel's or category's.
    pub place: String,
    /// What gives every permission, when something does: owning the
    /// server, or Administrator.
    pub everything: Option<PermissionSource>,
    /// Discord's groups, each under its heading.
    pub groups: Vec<PermissionsGroup>,
}

/// A group of the window's permissions, under its heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionsGroup {
    pub heading: &'static str,
    pub rows: Vec<PermissionRow>,
}

/// One permission: whether it's held, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRow {
    /// Its bit's number in Discord's permissions.
    pub bit: u32,
    /// Its title, as Discord's Permissions page words it there.
    pub title: &'static str,
    pub held: bool,
    pub source: PermissionSource,
}

/// What gives a permission or takes it, named for the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionSource {
    /// Owning the server.
    Owner,
    /// Administrator, from the role it names.
    Administrator(String),
    /// A role, by name; @everyone's is `@everyone`.
    Role(String),
    /// A channel's or category's overwrite for a role, by its name
    /// (`@everyone` for everyone's).
    RoleOverwrite(String),
    /// A channel's or category's overwrite for the member themselves.
    MemberOverwrite,
    /// Their timeout takes it away.
    Timeout,
    /// Nothing gives it.
    Nothing,
}

impl Application {
    /// Whether Permissions Viewer can show who may do what at a place: a
    /// Discord server, or a channel or category of it, whose roles are
    /// known.
    pub fn offers_permissions(&self, place: &str) -> bool {
        #[cfg(feature = "discord")]
        if let Some((server, _, _)) = self.discord_place(place) {
            return !self.state.preview && !server.roles.is_empty();
        }
        let _ = place;
        false
    }

    pub(super) fn show_permissions(&mut self, of: PermissionsOf) -> Vec<Effect> {
        #[cfg(feature = "discord")]
        {
            self.state.permissions = self.permissions_view(&of);
        }
        let _ = of;
        vec![]
    }

    #[cfg(feature = "discord")]
    fn permissions_view(&self, of: &PermissionsOf) -> Option<PermissionsView> {
        use backcord_discord::ready::{Grant, permission::ADMINISTRATOR};

        let place = match of {
            PermissionsOf::Member { place, .. }
            | PermissionsOf::Mine(place)
            | PermissionsOf::Role { place, .. } => place.as_str(),
        };
        if !self.offers_permissions(place) {
            return None;
        }
        let (server, id, me) = self.discord_place(place)?;
        let role_name = |role: &str| {
            if role == server.id {
                "@everyone".to_owned()
            } else {
                server
                    .role_names
                    .get(role)
                    .cloned()
                    .unwrap_or_else(|| role.to_owned())
            }
        };
        let source = |grant: Grant| match grant {
            Grant::Owner => PermissionSource::Owner,
            Grant::Administrator(role) => PermissionSource::Administrator(role_name(&role)),
            Grant::Role(role) => PermissionSource::Role(role_name(&role)),
            Grant::Overwrite { member: true, .. } => PermissionSource::MemberOverwrite,
            Grant::Overwrite { id, .. } => PermissionSource::RoleOverwrite(role_name(&id)),
            Grant::Timeout => PermissionSource::Timeout,
            Grant::Nothing => PermissionSource::Nothing,
        };
        // A role on its own is shown in the server as a whole; a member
        // where they were asked about, with its overwrites.
        let user = match of {
            PermissionsOf::Member { user, .. } => Some(user.as_str()),
            PermissionsOf::Mine(_) => Some(me),
            PermissionsOf::Role { .. } => None,
        };
        let overwrites =
            id.filter(|_| user.is_some())
                .and_then(|id| match server.server.channel(id) {
                    Some(channel) => Some(server.channel_overwrites(channel)),
                    None => server
                        .categories
                        .get(id)
                        .map(|category| &category.overwrites[..]),
                });
        let verdict: Box<dyn Fn(u64) -> (bool, PermissionSource)> = match (of, user) {
            (PermissionsOf::Role { role, .. }, _) => {
                let bits = server.roles.get(role).copied()?;
                let name = role_name(role);
                Box::new(move |bit| {
                    if bits & ADMINISTRATOR != 0 {
                        (true, PermissionSource::Administrator(name.clone()))
                    } else if bits & bit != 0 {
                        (true, PermissionSource::Role(name.clone()))
                    } else {
                        (false, PermissionSource::Nothing)
                    }
                })
            }
            (_, Some(user)) => {
                let roles = self.member_roles(place, user);
                // This account's own timeout is the server's to know.
                let timed_out = self.timed_out(place, user)
                    || server
                        .my_timeout
                        .as_ref()
                        .is_some_and(|timeout| timeout.user == user);
                Box::new(move |bit| {
                    let found = server.explain(bit, overwrites, user, &roles, timed_out);
                    (found.held, source(found.grant))
                })
            }
            (_, None) => return None,
        };
        let (name, color) = match (of, user) {
            (PermissionsOf::Role { role, .. }, _) => (
                format!("@{}", role_name(role).trim_start_matches('@')),
                server
                    .role_colors
                    .get(role.as_str())
                    .map(|(_, color)| *color),
            ),
            (_, Some(user)) => match self.state.person(user) {
                Some(person) => (person.name, person.color),
                None => (user.to_owned(), None),
            },
            (_, None) => return None,
        };
        // Owning the server or Administrator gives them all.
        let everything = match verdict(ADMINISTRATOR) {
            (true, source @ (PermissionSource::Owner | PermissionSource::Administrator(_))) => {
                Some(source)
            }
            _ => None,
        };
        // A channel's or category's page's groups and titles there, else a
        // role's, with those only Community servers list on one.
        let titled: Vec<(&'static str, Vec<(u32, &'static str)>)> = match overwrites {
            Some(_) => {
                let kind = self.channel_kind(place);
                channel_groups(kind)
                    .iter()
                    .map(|(heading, bits)| {
                        let titles = bits
                            .iter()
                            .map(|&bit| (bit, channel_permission(bit, kind, false).0))
                            .collect();
                        (*heading, titles)
                    })
                    .collect()
            }
            None => {
                let community = self
                    .state
                    .servers
                    .iter()
                    .any(|known| known.id == discord::server_id(&server.id) && known.community);
                ROLE_PERMISSIONS
                    .iter()
                    .map(|group| {
                        let titles = group
                            .permissions
                            .iter()
                            .filter(|info| community || !info.community)
                            .map(|info| (info.bit, info.title))
                            .collect();
                        (group.heading, titles)
                    })
                    .collect()
            }
        };
        let groups = titled
            .into_iter()
            .map(|(heading, titles)| PermissionsGroup {
                heading,
                rows: titles
                    .into_iter()
                    .map(|(bit, title)| {
                        let (held, source) = verdict(1 << bit);
                        PermissionRow {
                            bit,
                            title,
                            held,
                            source,
                        }
                    })
                    .collect(),
            })
            .collect();
        let place = match overwrites {
            Some(_) => self.channel_title(place),
            None => server.name.clone(),
        };
        Some(PermissionsView {
            name,
            color,
            place,
            everything,
            groups,
        })
    }
}
