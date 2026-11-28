//! Making a server, as Discord's "Create a server" does it: from nothing,
//! with `POST /guilds` and the default template's code, or from a server
//! template, looked up with `GET /guilds/templates/{code}` and used with
//! `POST /guilds/templates/{code}`; see
//! https://docs.discord.food/resources/guild#create-guild and
//! https://docs.discord.food/resources/guild-template.

use serde_json::{Value, json};

/// The template Discord's client names for a server made from nothing,
/// whose defaults `POST /guilds` takes for what it isn't given.
pub const DEFAULT_TEMPLATE: &str = "2TffvPucqHkN";

/// Discord's own templates, by their names and codes, as the list of
/// official templates docs.discord.food links to has them
/// (gist.github.com/dolfies/f70b1de53a1d185d58563b9ca8bb248d), verified
/// ones only.
pub const BUILT_IN: [(&str, &str); 6] = [
    ("Friends & Family", "hgM48av5Q69A"),
    ("Study Group", "FbwUwRp4j8Es"),
    ("Campus Clubs & Organizations", "Ctg7PUHcQmZu"),
    ("Local Communities", "64UDvRNCC52Y"),
    ("Creators & Hobbies", "6exdzMgjZgah"),
    ("Global Communities", "4sgbPdCjzAYU"),
];

/// A server template, as its preview shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Template {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    /// How many servers were made from it.
    pub usage_count: u64,
    /// Its channels in its order, each with its type (4 a category, 2 a
    /// voice channel) and name.
    pub channels: Vec<(u8, String)>,
    /// Its roles' names, @everyone left out.
    pub roles: Vec<String>,
}

impl Template {
    /// A guild template object.
    pub fn parse(value: &Value) -> Option<Self> {
        let source = &value["serialized_source_guild"];
        Some(Self {
            code: value["code"].as_str()?.to_owned(),
            name: value["name"].as_str().unwrap_or_default().to_owned(),
            description: value["description"]
                .as_str()
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
            usage_count: value["usage_count"].as_u64().unwrap_or_default(),
            channels: source["channels"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|channel| {
                    let kind = u8::try_from(channel["type"].as_u64()?).ok()?;
                    Some((kind, channel["name"].as_str()?.to_owned()))
                })
                .collect(),
            roles: source["roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|role| role["name"].as_str())
                .filter(|name| *name != "@everyone")
                .map(str::to_owned)
                .collect(),
        })
    }
}

/// The body of `POST /guilds` for a server made from nothing, as
/// Discord's client sends it: no channels of its own, so the default
/// template's come, and its code.
pub fn create_body(name: &str, icon: Option<&str>) -> Value {
    json!({
        "name": name.trim(),
        "icon": icon,
        "channels": [],
        "system_channel_id": null,
        "guild_template_code": DEFAULT_TEMPLATE,
    })
}

/// The body of `POST /guilds/templates/{code}`.
pub fn template_body(name: &str, icon: Option<&str>) -> Value {
    json!({ "name": name.trim(), "icon": icon })
}

/// A template's code from what was typed or clicked: a `discord.new/{code}`
/// link, a `discord.com/template/{code}` one, or the bare code.
pub fn code_of(typed: &str) -> Option<String> {
    let typed = typed.trim();
    if let Some(crate::link::Route::Template { code }) = crate::link::parse(typed) {
        return Some(code);
    }
    let bare = typed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("discord.new/");
    code(bare).then(|| bare.to_owned())
}

/// Whether `code` could be a template's: letters and digits only.
pub(crate) fn code(code: &str) -> bool {
    (2..=32).contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_templates_preview() {
        let template = Template::parse(&json!({
            "code": "abc", "name": "Gaming", "description": "", "usage_count": 12,
            "serialized_source_guild": {
                "roles": [{"id": 0, "name": "@everyone"}, {"id": 1, "name": "Mods"}],
                "channels": [
                    {"id": 1, "type": 4, "name": "Text Channels"},
                    {"id": 2, "type": 0, "name": "general", "parent_id": 1},
                    {"id": 3, "type": 2, "name": "Lounge"},
                ],
            },
        }))
        .unwrap();
        assert_eq!(template.description, None);
        assert_eq!(template.usage_count, 12);
        assert_eq!(template.channels[1], (0, "general".to_owned()));
        assert_eq!(template.roles, ["Mods"]);
    }

    #[test]
    fn makes_a_server_as_discords_client_does() {
        let body = create_body(" Ann's server ", None);
        assert_eq!(body["name"], "Ann's server");
        assert_eq!(body["guild_template_code"], DEFAULT_TEMPLATE);
        assert_eq!(body["channels"], json!([]));
        assert_eq!(body["system_channel_id"], Value::Null);
        assert_eq!(
            template_body("X", Some("data:,")),
            json!({"name": "X", "icon": "data:,"})
        );
    }

    #[test]
    fn finds_a_templates_code() {
        assert_eq!(
            code_of("https://discord.new/hgM48av5Q69A").as_deref(),
            Some("hgM48av5Q69A")
        );
        assert_eq!(
            code_of("discord.new/hgM48av5Q69A").as_deref(),
            Some("hgM48av5Q69A")
        );
        assert_eq!(
            code_of("https://discord.com/template/hgM48av5Q69A").as_deref(),
            Some("hgM48av5Q69A")
        );
        assert_eq!(code_of(" hgM48av5Q69A ").as_deref(), Some("hgM48av5Q69A"));
        assert_eq!(code_of("not a code"), None);
    }
}
