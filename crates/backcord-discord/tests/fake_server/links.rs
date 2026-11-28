//! Invites, gifts, events and authorizations.

use super::*;

/// Joins "Server" again by an invite to #general, as Join a Server does
/// with a vanity code: Discord asks for a captcha first, which a page
/// shows, and the server comes back with its GUILD_CREATE.
async fn joins_by_an_invite_as_discords_client_does(
    discord: &Discord,
    server: &str,
    general: &str,
) {
    use backcord_discord::invite::JoinFrom;

    let unknown = discord.invite("gone", None).await.unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "This invite is invalid or has expired."
    );
    let preview = discord.invite("server", Some("server")).await.unwrap();
    assert_eq!(
        (preview.name.as_str(), preview.channel.as_str()),
        ("Server", general)
    );
    assert!(preview.members.is_some());

    post_control(server, "/_control/invite-captcha", "");
    let mut pages = Vec::new();
    let joined = discord
        .join_invite(&preview, JoinFrom::Dialog, |page| {
            pages.push(page.to_owned());
            let page = page.to_owned();
            tokio::spawn(async move {
                reqwest::Client::new()
                    .post(page)
                    .body("solved")
                    .send()
                    .await
            });
        })
        .await
        .unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(
        (joined.guild.as_str(), joined.channel.as_str()),
        (preview.guild.as_str(), general)
    );
    assert!(!joined.verification_form);
    let joins: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/invite-joins", "")).unwrap();
    let join = joins.as_array().unwrap().last().unwrap();
    assert_eq!(join["code"], "server");
    assert_eq!(
        join["context"],
        serde_json::json!({
            "location": "Join Guild",
            "location_guild_id": preview.guild,
            "location_channel_id": general,
            "location_channel_type": 0,
        })
    );

    // A server with a form to fill in says so; she's in it already, which
    // joins nothing more.
    post_control(server, "/_control/invite-form", "");
    let joined = discord
        .join_invite(&preview, JoinFrom::Link, |_| panic!("no captcha"))
        .await
        .unwrap();
    assert!(joined.verification_form);
    let joins: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/invite-joins", "")).unwrap();
    assert_eq!(
        joins.as_array().unwrap().last().unwrap()["context"]["location"],
        "Desktop Invite Modal"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn authorizes_an_application() {
    let Fixture {
        _fake,
        discord,
        server,
        guild,
        ..
    } = Fixture::start("authorizes_an_application").await;
    use backcord_discord::oauth::Request;

    let query = "client_id=157730590492196864&scope=bot%20applications.commands&permissions=2048";
    let request = Request::parse(query).unwrap();
    // The preview its dialog shows. Looking it up authorizes nothing.
    let preview = discord.oauth_preview(&request).await.unwrap();
    assert_eq!(preview.name, "Alien App");
    assert_eq!(preview.bot.as_deref(), Some("Alien App"));
    assert!(!preview.authorized);
    let addable: Vec<&str> = preview
        .guilds
        .iter()
        .filter(|found| found.can_add)
        .map(|found| found.id.as_str())
        .collect();
    assert_eq!(addable, [guild.as_str()]);
    let made: serde_json::Value = serde_json::from_str(&control(
        server,
        "GET",
        "/_control/oauth-authorizations",
        "",
    ))
    .unwrap();
    assert!(made.as_array().unwrap().is_empty(), "{made}");

    // Authorizing adds it to Server with the bot permissions it asked
    // for, and Discord sends her nowhere else.
    let next = discord
        .authorize_oauth(&request, Some(guild.as_str()))
        .await
        .unwrap();
    assert_eq!(next, "https://discord.com/oauth2/authorized");
    let made: serde_json::Value = serde_json::from_str(&control(
        server,
        "GET",
        "/_control/oauth-authorizations",
        "",
    ))
    .unwrap();
    let made = made.as_array().unwrap();
    assert_eq!(made.len(), 1);
    assert_eq!(made[0]["guild_id"], guild.as_str());
    assert_eq!(made[0]["permissions"], "2048");
    assert_eq!(
        made[0]["query"],
        "client_id=157730590492196864&scope=bot+applications.commands"
    );
    assert!(discord.oauth_preview(&request).await.unwrap().authorized);

    // A sign-in flow answers with the app's own address and its code.
    let sign_in = Request::parse(
        "response_type=code&client_id=157730590492196864&scope=identify&state=xyz&redirect_uri=https%3A%2F%2Fnicememe.website",
    )
    .unwrap();
    let next = discord.authorize_oauth(&sign_in, None).await.unwrap();
    assert_eq!(
        next,
        "https://nicememe.website?code=fake-auth-code&state=xyz"
    );
    // What it sends is what the link asked for, rebuilt from what the
    // dialog read, without the fields it doesn't read.
    let tracked = Request::parse(
        "client_id=157730590492196864&scope=identify%20guilds&utm_source=chat&prompt=none&state=a%20b",
    )
    .unwrap();
    discord.authorize_oauth(&tracked, None).await.unwrap();
    let made: serde_json::Value = serde_json::from_str(&control(
        server,
        "GET",
        "/_control/oauth-authorizations",
        "",
    ))
    .unwrap();
    let last = &made.as_array().unwrap().last().unwrap();
    assert_eq!(
        last["query"],
        "client_id=157730590492196864&scope=identify+guilds&state=a+b"
    );
    assert_eq!(last["scope"], "identify guilds");

    // An application Discord doesn't know is refused as it refuses it.
    let unknown = Request::parse("client_id=999999999999999999&scope=bot").unwrap();
    let error = discord.oauth_preview(&unknown).await.unwrap_err();
    assert_eq!(error.to_string(), "Discord doesn't know that application.");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn redeems_a_gift() {
    let Fixture {
        _fake,
        discord,
        server,
        general,
        ..
    } = Fixture::start("redeems_a_gift").await;

    let unknown = discord.gift("gone").await.unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "This gift code is invalid or has expired."
    );

    // Bob's gift of Nitro, as a clicked gift link looks it up. Looking it
    // up redeems nothing.
    let preview = discord.gift("2CG6SV9QtRxerJTgCYNDnU7M").await.unwrap();
    assert_eq!(preview.name, "Nitro");
    assert_eq!(preview.plan.as_deref(), Some("Nitro Monthly"));
    assert!(preview.gifter.is_some());
    assert!(preview.redeemable());
    let redeems: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/gift-redeems", "")).unwrap();
    assert!(redeems.as_array().unwrap().is_empty(), "{redeems}");

    // Redeeming names the channel its link was opened in, once.
    discord
        .redeem_gift(&preview.code, Some(general.as_str()))
        .await
        .unwrap();
    let redeems: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/gift-redeems", "")).unwrap();
    let redeems = redeems.as_array().unwrap();
    assert_eq!(redeems.len(), 1);
    assert_eq!(redeems[0]["code"], "2CG6SV9QtRxerJTgCYNDnU7M");
    assert_eq!(redeems[0]["channel_id"], general.as_str());
    assert!(
        redeems[0]["gateway_checkout_context"].is_null(),
        "{redeems:?}"
    );

    // Afterwards the gift shows as redeemed, and redeeming again is
    // refused as Discord refuses it.
    let preview = discord.gift("2CG6SV9QtRxerJTgCYNDnU7M").await.unwrap();
    assert!(preview.redeemed && !preview.redeemable());
    let again = discord
        .redeem_gift(&preview.code, Some(general.as_str()))
        .await
        .unwrap_err();
    assert_eq!(again.to_string(), "This gift has already been redeemed.");

    // A gift that needs a payment source shows, but only Discord can
    // redeem it.
    let paid = discord.gift("AbCdEfGhIjKlMnOp").await.unwrap();
    assert!(paid.in_discord && !paid.redeemable());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn marks_interest_in_a_scheduled_event() {
    let Fixture {
        _fake,
        discord,
        server,
        guild,
        lounge,
        ..
    } = Fixture::start("marks_interest_in_a_scheduled_event").await;

    let unknown = discord
        .scheduled_event(&guild, "999999999999999999")
        .await
        .unwrap_err();
    assert_eq!(unknown.to_string(), "This event doesn't exist.");

    // The event in the Lounge, as a clicked event link looks it up.
    // Looking it up marks nothing.
    let preview = discord
        .scheduled_event(&guild, "1400000000000000901")
        .await
        .unwrap();
    assert_eq!(preview.name, "Alien meetup");
    assert_eq!(preview.channel.as_deref(), Some(lounge.as_str()));
    assert_eq!(preview.interested_count, Some(7));
    assert!(!preview.interested && !preview.over);
    let changes: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/event-interest", "")).unwrap();
    assert!(changes.as_array().unwrap().is_empty(), "{changes}");

    discord
        .set_event_interest(&guild, &preview.id, true)
        .await
        .unwrap();
    let preview = discord
        .scheduled_event(&guild, "1400000000000000901")
        .await
        .unwrap();
    assert!(preview.interested);
    assert_eq!(preview.interested_count, Some(8));

    discord
        .set_event_interest(&guild, &preview.id, false)
        .await
        .unwrap();
    let preview = discord
        .scheduled_event(&guild, "1400000000000000901")
        .await
        .unwrap();
    assert!(!preview.interested);
    assert_eq!(preview.interested_count, Some(7));
    let changes: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/event-interest", "")).unwrap();
    let changes = changes.as_array().unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0]["interested"], true);
    assert_eq!(
        changes[0]["response"], 1,
        "as Discord's Interested button sends"
    );
    assert_eq!(changes[1]["interested"], false);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn lists_makes_and_changes_scheduled_events() {
    use backcord_discord::{ready::EVENT_EXTERNAL, scheduled_event::EventForm};

    let Fixture {
        _fake,
        discord,
        server,
        guild,
        lounge,
        ..
    } = Fixture::start("lists_makes_and_changes_scheduled_events").await;
    let events_of = |snapshot: &Snapshot| {
        snapshot
            .servers
            .iter()
            .find(|known| known.id == guild)
            .map(|known| known.events.clone())
            .unwrap_or_default()
    };

    // The list, as the Events dialog asks for it, gives the counts.
    discord.load_events(&guild).await.unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .first()
            .is_some_and(|event| event.interested == 7)
    })
    .await;
    let listed = events_of(&snapshot);
    assert_eq!(listed[0].channel.as_deref(), Some(lounge.as_str()));
    assert_eq!(listed[0].creator.as_deref(), Some("Bob"));
    assert!(!listed[0].mine);
    let interested = discord
        .event_users(&guild, "1400000000000000901")
        .await
        .unwrap();
    assert_eq!(interested.len(), 1);

    // Being interested comes back as USER_ADD.
    discord
        .set_event_interest(&guild, "1400000000000000901", true)
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .first()
            .is_some_and(|event| event.mine && event.interested == 8)
    })
    .await;

    // An event somewhere else, made as the form sends it.
    let form = EventForm {
        kind: EVENT_EXTERNAL,
        location: Some("The park".into()),
        name: "Picnic".into(),
        description: "Bring food".into(),
        start: "2030-06-01T10:00:00Z".into(),
        end: Some("2030-06-01T12:00:00Z".into()),
        image: Some("data:image/png;base64,iVBORw0KGgo=".into()),
        ..EventForm::default()
    };
    discord
        .save_event(&guild, None, &form.body())
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .iter()
            .any(|event| event.name == "Picnic" && event.mine)
    })
    .await;
    let picnic = events_of(&snapshot)
        .iter()
        .find(|event| event.name == "Picnic")
        .cloned()
        .unwrap();
    assert_eq!(picnic.location.as_deref(), Some("The park"));
    assert!(picnic.cover.is_some());
    assert_eq!(
        picnic.interested, 1,
        "Discord subscribes the one who made it"
    );

    // Renamed, started, then ended, which takes it off the list.
    let renamed = EventForm {
        name: "Picnic!".into(),
        image: None,
        ..form
    };
    discord
        .save_event(&guild, Some(&picnic.id), &renamed.body())
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .iter()
            .any(|event| event.name == "Picnic!" && event.interested == 1)
    })
    .await;
    let status = |status: u8| serde_json::json!({ "status": status });
    discord
        .save_event(&guild, Some(&picnic.id), &status(3))
        .await
        .expect_err("a scheduled event can't end before it starts");
    discord
        .save_event(&guild, Some(&picnic.id), &status(2))
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .iter()
            .any(|event| event.id == picnic.id && event.status == 2)
    })
    .await;
    discord
        .save_event(&guild, Some(&picnic.id), &status(3))
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        !events_of(snapshot)
            .iter()
            .any(|event| event.id == picnic.id)
    })
    .await;

    // Another, in the Lounge, deleted.
    let voice = EventForm {
        kind: backcord_discord::ready::EVENT_VOICE,
        channel: Some(lounge.clone()),
        name: "Movie".into(),
        start: "2030-06-02T20:00:00Z".into(),
        ..EventForm::default()
    };
    discord
        .save_event(&guild, None, &voice.body())
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        events_of(snapshot)
            .iter()
            .any(|event| event.name == "Movie")
    })
    .await;
    let movie = events_of(&snapshot)
        .iter()
        .find(|event| event.name == "Movie")
        .cloned()
        .unwrap();
    discord.delete_event(&guild, &movie.id).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        !events_of(snapshot).iter().any(|event| event.id == movie.id)
    })
    .await;

    let writes: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/event-writes", "")).unwrap();
    let writes = writes.as_array().unwrap();
    let first = &writes[0]["body"];
    assert_eq!(writes[0]["method"], "POST");
    assert_eq!(
        (
            first["entity_type"].as_u64(),
            first["privacy_level"].as_u64()
        ),
        (Some(3), Some(2))
    );
    assert_eq!(first["entity_metadata"]["location"], "The park");
    assert!(
        first["image"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    assert!(
        writes[1]["body"].get("image").is_none(),
        "an edit keeps the cover"
    );
    assert_eq!(writes.last().unwrap()["method"], "DELETE");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn makes_servers_from_nothing_and_from_a_template() {
    let Fixture {
        _fake,
        discord,
        server,
        ..
    } = Fixture::start("makes_servers_from_nothing_and_from_a_template").await;

    let made = discord
        .create_server(
            "Alice's server",
            Some("data:image/png;base64,iVBORw0KGgo="),
            None,
        )
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.servers.iter().any(|known| known.id == made)
    })
    .await;
    let listed = snapshot
        .servers
        .iter()
        .find(|known| known.id == made)
        .unwrap();
    assert_eq!(listed.name, "Alice's server");
    assert!(
        discord.create_server("A", None, None).await.is_err(),
        "too short"
    );

    let unknown = discord.guild_template("nope").await;
    assert!(unknown.is_err());
    let template = discord.guild_template("hgM48av5Q69A").await.unwrap();
    assert_eq!(template.name, "Friends & Family");
    assert!(
        template
            .channels
            .iter()
            .any(|(kind, name)| *kind == 2 && name == "Lounge")
    );
    let from_template = discord
        .create_server("Family", None, Some(&template.code))
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        snapshot
            .servers
            .iter()
            .any(|known| known.id == from_template)
    })
    .await;

    let bodies: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/servers-made", "")).unwrap();
    let bodies = bodies.as_array().unwrap();
    assert_eq!(bodies.len(), 2);
    let first = &bodies[0]["body"];
    assert_eq!(first["guild_template_code"], "2TffvPucqHkN");
    assert_eq!(first["channels"], serde_json::json!([]));
    assert!(
        first["icon"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png")
    );
    assert_eq!(bodies[1]["template"], "hgM48av5Q69A");
    assert_eq!(bodies[1]["body"]["name"], "Family");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn makes_invites() {
    let Fixture {
        _fake,
        discord,
        server,
        general,
        ..
    } = Fixture::start("makes_invites").await;
    let carol_id = discord.find_people("carol")[0].id.clone();
    let carol_dm = discord.open_dm(&carol_id).await.unwrap();
    // An invite to #general, as Discord's invite dialog asks for one.
    let code = discord.create_invite(&general, 3600, 5).await.unwrap();
    assert_eq!(code.len(), 8);
    let invites: serde_json::Value =
        serde_json::from_str(&self::control(server, "GET", "/_control/invites", "")).unwrap();
    let made = invites
        .as_array()
        .unwrap()
        .iter()
        .find(|invite| invite["code"] == code.as_str())
        .unwrap();
    assert_eq!(
        (made["max_age"].as_u64(), made["max_uses"].as_u64()),
        (Some(3600), Some(5))
    );
    assert_eq!(made["channel"]["id"], general.as_str());
    assert!(discord.create_invite(&general, 3600, 101).await.is_err());

    // Invite to Server, from Carol's menu: an invite that works once and
    // lasts Discord's default day, its link sent in her DM.
    discord.send_invite(&carol_id, &general).await.unwrap();
    let invites: serde_json::Value =
        serde_json::from_str(&self::control(server, "GET", "/_control/invites", "")).unwrap();
    let sent = invites.as_array().unwrap().last().unwrap();
    assert_eq!(
        (sent["max_age"].as_u64(), sent["max_uses"].as_u64()),
        (Some(86_400), Some(1))
    );
    assert_eq!(sent["location"], "User Invite Context Menu");
    let in_dm = discord
        .history(PRIVATE, &carol_dm, Span::Latest)
        .await
        .unwrap()
        .messages;
    assert_eq!(
        in_dm.last().map(|message| message.content.as_str()),
        Some(format!("https://discord.gg/{}", sent["code"].as_str().unwrap()).as_str())
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn leaves_and_joins_a_server() {
    let Fixture {
        _fake,
        discord,
        server,
        guild,
        general,
        ..
    } = Fixture::start("leaves_and_joins_a_server").await;
    // Leaving "Server", which Bob owns, takes it out of her list; the fake
    // lets her back in, so the tests after these find it again.
    discord.leave_server(&guild).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        !snapshot.servers.iter().any(|known| known.id == guild)
    })
    .await;
    assert!(
        discord.leave_server(&guild).await.is_err(),
        "she isn't in it"
    );
    joins_by_an_invite_as_discords_client_does(&discord, server, &general).await;
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.servers.iter().any(|known| known.id == guild)
    })
    .await;
    assert!(
        snapshot.servers[0]
            .text_channels
            .iter()
            .any(|channel| channel.id == general)
    );
}
