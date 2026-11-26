//! Badge Spoofer, as Testcord's BadgeSpoofer has it but only on this
//! screen: this account's own profile shows the Discord badges chosen in
//! Settings' Extra Features, as if it had them. Nothing goes to Discord:
//! nobody else sees them, and the profile Discord keeps doesn't change.
//!
//! The badges are Discord's own pictures, from the same place the badges
//! of anyone's profile come from, so they show as Discord's do.

use crate::chat::{Profile, ProfileBadge};

/// A badge Badge Spoofer can show: its name in Settings, what it says on
/// hover as Discord words it, its picture's hash on Discord's CDN and where
/// a click on it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spoofable {
    pub label: &'static str,
    pub description: &'static str,
    pub hash: &'static str,
    pub link: Option<&'static str>,
}

const fn badge(
    label: &'static str,
    description: &'static str,
    hash: &'static str,
    link: Option<&'static str>,
) -> Spoofable {
    Spoofable {
        label,
        description,
        hash,
        link,
    }
}

const PREMIUM: Option<&str> = Some("https://discord.com/settings/premium");
const BUGS: Option<&str> =
    Some("https://support.discord.com/hc/en-us/articles/360046057772-Discord-Bugs");
const HOUSE: Option<&str> = Some("https://discord.com/settings/hypesquad-online");

/// The badges ticked one by one, in Discord's order, each a bit of
/// [`BadgeSpoofer::badges`] by its place.
pub const BADGES: [Spoofable; 10] = [
    badge(
        "Discord Staff",
        "Discord Staff",
        "5e74e9b61934fc1f67c65515d1f7e60d",
        Some("https://discord.com/company"),
    ),
    badge(
        "Partnered Server Owner",
        "Partnered Server Owner",
        "3f9748e53446a137a052f3454e2de41e",
        Some("https://discord.com/partners"),
    ),
    badge(
        "Moderator Programs Alumni",
        "Moderator Programs Alumni",
        "fee1624003e2fee35cb398e125dc479b",
        Some("https://discord.com/safety"),
    ),
    badge(
        "HypeSquad Events",
        "HypeSquad Events",
        "bf01d1073931f921909045f3a39fd264",
        Some("https://support.discord.com/hc/en-us/articles/360035962891-Profile-Badges-101"),
    ),
    badge(
        "Bug Hunter",
        "Discord Bug Hunter",
        "2717692c7dca7289b35297368a940dd0",
        BUGS,
    ),
    badge(
        "Golden Bug Hunter",
        "Discord Bug Hunter",
        "848f79194d4be5ff5f81505cbd0ce1e6",
        BUGS,
    ),
    badge(
        "Active Developer",
        "Active Developer",
        "6bdc42827a38498929a4920da12695d9",
        Some("https://support-dev.discord.com/hc/en-us/articles/10113997751447?ref=badge"),
    ),
    badge(
        "Early Verified Bot Developer",
        "Early Verified Bot Developer",
        "6df5892e0f35b051f8b61eace34f4967",
        None,
    ),
    badge(
        "Early Supporter",
        "Early Supporter",
        "7060786766c9c840eb3019e725d2b358",
        PREMIUM,
    ),
    badge(
        "Completed a Quest",
        "Completed a Quest",
        "7d9ae358c8c5e118768335dbe68b4fb8",
        Some("https://discord.com/discovery/quests"),
    ),
];

/// The HypeSquad houses, of which a profile shows one.
pub const HOUSES: [Spoofable; 3] = [
    badge(
        "Bravery",
        "HypeSquad Bravery",
        "8a88d63823d8a71cd5e390baa45efa02",
        HOUSE,
    ),
    badge(
        "Brilliance",
        "HypeSquad Brilliance",
        "011940fd013da3f7fb926e4a1cd2e618",
        HOUSE,
    ),
    badge(
        "Balance",
        "HypeSquad Balance",
        "3aa41de486fa12454c3761e8e223442e",
        HOUSE,
    ),
];

