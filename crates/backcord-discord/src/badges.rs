//! This account's badges, as Discord's profile editor shows and arranges
//! them: `GET /users/@me/badges` lists every badge, those owned in the order
//! the profile shows them, each saying whether it's hidden; `PATCH
//! /users/@me/badges/settings` saves `{display_order, hidden_badges}`, each a
//! list of badge ids, and answers with both.

use serde_json::{Value, json};

/// A badge, owned or one that can be earned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// What it says of this account, as "10 years, 3 months".
    pub info: String,
    /// The address of its picture, a PNG.
    pub icon: Option<String>,
    pub owned: bool,
    pub hidden: bool,
    /// Others can still earn it.
    pub earnable: bool,
}

/// The badges in an answer from `GET /users/@me/badges`, in its order.
pub fn parse(answer: &Value) -> Option<Vec<Badge>> {
    let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    Some(
        answer["badges"]
            .as_array()?
            .iter()
            .filter_map(|badge| {
                Some(Badge {
                    id: u32::try_from(badge["badge_id"].as_u64()?).ok()?,
                    name: text(&badge["name"]),
                    description: text(&badge["description"]),
                    info: text(&badge["info_label"]),
                    icon: badge["simple_icon_raster_url"]
                        .as_str()
                        .filter(|url| !url.is_empty())
                        .map(str::to_owned),
                    owned: badge["owned"].as_bool().unwrap_or(false),
                    hidden: badge["hidden"].as_bool().unwrap_or(false),
                    earnable: badge["is_earnable"].as_bool().unwrap_or(false),
                })
            })
            .collect(),
    )
}

/// The body that shows the badges owned in `order`, with `hidden` hidden.
pub fn settings(order: &[u32], hidden: &[u32]) -> Value {
    json!({"display_order": order, "hidden_badges": hidden})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_badges_in_their_order() {
        let answer = json!({"badges": [
            {"badge_id": 22, "name": "Discord Staff", "is_earnable": false, "owned": false, "hidden": false},
            {"badge_id": 1, "name": "Nitro", "description": "Stay subscribed",
             "info_label": "Subscriber since 3/22/25",
             "simple_icon_raster_url": "https://cdn.discordapp.com/assets/content/n.png",
             "is_earnable": true, "owned": true, "hidden": false},
            {"badge_id": 18, "name": "Account Age", "owned": true, "hidden": true, "is_earnable": true},
        ]});
        let badges = parse(&answer).unwrap();
        assert_eq!(
            badges.iter().map(|badge| badge.id).collect::<Vec<_>>(),
            [22, 1, 18]
        );
        assert_eq!(badges[1].info, "Subscriber since 3/22/25");
        assert_eq!(
            badges[1].icon.as_deref(),
            Some("https://cdn.discordapp.com/assets/content/n.png")
        );
        assert!(badges[2].hidden && badges[2].owned);
        assert!(!badges[0].owned);
    }

    #[test]
    fn saves_the_order_and_the_hidden_ones() {
        assert_eq!(
            settings(&[1, 18, 12], &[1, 12]),
            json!({"display_order": [1, 18, 12], "hidden_badges": [1, 12]})
        );
    }
}
