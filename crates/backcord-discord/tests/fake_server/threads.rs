//! Threads and forums.

use super::*;

/// Joining a thread from its menu, then leaving it, as Discord's client
/// does; part of [`talks_to_the_fake_server`], whose session it shares.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn joins_and_leaves_threads_as_discords_client_does() {
    let fixture = Fixture::start("joins_and_leaves_threads_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let book_club = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .threads
            .iter()
            .find(|thread| thread.name == "Book club")
            .and_then(|thread| Some((thread.id.clone(), thread.thread.as_deref()?.joined)))
    };
    // Viewing the server subscribes to its threads.
    discord.view(&fixture.guild, Some(&fixture.general));
    let (id, joined) =
        book_club(&*snapshot_where(discord, |snapshot| book_club(snapshot).is_some()).await)
            .unwrap();
    assert!(!joined);
    discord
        .join_thread(&id, true, "Context Menu")
        .await
        .unwrap();
    snapshot_where(discord, |snapshot| {
        book_club(snapshot).is_some_and(|(_, joined)| joined)
    })
    .await;
    discord
        .join_thread(&id, false, "Toolbar Overflow")
        .await
        .unwrap();
    snapshot_where(discord, |snapshot| {
        book_club(snapshot).is_some_and(|(_, joined)| !joined)
    })
    .await;
    assert_eq!(
        self::control(server, "GET", "/_control/thread-members", ""),
        r#"["POST Context Menu","DELETE Toolbar Overflow"]"#
    );
}

/// Changing this account's notifications in a thread it's in, as Discord's
/// client does from the thread's menu: a level of its own, then a mute.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn changes_a_threads_notifications_as_discords_client_does() {
    use backcord_discord::notifications::{Change, Level};
    let fixture = Fixture::start("changes_a_threads_notifications_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let plans = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .threads
            .iter()
            .find(|thread| thread.name == "Plans")
            .and_then(|thread| Some((thread.id.clone(), thread.thread.as_deref()?.clone())))
    };
    discord.view(&fixture.guild, Some(&fixture.general));
    let (id, thread) =
        plans(&*snapshot_where(discord, |snapshot| plans(snapshot).is_some()).await).unwrap();
    assert!(thread.joined && thread.level.is_none() && !thread.mute.muted);
    discord
        .change_thread_notifications(&id, Change::Level(Some(Level::Nothing)))
        .await
        .unwrap();
    snapshot_where(discord, |snapshot| {
        plans(snapshot).is_some_and(|(_, thread)| thread.level == Some(Level::Nothing))
    })
    .await;
    discord
        .change_thread_notifications(
            &id,
            Change::Mute {
                muted: true,
                seconds: Some(900),
            },
        )
        .await
        .unwrap();
    let snapshot = snapshot_where(discord, |snapshot| {
        plans(snapshot).is_some_and(|(_, thread)| thread.mute.muted)
    })
    .await;
    let (_, thread) = plans(&snapshot).unwrap();
    // The level stays, and the mute ends a quarter of an hour on.
    assert_eq!(thread.level, Some(Level::Nothing));
    assert!(thread.mute.until.is_some());
    // A thread has no default to follow.
    assert!(
        discord
            .change_thread_notifications(&id, Change::Level(None))
            .await
            .is_err()
    );
    let sent: serde_json::Value = serde_json::from_str(&self::control(
        server,
        "GET",
        "/_control/thread-settings",
        "",
    ))
    .unwrap();
    assert_eq!(sent[0]["flags"], 8);
    assert_eq!(sent[1]["muted"], true);
    assert_eq!(sent[1]["mute_config"]["selected_time_window"], 900);
    assert!(sent[1]["mute_config"]["end_time"].is_string());
}

