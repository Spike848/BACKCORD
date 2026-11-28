//! A server's scheduled events: what one shows from its link, and being
//! interested in it. Discord's client resolves an event with
//! `GET /guilds/{guild}/scheduled-events/{event}`, reads this account's
//! subscriptions with `GET /users/@me/scheduled-events`, and subscribes
//! or unsubscribes with `PUT` or `DELETE` on
//! `/guilds/{guild}/scheduled-events/{event}/users/@me`; see
//! https://docs.discord.food/resources/guild-scheduled-event.

use serde_json::Value;

/// An event that hasn't started yet, and one going on now.
const SCHEDULED: u64 = 1;
const ACTIVE: u64 = 2;
/// The response of a subscription the Interested button makes.
pub const INTERESTED: u64 = 1;

/// What a scheduled event shows from its link, from
/// `GET /guilds/{guild}/scheduled-events/{event}` with its user count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub id: String,
    pub guild: String,
    pub name: String,
    /// What the event says about itself, when it says anything.
    pub description: Option<String>,
    /// When it starts and ends, as Discord timestamps them; an event
    /// somewhere else always says when it ends.
    pub starts_at: String,
    pub ends_at: Option<String>,
    /// The voice or stage channel it happens in, when it happens in one.
    pub channel: Option<String>,
    /// Where it happens, when it happens somewhere else.
    pub location: Option<String>,
    /// Who made the event, by the name Discord shows for them, when it
    /// says.
    pub creator: Option<String>,
    /// How many people are interested, when Discord counts them.
    pub interested_count: Option<u64>,
    /// The event is going on now.
    pub active: bool,
    /// The event has ended or was canceled, so interest can't change.
    pub over: bool,
    /// This account is interested in the event.
    pub interested: bool,
}

impl Preview {
    /// The preview of a scheduled event object, with whether this
    /// account is interested in it.
    pub fn of(event: &Value, interested: bool) -> Option<Self> {
        let status = event["status"].as_u64().unwrap_or_default();
        let creator = &event["creator"];
        Some(Self {
            id: event["id"].as_str()?.to_owned(),
            guild: event["guild_id"].as_str()?.to_owned(),
            name: event["name"].as_str().unwrap_or_default().to_owned(),
            description: event["description"].as_str().map(str::to_owned),
            starts_at: event["scheduled_start_time"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            ends_at: event["scheduled_end_time"].as_str().map(str::to_owned),
            channel: event["channel_id"].as_str().map(str::to_owned),
            location: event["entity_metadata"]["location"]
                .as_str()
                .map(str::to_owned),
            creator: creator["global_name"]
                .as_str()
                .or_else(|| creator["username"].as_str())
                .map(str::to_owned),
            interested_count: event["user_count"].as_u64(),
            active: status == ACTIVE,
            over: !matches!(status, SCHEDULED | ACTIVE),
            interested,
        })
    }
}

/// Whether the event `id` is among this account's subscriptions, from
/// `GET /users/@me/scheduled-events`: subscribed with an interested
/// response, as Discord's Interested button leaves it. A subscription to
/// one occurrence of a recurring event, with its exception id, is an
/// answer of its own and doesn't count for the series.
pub fn interested_in(subscriptions: &Value, id: &str) -> bool {
    subscriptions.as_array().is_some_and(|subscriptions| {
        subscriptions.iter().any(|subscription| {
            subscription["guild_scheduled_event_id"].as_str() == Some(id)
                && subscription["guild_scheduled_event_exception_id"].is_null()
                && subscription["response"].as_u64() == Some(INTERESTED)
        })
    })
}

/// A server's events from `GET /guilds/{guild}/scheduled-events`, with
/// whether this account is interested in each from its subscriptions.
pub fn listed(list: &Value, subscriptions: &Value) -> Vec<crate::ready::ScheduledEvent> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(crate::ready::ScheduledEvent::parse)
        .map(|mut event| {
            event.mine = interested_in(subscriptions, &event.id);
            event
        })
        .collect()
}

/// Someone interested in an event, as Who's interested lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventUser {
    pub id: String,
    /// Their name in the server, else their display name or username.
    pub name: String,
    pub avatar: String,
}

/// Those `GET /guilds/{guild}/scheduled-events/{event}/users` lists, which
/// Discord gives as a `users` list of users with their `guild_member`,
/// or as a list of event users with their `user` and `member`.
pub fn users(answer: &Value) -> Vec<EventUser> {
    let rows: Vec<(&Value, &Value)> = match answer["users"].as_array() {
        Some(users) => users
            .iter()
            .map(|user| (user, &user["guild_member"]))
            .collect(),
        None => answer
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| (&row["user"], &row["member"]))
            .collect(),
    };
    rows.into_iter()
        .filter_map(|(user, member)| {
            let id = user["id"].as_str()?;
            let name = member["nick"]
                .as_str()
                .or_else(|| user["global_name"].as_str())
                .or_else(|| user["username"].as_str())
                .unwrap_or("Discord user");
            Some(EventUser {
                id: id.to_owned(),
                name: name.to_owned(),
                avatar: crate::ready::avatar(id, user["avatar"].as_str()),
            })
        })
        .collect()
}

/// What the event form sends: where (`kind`, one of
/// [`crate::ready::EVENT_STAGE`], `EVENT_VOICE` or `EVENT_EXTERNAL`,
/// with its channel or location), what, and when, as ISO 8601 times.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventForm {
    pub kind: u8,
    pub channel: Option<String>,
    pub location: Option<String>,
    pub name: String,
    pub description: String,
    pub start: String,
    pub end: Option<String>,
    /// A new cover as a data URI; none keeps the one it has.
    pub image: Option<String>,
}