/// Nitro's badge, then its levels by how long one has subscribed.
pub const NITRO: [Spoofable; 9] = [
    badge(
        "Nitro",
        "Nitro",
        "2ba85e8026a8614b640c2837bcdfe21b",
        PREMIUM,
    ),
    badge(
        "Bronze",
        "Nitro Bronze",
        "4f33c4a9c64ce221936bd256c356f91f",
        PREMIUM,
    ),
    badge(
        "Silver",
        "Nitro Silver",
        "4514fab914bdbfb4ad2fa23df76121a6",
        PREMIUM,
    ),
    badge(
        "Gold",
        "Nitro Gold",
        "2895086c18d5531d499862e41d1155a6",
        PREMIUM,
    ),
    badge(
        "Platinum",
        "Nitro Platinum",
        "0334688279c8359120922938dcb1d6f8",
        PREMIUM,
    ),
    badge(
        "Diamond",
        "Nitro Diamond",
        "0d61871f72bb9a33a7ae568c1fb4f20a",
        PREMIUM,
    ),
    badge(
        "Emerald",
        "Nitro Emerald",
        "11e2d339068b55d3a506cff34d3780f3",
        PREMIUM,
    ),
    badge(
        "Ruby",
        "Nitro Ruby",
        "cd5e2cfd9d7f27a8cdcd3e8a8d5dc9f4",
        PREMIUM,
    ),
    badge(
        "Opal",
        "Nitro Opal",
        "5b154df19c53dce2af92c9b61e6be5e2",
        PREMIUM,
    ),
];

/// The server booster's levels, by how long one has boosted.
pub const BOOSTS: [Spoofable; 9] = [
    badge(
        "1 month",
        "Server Booster: 1 month",
        "51040c70d4f20a921ad6674ff86fc95c",
        PREMIUM,
    ),
    badge(
        "2 months",
        "Server Booster: 2 months",
        "0e4080d1d333bc7ad29ef6528b6f2fb7",
        PREMIUM,
    ),
    badge(
        "3 months",
        "Server Booster: 3 months",
        "72bed924410c304dbe3d00a6e593ff59",
        PREMIUM,
    ),
    badge(
        "6 months",
        "Server Booster: 6 months",
        "df199d2050d3ed4ebf84d64ae83989f8",
        PREMIUM,
    ),
    badge(
        "9 months",
        "Server Booster: 9 months",
        "996b3e870e8a22ce519b3a50e6bdd52f",
        PREMIUM,
    ),
    badge(
        "1 year",
        "Server Booster: 1 year",
        "991c9f39ee33d7537d9f408c3e53141e",
        PREMIUM,
    ),
    badge(
        "15 months",
        "Server Booster: 15 months",
        "cb3ae83c15e970e8f3d410bc62cb8b99",
        PREMIUM,
    ),
    badge(
        "18 months",
        "Server Booster: 18 months",
        "7142225d31238f6387d9f09efaa02759",
        PREMIUM,
    ),
    badge(
        "2 years",
        "Server Booster: 2 years",
        "ec92202290b48d0879b7413d2dde3bab",
        PREMIUM,
    ),
];

/// Badge Spoofer's settings: off unless Settings turn it on, and then the
/// badges chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadgeSpoofer {
    pub on: bool,
    /// The badges of [`BADGES`] chosen, a bit each by its place.
    pub badges: u32,
    /// The house, Nitro level and boost level chosen: none for 0, else the
    /// one at that place in [`HOUSES`], [`NITRO`] and [`BOOSTS`] after 1.
    pub house: u8,
    pub nitro: u8,
    pub boost: u8,
    /// The badges the profile really has show too, beside those chosen.
    pub keep_real: bool,
}

impl BadgeSpoofer {
    /// Off, with nothing chosen and the real badges kept.
    pub const OFF: Self = Self {
        on: false,
        badges: 0,
        house: 0,
        nitro: 0,
        boost: 0,
        keep_real: true,
    };

    /// Whether the badge at `index` of [`BADGES`] is chosen.
    pub fn has(&self, index: usize) -> bool {
        index < 32 && self.badges & (1 << index) != 0
    }

    /// The settings with the badge at `index` of [`BADGES`] chosen or not.
    #[must_use]
    pub fn with(mut self, index: usize, chosen: bool) -> Self {
        if index < 32 {
            if chosen {
                self.badges |= 1 << index;
            } else {
                self.badges &= !(1 << index);
            }
        }
        self
    }