/// A forum's posts and what their cards show, as Discord's client loads
/// them; part of [`talks_to_the_fake_server`], whose session it shares.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn shows_a_forum_as_discords_client_does() {
    let fixture = Fixture::start("shows_a_forum_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let guild = fixture.guild.as_str();
    use backcord_discord::forum::Emoji;
    // Viewing the server subscribes to its threads, and so its forum posts.
    discord.view(guild, Some(&fixture.general));
    // The forum lists among the text channels, and its active posts came
    // with the server's threads.
    let snapshot = snapshot_where(discord, |snapshot| {
        snapshot.servers[0]
            .threads
            .iter()
            .any(|thread| thread.name == "Keyboard shortcut list?")
    })
    .await;
    let listed = &snapshot.servers[0];
    let forum = listed
        .text_channels
        .iter()
        .find(|channel| channel.name == "ideas")
        .expect("the forum lists");
    let details = forum.forum.as_deref().expect("as a forum");
    let tags: Vec<_> = details.tags.iter().map(|tag| tag.name.as_str()).collect();
    assert_eq!(tags, ["Feature", "Bug", "Question", "Done"]);
    assert!(details.tags[3].moderated);
    assert_eq!(details.reaction, Some(Emoji::Unicode("👍".into())));
    assert!(!details.require_tag && !details.by_creation);
    let post = |name: &str| {
        listed
            .threads
            .iter()
            .find(|thread| thread.name == name)
            .unwrap_or_else(|| panic!("{name} lists"))
    };
    let details = |name: &str| post(name).thread.as_deref().unwrap();
    let active: Vec<_> = listed
        .threads
        .iter()
        .filter(|thread| thread.parent.as_deref() == Some(forum.id.as_str()))
        .map(|thread| thread.name.as_str())
        .collect();
    assert_eq!(
        active,
        [
            "Keyboard shortcut list?",
            "Crash when pasting a GIF",
            "Dark theme for the sidebar",
            "Welcome to #ideas"
        ]
    );
    assert!(details("Welcome to #ideas").pinned);
    assert_eq!(
        details("Crash when pasting a GIF").tags,
        [details_tag(forum, 1), details_tag(forum, 2)]
    );
    let opened = |name: &str| {
        listed
            .threads()
            .find(|thread| thread.name == name)
            .is_some_and(|thread| thread.marks.opened)
    };
    let dark = details("Dark theme for the sidebar");
    assert!(dark.joined && opened("Dark theme for the sidebar"));
    // Its first message isn't counted.
    assert_eq!(dark.messages, 2);
    assert!(!opened("Keyboard shortcut list?"));

    // Their first messages, as their cards show them.
    let ids: Vec<&str> = active.iter().map(|name| post(name).id.as_str()).collect();
    let starters = discord.posts(guild, &forum.id, &ids).await.unwrap();
    let starter = |name: &str| {
        let id = &post(name).id;
        starters
            .iter()
            .find(|(post, _)| post == id)
            .and_then(|(_, starter)| starter.clone())
            .unwrap_or_else(|| panic!("{name}'s first message"))
    };
    let dark = starter("Dark theme for the sidebar");
    assert_eq!(dark.content, "The sidebar could follow the system theme.");
    assert_eq!(dark.reaction, Some((Emoji::Unicode("👍".into()), 3)));
    assert!(dark.picture.is_none());
    let crash = starter("Crash when pasting a GIF");
    let picture = crash.picture.expect("its screenshot");
    assert!(
        picture.contains("/crash.png?format=webp&width=144&height=108"),
        "{picture}"
    );
    assert!(!crash.spoiler);
    assert!(starter("Keyboard shortcut list?").reaction.is_none());
    assert_eq!(
        self::control(server, "GET", "/_control/post-data", ""),
        format!("[{}]", serde_json::to_string(&ids).unwrap())
    );

    // Older posts come a page at a time, with their first messages.
    let page = discord.threads(guild, &forum.id, None, 0).await.unwrap();
    let older: Vec<_> = page
        .threads
        .iter()
        .map(|thread| thread.name.as_str())
        .collect();
    assert_eq!(older, ["Old idea 1", "Old idea 2", "Old idea 3"]);
    let first = page
        .starters
        .iter()
        .map(|(_, starter)| starter.as_ref().map(|starter| starter.content.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        first,
        [
            Some("Idea 1, done long ago."),
            Some("Idea 2, done long ago."),
            Some("Idea 3, done long ago.")
        ]
    );

    // Read, the forum has nothing new until someone posts.
    discord.mark_read(guild, &forum.id, true);
    let unread = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .text_channels()
            .any(|channel| channel.name == "ideas" && channel.marks.unread)
    };
    snapshot_where(discord, |snapshot| !unread(snapshot)).await;
    post_control(server, "/_control/forum-post", "");
    let snapshot = snapshot_where(discord, |snapshot| {
        unread(snapshot)
            && snapshot.servers[0]
                .threads
                .iter()
                .any(|thread| thread.name == "Emoji picker search")
    })
    .await;
    let new = snapshot.servers[0]
        .threads
        .iter()
        .find(|thread| thread.name == "Emoji picker search")
        .unwrap();
    assert_eq!(new.thread.as_deref().unwrap().tags, [details_tag(forum, 0)]);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn posts_in_a_forum_as_discords_client_does() {
    let fixture = Fixture::start("posts_in_a_forum_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let guild = fixture.guild.as_str();
    let forum = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .text_channels
            .iter()
            .find(|channel| channel.name == "ideas")
            .cloned()
            .unwrap()
    };
    // A forum that requires a tag refuses a post without one.
    post_control(server, "/_control/forum-require-tag", "");
    let snapshot = snapshot_where(discord, |snapshot| {
        forum(snapshot)
            .forum
            .as_deref()
            .is_some_and(|forum| forum.require_tag)
    })
    .await;
    let ideas = forum(&snapshot);
    let refused = discord
        .post_in_forum(guild, &ideas.id, "Untagged", &[], "Hello", vec![])
        .await;
    assert!(refused.is_err_and(|error| error.to_string().contains("tag is required")));

    // A post with a tag, text and a file lists, its first message with
    // them, as Discord sends it.
    let bug = details_tag(&ideas, 1);
    let post = discord
        .post_in_forum(
            guild,
            &ideas.id,
            "Crash when dragging",
            std::slice::from_ref(&bug),
            "It closes.",
            vec![("notes.txt".into(), b"steps".to_vec())],
        )
        .await
        .unwrap();
    let snapshot = snapshot_where(discord, |snapshot| {
        snapshot.servers[0]
            .threads
            .iter()
            .any(|thread| thread.id == post)
    })
    .await;
    let listed = snapshot.servers[0]
        .threads
        .iter()
        .find(|thread| thread.id == post)
        .unwrap();
    assert_eq!(listed.name, "Crash when dragging");
    assert_eq!(listed.parent.as_deref(), Some(ideas.id.as_str()));
    assert_eq!(
        listed.thread.as_deref().unwrap().tags,
        std::slice::from_ref(&bug)
    );
    let first = discord
        .history(guild, &post, Span::Latest)
        .await
        .unwrap()
        .messages;
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id, post, "the first message has the post's id");
    assert_eq!(first[0].content, "It closes.");
    assert_eq!(first[0].attachments.len(), 1);
    assert_eq!(first[0].attachments[0].name, "notes.txt");
    let creates: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/thread-creates", "")).unwrap();
    let body = creates.as_array().unwrap().last().unwrap();
    assert_eq!(body["name"], "Crash when dragging");
    assert_eq!(body["applied_tags"], serde_json::json!([bug]));
    assert_eq!(body["auto_archive_duration"], 4320);
    assert_eq!(body["message"]["content"], "It closes.");
    assert_eq!(body["message"]["attachments"][0]["filename"], "notes.txt");
    post_control(server, "/_control/forum-require-tag?off", "");
}

