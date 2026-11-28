//! A Discord server's AutoMod rules, as AutoMod in its settings fetches
//! and changes them: `GET /guilds/{id}/auto-moderation/rules`, read into
//! [`AutomodRule`]s, and the bodies a new or changed rule is sent with. See
//! "AutoMod" in docs/discord/server-settings.md.

use serde_json::{Map, Value, json};

use super::{commands::NOT_DISCORD, *};
use crate::chat::{AutomodAction, AutomodAsk, AutomodDid, AutomodKind, AutomodRule};

/// Makes, changes or deletes a rule, or gives back any other command.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    let Command::Discord(DiscordCommand::Automod { server, ask }) = command else {
        return Some(command);
    };
    let did = match &ask {
        AutomodAsk::Create(_) => AutomodDid::Created,
        AutomodAsk::Edit(rule) => AutomodDid::Edited(rule.id.clone()),
        AutomodAsk::Enable { rule, .. } => AutomodDid::Enabled(rule.clone()),
        AutomodAsk::Delete(rule) => AutomodDid::Deleted(rule.clone()),
    };
    let (Ok(discord), Some(guild)) = (discord, discord::parse_server(&server)) else {
        let why = discord.err().unwrap_or(NOT_DISCORD);
        let _ = events.send(Event::AutomodDone {
            server,
            did,
            result: Err(why.into()),
        });
        return None;
    };
    let (discord, guild, events) = (discord.clone(), guild.to_owned(), events.clone());
    tokio::spawn(async move {
        let result = match ask {
            AutomodAsk::Create(rule) => {
                let body = body(&rule, true);
                discord
                    .stepping_up(|| discord.create_automod_rule(&guild, &body))
                    .await
                    .map(Some)
            }
            AutomodAsk::Edit(rule) => {
                let body = body(&rule, false);
                discord
                    .stepping_up(|| discord.edit_automod_rule(&guild, &rule.id, &body))
                    .await
                    .map(Some)
            }
            AutomodAsk::Enable { rule, enabled } => {
                let body = json!({ "enabled": enabled });
                discord
                    .stepping_up(|| discord.edit_automod_rule(&guild, &rule, &body))
                    .await
                    .map(Some)
            }
            AutomodAsk::Delete(rule) => discord
                .stepping_up(|| discord.delete_automod_rule(&guild, &rule))
                .await
                .map(|()| None),
        };
        let result = result
            .map(|rule| rule.as_ref().and_then(parse_rule).map(Box::new))
            .map_err(|error| {
                tracing::info!(
                    error = format!("{error:#}"),
                    "Discord refused an AutoMod change"
                );
                let what = match &did {
                    AutomodDid::Created | AutomodDid::Edited(_) => "Couldn't save the rule",
                    AutomodDid::Enabled(_) => "Couldn't turn the rule on or off",
                    AutomodDid::Deleted(_) => "Couldn't delete the rule",
                };
                format!("{what}: {error}")
            });
        let _ = events.send(Event::AutomodDone {
            server,
            did,
            result,
        });
    });
    None
}

/// The rules in Discord's answer, leaving out any it sent without an id.
pub(super) fn parse(rules: &Value) -> Vec<AutomodRule> {
    rules
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_rule)
        .collect()
}

/// A rule as Discord sends it.
pub(super) fn parse_rule(rule: &Value) -> Option<AutomodRule> {
    let id = rule["id"].as_str()?.to_owned();
    let metadata = &rule["trigger_metadata"];
    let texts = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|text| text.as_str().map(str::to_owned))
            .collect()
    };
    let small = |value: &Value| value.as_u64().and_then(|n| u8::try_from(n).ok());
    let actions = rule["actions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|action| {
            let kind = small(&action["type"])?;
            let data = &action["metadata"];
            Some(match kind {
                1 => AutomodAction::Block {
                    message: data["custom_message"]
                        .as_str()
                        .filter(|message| !message.is_empty())
                        .map(str::to_owned),
                },
                2 => AutomodAction::Alert {
                    channel: data["channel_id"].as_str()?.to_owned(),
                },
                3 => AutomodAction::Timeout {
                    seconds: data["duration_seconds"]
                        .as_u64()
                        .and_then(|seconds| u32::try_from(seconds).ok())?,
                },
                4 => AutomodAction::BlockInteraction,
                _ => AutomodAction::Other {
                    kind,
                    metadata: data.clone(),
                },
            })
        })
        .collect();
    Some(AutomodRule {
        id,
        name: rule["name"].as_str().unwrap_or_default().to_owned(),
        kind: AutomodKind::from_trigger(small(&rule["trigger_type"]).unwrap_or(0)),
        enabled: rule["enabled"].as_bool().unwrap_or(false),
        keywords: texts(&metadata["keyword_filter"]),
        regex: texts(&metadata["regex_patterns"]),
        allowed: texts(&metadata["allow_list"]),
        presets: metadata["presets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(small)
            .collect(),
        mention_limit: small(&metadata["mention_total_limit"]).unwrap_or(0),
        raid_protection: metadata["mention_raid_protection_enabled"]
            .as_bool()
            .unwrap_or(false),
        actions,
        exempt_roles: texts(&rule["exempt_roles"]),
        exempt_channels: texts(&rule["exempt_channels"]),
    })
}