    /// The badges chosen, in Discord's order: the first four of [`BADGES`],
    /// the house, the rest, then Nitro and the boost.
    fn chosen(&self) -> Vec<(usize, &'static Spoofable)> {
        let level = |list: &'static [Spoofable], at: u8| {
            usize::from(at)
                .checked_sub(1)
                .and_then(|index| list.get(index))
        };
        let ticked = |range: std::ops::Range<usize>| {
            range
                .filter(|index| self.has(*index))
                .map(|index| &BADGES[index])
                .collect::<Vec<_>>()
        };
        ticked(0..4)
            .into_iter()
            .chain(level(&HOUSES, self.house))
            .chain(ticked(4..BADGES.len()))
            .chain(level(&NITRO, self.nitro))
            .chain(level(&BOOSTS, self.boost))
            .map(|badge| (rank(badge.hash).unwrap_or(usize::MAX), badge))
            .collect()
    }
}

impl Default for BadgeSpoofer {
    fn default() -> Self {
        Self::OFF
    }
}

/// Every badge Badge Spoofer knows, in Discord's order.
fn ordered() -> impl Iterator<Item = &'static Spoofable> {
    BADGES[..4]
        .iter()
        .chain(&HOUSES)
        .chain(&BADGES[4..])
        .chain(&NITRO)
        .chain(&BOOSTS)
}

/// Where a badge comes in Discord's order, by its picture's hash.
fn rank(hash: &str) -> Option<usize> {
    ordered().position(|badge| badge.hash == hash)
}

/// The kind of badge of which a profile shows one: a house, Nitro or a
/// boost; none for the others.
fn group(hash: &str) -> Option<u8> {
    let within = |list: &[Spoofable]| list.iter().any(|badge| badge.hash == hash);
    if within(&HOUSES) {
        Some(0)
    } else if within(&NITRO) {
        Some(1)
    } else if within(&BOOSTS) {
        Some(2)
    } else {
        None
    }
}

/// The hash of a badge's picture on Discord's CDN, from its address.
fn hash_of(icon: &str) -> Option<&str> {
    let (_, rest) = icon.split_once("/badge-icons/")?;
    rest.split(['.', '?']).next()
}

/// The address of a badge's picture, where the badges of profiles come from.
pub fn icon(hash: &str) -> String {
    #[cfg(feature = "discord")]
    {
        backcord_discord::ready::badge_icon(hash)
    }
    #[cfg(not(feature = "discord"))]
    {
        format!("https://cdn.discordapp.com/badge-icons/{hash}.png")
    }
}

/// The badge a profile shows for one Badge Spoofer shows.
fn shown_badge(badge: &Spoofable) -> ProfileBadge {
    ProfileBadge {
        description: badge.description.to_owned(),
        icon: icon(badge.hash),
        link: badge.link.map(str::to_owned),
        level: None,
    }
}

/// The badges a profile shows with Badge Spoofer's `settings`: those it
/// really has, as they are, while it's off or nothing is chosen; else those
/// chosen, with its own beside them if they're kept, in Discord's order.
/// A real house, Nitro or boost badge goes when one of its kind is chosen,
/// and a real badge that is chosen too shows once.
pub fn shown(real: &[ProfileBadge], settings: &BadgeSpoofer) -> Vec<ProfileBadge> {
    let chosen = settings.chosen();
    if !settings.on || chosen.is_empty() {
        return real.to_vec();
    }
    let groups: Vec<u8> = chosen
        .iter()
        .filter_map(|(_, badge)| group(badge.hash))
        .collect();
    let mut badges: Vec<(usize, ProfileBadge)> = chosen
        .iter()
        .map(|(rank, badge)| (*rank, shown_badge(badge)))
        .collect();
    if settings.keep_real {
        // A badge Badge Spoofer doesn't know keeps its place after the
        // last known one before it.
        let mut last = 0;
        for badge in real {
            let hash = hash_of(&badge.icon);
            if hash.is_some_and(|hash| {
                chosen.iter().any(|(_, chosen)| chosen.hash == hash)
                    || group(hash).is_some_and(|kind| groups.contains(&kind))
            }) {
                continue;
            }
            if let Some(known) = hash.and_then(rank) {
                last = known;
            }
            badges.push((last, badge.clone()));
        }
    }
    // A stable sort keeps the real badges after a chosen one of the same
    // place, in their own order.
    badges.sort_by_key(|(rank, _)| *rank);
    badges.into_iter().map(|(_, badge)| badge).collect()
}