/// The id of a forum's `index`th tag.
fn details_tag(forum: &backcord_discord::ready::Channel, index: usize) -> String {
    forum.forum.as_deref().unwrap().tags[index].id.clone()
}

/// A channel's list of threads, as Discord's client fills it; part of
/// [`talks_to_the_fake_server`], whose session it shares.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn lists_a_channels_threads_as_discords_client_does() {
    let fixture = Fixture::start("lists_a_channels_threads_as_discords_client_does").await;
    let discord = &fixture.discord;
    let guild = fixture.guild.as_str();
    let general = fixture.general.as_str();
    let (guild, general) = (guild.to_owned(), general.to_owned());
    discord.view(&guild, Some(&general));
    // Viewing the server subscribed to its threads: the active ones came,
    // with their newest messages, but not a private one she isn't in.
    let snapshot = snapshot_where(discord, |snapshot| {
        snapshot.servers[0]
            .threads
            .iter()
            .any(|thread| thread.name == "Book club")
    })
    .await;
    let threads = &snapshot.servers[0].threads;
    let book_club = threads
        .iter()
        .find(|thread| thread.name == "Book club")
        .unwrap();
    let details = book_club.thread.as_deref().unwrap();
    assert!(!details.joined);
    assert_eq!(details.members, 2);
    let recent = details.recent.as_deref().expect("its newest message");
    assert_eq!(recent.content, "Chapter 3 by Friday?");
    assert!(
        threads
            .iter()
            .all(|thread| thread.name != "Surprise for Bob")
    );

    // Archived threads come a page at a time, the newest first.
    let first = discord.threads(&guild, &general, None, 0).await.unwrap();
    assert_eq!(first.threads.len(), 25);
    assert!(first.more);
    assert!(
        first
            .threads
            .iter()
            .all(|thread| thread.thread.as_ref().unwrap().archived)
    );
    assert_eq!(first.threads[0].name, "Game night 1");
    let owner = first.threads[0]
        .thread
        .as_ref()
        .unwrap()
        .owner
        .clone()
        .unwrap();
    assert_eq!(discord.name(&guild, &owner).as_deref(), Some("Bob"));
    let second = discord.threads(&guild, &general, None, 25).await.unwrap();
    assert!(!second.more);
    assert!(second.threads.len() >= 5);
    // A search finds active and archived ones by name.
    let found = discord
        .threads(&guild, &general, Some("night"), 0)
        .await
        .unwrap();
    assert!(found.threads.len() >= 5);
    assert!(
        found
            .threads
            .iter()
            .all(|thread| thread.name.contains("night"))
    );

    // One the list found opens from what Discord gave, and lists meanwhile.
    let id = first.threads[3].id.clone();
    let listed = discord.listed_thread(&id);
    assert!(listed.is_some());
    discord.forget_listed_threads();
    assert!(discord.listed_thread(&id).is_none());
    discord
        .open_thread(&guild, &id, listed, false)
        .await
        .unwrap();
    snapshot_where(discord, |snapshot| {
        snapshot.viewing.contains(&id)
            && snapshot.servers[0]
                .threads
                .iter()
                .any(|thread| thread.id == id)
    })
    .await;
}

