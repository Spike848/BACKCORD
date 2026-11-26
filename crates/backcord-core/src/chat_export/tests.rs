use super::*;

#[test]
fn the_file_is_named_for_its_place_and_day() {
    let day = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
    assert_eq!(
        file_name("general", ExportFormat::Text, day),
        "general-2026-11-25.txt"
    );
    assert_eq!(
        file_name("#a/b: c?", ExportFormat::Html, day),
        "a_b_ c_-2026-11-25.html"
    );
    assert_eq!(
        file_name("  ", ExportFormat::Json, day),
        "chat-2026-11-25.json"
    );
}

#[test]
fn a_day_is_typed_as_in_italy_or_as_iso() {
    let day = NaiveDate::from_ymd_opt(2026, 3, 7);
    assert_eq!(parse_day("07/03/2026"), day);
    assert_eq!(parse_day(" 7-3-2026 "), day);
    assert_eq!(parse_day("2026-03-07"), day);
    assert_eq!(parse_day("31/02/2026"), None);
    assert_eq!(parse_day("ieri"), None);
}

#[cfg(feature = "discord")]
mod rendering {
    use super::*;
    use backcord_discord::message::snowflake_at;
    use serde_json::json;

    /// 25 November 2026, 14:05 UTC, and a minute later.
    const AT: u64 = 1_795_615_500_000;

    fn messages() -> Vec<serde_json::Value> {
        vec![
            json!({
                "id": snowflake_at(AT).to_string(), "type": 0, "channel_id": "11",
                "author": {"id": "1", "username": "ann", "global_name": "Ann"},
                "content": "Hi <@2>, look <:wave:123> & <b>this</b>",
                "mentions": [{"id": "2", "username": "bob", "global_name": null}],
                "attachments": [{
                    "id": "9", "filename": "cat.png", "size": 10,
                    "url": "https://cdn.discordapp.com/attachments/11/9/cat.png",
                    "proxy_url": "https://media.discordapp.net/attachments/11/9/cat.png",
                    "content_type": "image/png", "width": 4, "height": 4
                }],
                "embeds": [],
            }),
            json!({
                "id": snowflake_at(AT + 60_000).to_string(), "type": 6, "channel_id": "11",
                "author": {"id": "2", "username": "bob"},
                "content": "", "attachments": [], "embeds": [], "mentions": [],
            }),
        ]
    }

    #[test]
    fn text_has_a_line_a_message_with_its_files_under_it() {
        let text = render_in(
            &messages(),
            "general",
            ExportFormat::Text,
            true,
            &chrono::Utc,
        );
        assert_eq!(
            text,
            "[25/11/2026 14:05] Ann: Hi @bob, look :wave: & <b>this</b>\n\
             \x20   cat.png: https://cdn.discordapp.com/attachments/11/9/cat.png\n\
             [25/11/2026 14:06] bob pinned a message.\n"
        );
        let bare = render_in(
            &messages(),
            "general",
            ExportFormat::Text,
            false,
            &chrono::Utc,
        );
        assert!(!bare.contains("cat.png"), "files left out");
    }

    #[test]
    fn the_page_is_escaped_with_pictures_as_links() {
        let page = render_in(
            &messages(),
            "<general>",
            ExportFormat::Html,
            true,
            &chrono::Utc,
        );
        assert!(page.starts_with("<!DOCTYPE html>"));
        assert!(page.contains("<title>&lt;general&gt;</title>"));
        assert!(page.contains("&amp; &lt;b&gt;this&lt;/b&gt;"), "{page}");
        assert!(page.contains(
            "<a href=\"https://cdn.discordapp.com/attachments/11/9/cat.png\">cat.png</a>"
        ));
        assert!(!page.contains("<img"), "pictures are links");
        assert!(page.contains("25/11/2026 14:05"));
    }

    #[test]
    fn json_keeps_discords_objects_oldest_first() {
        let all: Vec<serde_json::Value> = serde_json::from_str(&render_in(
            &messages(),
            "general",
            ExportFormat::Json,
            true,
            &chrono::Utc,
        ))
        .unwrap();
        assert_eq!(all, messages());
        let bare: Vec<serde_json::Value> = serde_json::from_str(&render_in(
            &messages(),
            "general",
            ExportFormat::Json,
            false,
            &chrono::Utc,
        ))
        .unwrap();
        assert!(bare[0].get("attachments").is_none());
        assert_eq!(bare[0]["content"], messages()[0]["content"]);
    }
}