/// Whether `profile` is this account's own Discord profile, which alone
/// shows the badges chosen.
pub fn is_own(profile: &Profile, me: Option<&str>) -> bool {
    !profile.id.is_empty() && me == Some(profile.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(hash: &str, description: &str) -> ProfileBadge {
        ProfileBadge {
            description: description.into(),
            icon: icon(hash),
            link: None,
            level: None,
        }
    }

    fn descriptions(badges: &[ProfileBadge]) -> Vec<&str> {
        badges
            .iter()
            .map(|badge| badge.description.as_str())
            .collect()
    }

    #[test]
    fn off_or_with_nothing_chosen_the_real_badges_show() {
        let real = [real(HOUSES[0].hash, "HypeSquad Bravery")];
        assert_eq!(shown(&real, &BadgeSpoofer::OFF), real);
        let chosen_but_off = BadgeSpoofer {
            nitro: 4,
            ..BadgeSpoofer::OFF
        }
        .with(0, true);
        assert_eq!(shown(&real, &chosen_but_off), real);
        let on_with_nothing = BadgeSpoofer {
            on: true,
            ..BadgeSpoofer::OFF
        };
        assert_eq!(shown(&real, &on_with_nothing), real);
    }

    #[test]
    fn the_chosen_badges_show_in_discords_order_with_discords_pictures() {
        let settings = BadgeSpoofer {
            on: true,
            house: 3,
            nitro: 4,
            boost: 9,
            keep_real: false,
            ..BadgeSpoofer::OFF
        }
        .with(8, true)
        .with(0, true)
        .with(6, true);
        let badges = shown(&[real(BADGES[3].hash, "HypeSquad Events")], &settings);
        assert_eq!(
            descriptions(&badges),
            [
                "Discord Staff",
                "HypeSquad Balance",
                "Active Developer",
                "Early Supporter",
                "Nitro Gold",
                "Server Booster: 2 years",
            ],
            "the real badge goes while the real ones aren't kept"
        );
        assert!(
            badges[0]
                .icon
                .ends_with("/badge-icons/5e74e9b61934fc1f67c65515d1f7e60d.png")
        );
        assert_eq!(
            badges[0].link.as_deref(),
            Some("https://discord.com/company")
        );
        assert!(badges.iter().all(|badge| badge.level.is_none()));
    }

    #[test]
    fn real_badges_stay_in_their_places_beside_the_chosen_ones() {
        let settings = BadgeSpoofer {
            on: true,
            house: 2,
            nitro: 1,
            ..BadgeSpoofer::OFF
        }
        .with(0, true)
        .with(9, true);
        let mut levelled = real("ffff", "Played for 100 hours");
        levelled.link = Some("https://example.com".into());
        let real = [
            // Replaced by the house chosen, and shown once though chosen.
            real(HOUSES[0].hash, "HypeSquad Bravery"),
            real(BADGES[6].hash, "Active Developer"),
            real(BADGES[9].hash, "Completed a Quest"),
            // Replaced by the Nitro chosen.
            real(NITRO[5].hash, "Nitro Diamond"),
            real(BOOSTS[0].hash, "Server Booster: 1 month"),
            levelled.clone(),
        ];
        let badges = shown(&real, &settings);
        assert_eq!(
            descriptions(&badges),
            [
                "Discord Staff",
                "HypeSquad Brilliance",
                "Active Developer",
                "Completed a Quest",
                "Nitro",
                "Server Booster: 1 month",
                "Played for 100 hours",
            ]
        );
        assert_eq!(badges[6], levelled, "a badge it doesn't know is kept whole");
    }

    #[test]
    fn badges_are_chosen_and_unchosen_by_their_place() {
        let settings = BadgeSpoofer::OFF.with(3, true).with(5, true).with(3, false);
        assert!(!settings.has(3) && settings.has(5));
        assert_eq!(settings.badges, 1 << 5);
        assert_eq!(BadgeSpoofer::OFF.with(40, true), BadgeSpoofer::OFF);
    }

    #[test]
    fn only_this_accounts_profile_is_its_own() {
        let profile = Profile {
            id: "42".into(),
            ..Default::default()
        };
        assert!(is_own(&profile, Some("42")));
        assert!(!is_own(&profile, Some("7")));
        assert!(!is_own(&profile, None));
        assert!(!is_own(&Profile::default(), Some("")));
    }
}