/// A thread this account is in goes quiet its archive duration after its
/// last activity, without Discord saying so; part of
/// [`talks_to_the_fake_server`], whose session it shares.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn marks_quiet_threads_as_discords_client_does() {
    let fixture = Fixture::start("marks_quiet_threads_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let plans_quiet = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .threads()
            .find(|thread| thread.name == "Plans")
            .map(|thread| thread.marks.quiet)
    };
    // Opened again as the fake started, it isn't quiet yet.
    assert_eq!(plans_quiet(&discord.snapshots().borrow()), Some(false));
    // Once its time passes, a snapshot marks it quiet with nothing else
    // happening.
    post_control(server, "/_control/thread-quiet?in=1500", "");
    let updated = snapshot_where(discord, |snapshot| {
        snapshot.servers[0].threads.iter().any(|thread| {
            thread
                .thread
                .as_deref()
                .is_some_and(|details| details.archive_after == 60)
        })
    })
    .await;
    assert_eq!(plans_quiet(&updated), Some(false));
    snapshot_where(discord, |snapshot| plans_quiet(snapshot) == Some(true)).await;
    post_control(server, "/_control/thread-quiet?reset", "");
    snapshot_where(discord, |snapshot| plans_quiet(snapshot) == Some(false)).await;
}

/// Threads Alice starts, as Discord's client starts them; part of
/// [`talks_to_the_fake_server`], whose session it shares.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn starts_threads_as_discords_client_does() {
    let fixture = Fixture::start("starts_threads_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    let guild = fixture.guild.as_str();
    let general = fixture.general.as_str();
    let latest = discord
        .history(guild, general, Span::Latest)
        .await
        .unwrap()
        .messages;
    let message = latest.last().unwrap().id.clone();
    // From a message: the thread takes its id, and she's in it.
    let thread = discord
        .start_thread(guild, general, Some(&message), "Snacks", false, "Message")
        .await
        .unwrap();
    assert_eq!(thread, message);
    snapshot_where(discord, |snapshot| {
        snapshot.servers[0].threads.iter().any(|listed| {
            listed.id == thread
                && listed.name == "Snacks"
                && listed.thread.as_ref().is_some_and(|thread| thread.joined)
        })
    })
    .await;
    // A message has one thread at most, and a name has 1 to 100 letters.
    assert!(
        discord
            .start_thread(guild, general, Some(&message), "Again", false, "Message")
            .await
            .is_err()
    );
    assert!(
        discord
            .start_thread(guild, general, None, &"a".repeat(101), false, "Plus Button")
            .await
            .is_err()
    );
    // Without a message, private as she chose.
    let private = discord
        .start_thread(
            guild,
            general,
            None,
            "Surprise",
            true,
            "Thread Browser Toolbar",
        )
        .await
        .unwrap();
    snapshot_where(discord, |snapshot| {
        snapshot.servers[0].threads.iter().any(|listed| {
            listed.id == private && listed.thread.as_ref().is_some_and(|thread| thread.private)
        })
    })
    .await;
    let creates: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/thread-creates", "")).unwrap();
    let creates = creates.as_array().unwrap();
    assert_eq!(
        creates[0],
        serde_json::json!({
            "channel": general,
            "message": message,
            "name": "Snacks",
            "type": 11,
            "auto_archive_duration": 4320,
            "location": "Message",
        })
    );
    assert_eq!(
        creates.last().unwrap(),
        &serde_json::json!({
            "channel": general,
            "name": "Surprise",
            "type": 12,
            "auto_archive_duration": 4320,
            "location": "Thread Browser Toolbar",
        })
    );
}