/// The body a rule is made (`create`) or changed with: what its kind
/// has, its actions and exemptions; a change leaves out the trigger type,
/// which Discord doesn't change.
pub(super) fn body(rule: &AutomodRule, create: bool) -> Value {
    let kind = rule.kind;
    let mut metadata = Map::new();
    match kind {
        AutomodKind::Keyword | AutomodKind::MemberProfile => {
            metadata.insert("keyword_filter".into(), json!(rule.keywords));
            metadata.insert("regex_patterns".into(), json!(rule.regex));
            metadata.insert("allow_list".into(), json!(rule.allowed));
        }
        AutomodKind::Preset => {
            metadata.insert("presets".into(), json!(rule.presets));
            metadata.insert("allow_list".into(), json!(rule.allowed));
        }
        AutomodKind::MentionSpam => {
            metadata.insert("mention_total_limit".into(), json!(rule.mention_limit));
            metadata.insert(
                "mention_raid_protection_enabled".into(),
                json!(rule.raid_protection),
            );
        }
        AutomodKind::Spam | AutomodKind::Other(_) => {}
    }
    let actions: Vec<Value> = rule
        .actions
        .iter()
        .map(|action| match action {
            AutomodAction::Block { message } => {
                let mut data = Map::new();
                if let Some(message) = message {
                    data.insert("custom_message".into(), json!(message));
                }
                json!({ "type": 1, "metadata": data })
            }
            AutomodAction::Alert { channel } => {
                json!({ "type": 2, "metadata": { "channel_id": channel } })
            }
            AutomodAction::Timeout { seconds } => {
                json!({ "type": 3, "metadata": { "duration_seconds": seconds } })
            }
            AutomodAction::BlockInteraction => json!({ "type": 4, "metadata": {} }),
            AutomodAction::Other { kind, metadata } => {
                json!({ "type": kind, "metadata": metadata })
            }
        })
        .collect();
    let mut body = json!({
        "name": rule.name,
        "event_type": kind.event(),
        "trigger_metadata": metadata,
        "actions": actions,
        "enabled": rule.enabled,
        "exempt_roles": rule.exempt_roles,
    });
    // A member's profile isn't in a channel.
    if kind != AutomodKind::MemberProfile {
        body["exempt_channels"] = json!(rule.exempt_channels);
    }
    if create {
        body["trigger_type"] = json!(kind.trigger());
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: AutomodKind) -> AutomodRule {
        AutomodRule {
            id: "7".into(),
            name: "Rule".into(),
            kind,
            enabled: true,
            keywords: vec!["bad".into()],
            regex: vec!["b[a4]d".into()],
            allowed: vec!["badge".into()],
            presets: vec![1, 3],
            mention_limit: 12,
            raid_protection: true,
            actions: vec![AutomodAction::Alert {
                channel: "30".into(),
            }],
            exempt_roles: vec!["20".into()],
            exempt_channels: vec!["31".into()],
        }
    }

    #[test]
    fn reads_rules_as_discord_sends_them() {
        let rules = parse(&json!([{
            "id": "900", "guild_id": "10", "name": "Words", "creator_id": "1",
            "event_type": 1, "trigger_type": 1, "enabled": true,
            "trigger_metadata": {"keyword_filter": ["bad", "*worse*"],
                                 "regex_patterns": ["b[a4]d"], "allow_list": ["badge"]},
            "actions": [
                {"type": 1, "metadata": {"custom_message": "Not here."}},
                {"type": 2, "metadata": {"channel_id": "30"}},
                {"type": 3, "metadata": {"duration_seconds": 600}},
                {"type": 9, "metadata": {"x": 1}},
                {"type": 2, "metadata": {}},
            ],
            "exempt_roles": ["20"], "exempt_channels": [],
        }, {
            "id": "901", "name": "Mentions", "event_type": 1, "trigger_type": 5,
            "enabled": false,
            "trigger_metadata": {"mention_total_limit": 12, "mention_raid_protection_enabled": true},
            "actions": [{"type": 1, "metadata": {}}],
        }, {
            "id": "902", "name": "Lists", "trigger_type": 4,
            "trigger_metadata": {"presets": [1, 3], "allow_list": []}, "actions": [],
        }, {"id": "903", "name": "Policy", "trigger_type": 7}, {"name": "no id"}]));
        assert_eq!(rules.len(), 4);
        let words = &rules[0];
        assert_eq!(words.kind, AutomodKind::Keyword);
        assert_eq!(words.keywords, ["bad", "*worse*"]);
        assert_eq!(words.regex, ["b[a4]d"]);
        assert_eq!(words.allowed, ["badge"]);
        assert_eq!(words.exempt_roles, ["20"]);
        assert_eq!(
            words.actions,
            [
                AutomodAction::Block {
                    message: Some("Not here.".into())
                },
                AutomodAction::Alert {
                    channel: "30".into()
                },
                AutomodAction::Timeout { seconds: 600 },
                AutomodAction::Other {
                    kind: 9,
                    metadata: json!({"x": 1})
                },
            ],
            "an alert without its channel is left out"
        );
        let mentions = &rules[1];
        assert_eq!(mentions.kind, AutomodKind::MentionSpam);
        assert!(!mentions.enabled && mentions.raid_protection);
        assert_eq!(mentions.mention_limit, 12);
        assert_eq!(mentions.actions, [AutomodAction::Block { message: None }]);
        assert_eq!(rules[2].presets, [1, 3]);
        assert_eq!(rules[3].kind, AutomodKind::Other(7));
    }

    #[test]
    fn a_rule_is_sent_with_what_its_kind_has() {
        let keyword = body(&rule(AutomodKind::Keyword), true);
        assert_eq!(
            keyword,
            json!({
                "name": "Rule", "event_type": 1, "trigger_type": 1, "enabled": true,
                "trigger_metadata": {"keyword_filter": ["bad"], "regex_patterns": ["b[a4]d"],
                                     "allow_list": ["badge"]},
                "actions": [{"type": 2, "metadata": {"channel_id": "30"}}],
                "exempt_roles": ["20"], "exempt_channels": ["31"],
            })
        );
        // A change leaves the trigger type out.
        assert!(
            body(&rule(AutomodKind::Keyword), false)
                .get("trigger_type")
                .is_none()
        );
        assert_eq!(
            body(&rule(AutomodKind::Preset), true)["trigger_metadata"],
            json!({"presets": [1, 3], "allow_list": ["badge"]})
        );
        assert_eq!(
            body(&rule(AutomodKind::MentionSpam), true)["trigger_metadata"],
            json!({"mention_total_limit": 12, "mention_raid_protection_enabled": true})
        );
        assert_eq!(
            body(&rule(AutomodKind::Spam), true)["trigger_metadata"],
            json!({})
        );
        let mut profile = rule(AutomodKind::MemberProfile);
        profile.actions = vec![AutomodAction::BlockInteraction];
        let profile = body(&profile, true);
        assert_eq!(
            (
                profile["event_type"].clone(),
                profile["trigger_type"].clone()
            ),
            (json!(2), json!(6))
        );
        assert_eq!(profile["actions"], json!([{"type": 4, "metadata": {}}]));
        assert!(
            profile.get("exempt_channels").is_none(),
            "no channels for a profile"
        );
        let mut block = rule(AutomodKind::Spam);
        block.actions = vec![
            AutomodAction::Block { message: None },
            AutomodAction::Block {
                message: Some("No.".into()),
            },
            AutomodAction::Timeout { seconds: 60 },
        ];
        assert_eq!(
            body(&block, true)["actions"],
            json!([
                {"type": 1, "metadata": {}},
                {"type": 1, "metadata": {"custom_message": "No."}},
                {"type": 3, "metadata": {"duration_seconds": 60}},
            ])
        );
        // What Discord answers reads back as the rule sent.
        let mut keyword = rule(AutomodKind::Keyword);
        keyword.presets.clear();
        keyword.mention_limit = 0;
        keyword.raid_protection = false;
        let mut sent = body(&keyword, true);
        sent["id"] = json!("7");
        assert_eq!(parse_rule(&sent), Some(keyword));
    }
}