impl EventForm {
    /// The body of `POST /guilds/{guild}/scheduled-events`, or of the
    /// `PATCH` that edits one, as Discord's client sends it: a server-only
    /// event (`privacy_level` 2); an event elsewhere with no channel, its
    /// location and an end, one in a channel with no metadata.
    pub fn body(&self) -> Value {
        let external = self.kind == crate::ready::EVENT_EXTERNAL;
        let mut body = serde_json::json!({
            "name": self.name.trim(),
            "privacy_level": 2,
            "scheduled_start_time": self.start,
            "scheduled_end_time": self.end,
            "description": (!self.description.trim().is_empty()).then(|| self.description.trim()),
            "entity_type": self.kind,
            "channel_id": if external { None } else { self.channel.as_deref() },
            "entity_metadata": if external {
                serde_json::json!({"location": self.location.as_deref().unwrap_or_default().trim()})
            } else {
                Value::Null
            },
        });
        if let Some(image) = &self.image {
            body["image"] = Value::String(image.clone());
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn lists_events_with_this_accounts_interest() {
        let list = json!([
            {"id": "1", "name": "A", "scheduled_start_time": "2030-01-02T00:00:00+00:00", "status": 1, "user_count": 3},
            {"id": "2", "name": "B", "scheduled_start_time": "2030-01-01T00:00:00+00:00", "status": 2},
        ]);
        let own = json!([{"guild_scheduled_event_id": "2", "user_id": "9", "response": 1}]);
        let events = listed(&list, &own);
        assert_eq!(events.len(), 2);
        assert!(!events[0].mine && events[1].mine);
        assert_eq!(events[0].interested, 3);
    }

    #[test]
    fn reads_whos_interested_either_way_discord_lists_them() {
        let wrapped = json!({"users": [
            {"id": "5", "username": "ann", "global_name": "Ann", "avatar": null, "guild_member": {"nick": "Annie"}},
            {"id": "6", "username": "bob"},
        ]});
        let names: Vec<_> = users(&wrapped).into_iter().map(|user| user.name).collect();
        assert_eq!(names, ["Annie", "bob"]);
        let rows = json!([{"guild_scheduled_event_id": "1", "user": {"id": "7", "username": "cy"}, "member": null}]);
        assert_eq!(users(&rows)[0].name, "cy");
    }

    #[test]
    fn an_event_form_sends_what_its_place_needs() {
        let mut form = EventForm {
            kind: crate::ready::EVENT_EXTERNAL,
            channel: Some("5".into()),
            location: Some(" Park ".into()),
            name: " Picnic ".into(),
            description: String::new(),
            start: "2030-01-01T10:00:00Z".into(),
            end: Some("2030-01-01T12:00:00Z".into()),
            image: None,
        };
        let body = form.body();
        assert_eq!(body["name"], "Picnic");
        assert_eq!(body["channel_id"], Value::Null);
        assert_eq!(body["entity_metadata"]["location"], "Park");
        assert_eq!(body["description"], Value::Null);
        assert!(body.get("image").is_none());
        form.kind = crate::ready::EVENT_VOICE;
        form.end = None;
        form.image = Some("data:image/png;base64,AA==".into());
        let body = form.body();
        assert_eq!(
            (body["channel_id"].as_str(), body["entity_type"].as_u64()),
            (Some("5"), Some(2))
        );
        assert_eq!(body["entity_metadata"], Value::Null);
        assert_eq!(body["image"], "data:image/png;base64,AA==");
    }

    fn event(status: u64) -> Value {
        json!({
            "id": "1059954443799498922",
            "guild_id": "1046920999469330512",
            "name": "Alien meetup",
            "description": "Aliens only!",
            "channel_id": null,
            "creator": {"id": "10", "username": "dziurwa", "global_name": "Dziurwa"},
            "scheduled_start_time": "2023-12-31T23:00:00+00:00",
            "scheduled_end_time": "2024-01-01T23:00:00+00:00",
            "status": status,
            "entity_type": 3,
            "entity_metadata": {"location": "somewhere in ocean"},
            "user_count": 7,
        })
    }

    #[test]
    fn reads_what_an_event_shows_from_its_link() {
        let preview = Preview::of(&event(1), true).unwrap();
        assert_eq!(preview.name, "Alien meetup");
        assert_eq!(preview.location.as_deref(), Some("somewhere in ocean"));
        assert_eq!(preview.creator.as_deref(), Some("Dziurwa"));
        assert_eq!(preview.interested_count, Some(7));
        assert!(preview.interested && !preview.active && !preview.over);
    }

    #[test]
    fn an_ended_or_canceled_event_is_over() {
        assert!(Preview::of(&event(3), false).unwrap().over);
        assert!(Preview::of(&event(4), false).unwrap().over);
        let live = Preview::of(&event(2), false).unwrap();
        assert!(live.active && !live.over);
    }

    #[test]
    fn interest_reads_from_the_accounts_subscriptions() {
        let subscriptions = json!([
            {"guild_scheduled_event_id": "11", "user_id": "99", "response": 1},
            {"guild_scheduled_event_id": "12", "user_id": "99", "response": 0},
            // Interested in one occurrence of a recurring event only.
            {"guild_scheduled_event_id": "14", "guild_scheduled_event_exception_id": "15",
             "user_id": "99", "response": 1},
        ]);
        assert!(interested_in(&subscriptions, "11"));
        assert!(!interested_in(&subscriptions, "12"));
        assert!(!interested_in(&subscriptions, "14"));
        assert!(!interested_in(&subscriptions, "13"));
        assert!(!interested_in(&json!([]), "11"));
    }
}