/// Giving a forum its tags whole, as Discord's client saves them from the
/// forum's settings: one renamed, one added, the rest deleted.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn edits_a_forums_tags_as_discords_client_does() {
    use backcord_discord::forum::{Emoji, TagEdit};
    let fixture = Fixture::start("edits_a_forums_tags_as_discords_client_does").await;
    let discord = &fixture.discord;
    let server = fixture.server;
    discord.view(&fixture.guild, Some(&fixture.general));
    // The forum's id, and each tag's id, name and whether it's moderated.
    type Tags = (String, Vec<(String, String, bool)>);
    let tags = |snapshot: &Snapshot| -> Option<Tags> {
        let forum = snapshot.servers[0]
            .text_channels
            .iter()
            .find(|channel| channel.name == "ideas")?;
        let tags = forum
            .forum
            .as_deref()?
            .tags
            .iter()
            .map(|tag| (tag.id.clone(), tag.name.clone(), tag.moderated))
            .collect();
        Some((forum.id.clone(), tags))
    };
    let (forum, before) =
        tags(&*snapshot_where(discord, |snapshot| tags(snapshot).is_some()).await).unwrap();
    let feature = before[0].0.clone();
    discord
        .edit_forum_tags(
            &forum,
            &[
                TagEdit {
                    id: Some(feature.clone()),
                    name: "Features".into(),
                    moderated: false,
                    emoji: Some(Emoji::Unicode("🚀".into())),
                },
                TagEdit {
                    id: None,
                    name: "Docs".into(),
                    moderated: true,
                    emoji: None,
                },
            ],
        )
        .await
        .unwrap();
    let snapshot = snapshot_where(discord, |snapshot| {
        tags(snapshot).is_some_and(|(_, tags)| tags.len() == 2)
    })
    .await;
    let (_, after) = tags(&snapshot).unwrap();
    assert_eq!(after[0], (feature, "Features".into(), false));
    assert_eq!((after[1].1.as_str(), after[1].2), ("Docs", true));
    assert!(!after[1].0.is_empty(), "Discord gave the new one an id");
    let sent: serde_json::Value =
        serde_json::from_str(&self::control(server, "GET", "/_control/forum-tags", "")).unwrap();
    assert_eq!(sent[0][0]["emoji_name"], "🚀");
    assert!(
        sent[0][1].get("id").is_none(),
        "a new tag goes without an id"
    );
    // A name too long is refused.
    let long = TagEdit {
        name: "x".repeat(51),
        ..TagEdit::default()
    };
    assert!(discord.edit_forum_tags(&forum, &[long]).await.is_err());
}
