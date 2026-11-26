//! Fake Nitro, as Vencord's fakeNitro has it, without anything that misleads
//! Discord: a custom emoji this account may not use where it writes (another
//! server's, or an animated one, without Nitro) goes as a link to its
//! picture, and so does a sticker it may not send. Links to emoji pictures
//! that come, bare or masked as `[name](link)`, show as the emoji they are,
//! and the picture Discord embeds for them goes.
//!
//! It is off unless Settings' Extra Features turn it on; see
//! [`crate::extras::FakeNitro`] for its settings.

use std::borrow::Cow;

use crate::chat::{CustomEmoji, Sticker};
use crate::extras::FakeNitro;

/// The sizes an emoji's picture may be linked at, as Settings offers them.
pub const SIZES: [u16; 6] = [32, 48, 64, 96, 128, 160];

/// The size an emoji's picture is linked at unless chosen otherwise, as
/// Vencord's plugin has it.
pub const DEFAULT_SIZE: u16 = 48;

/// The size a sticker's picture is linked at, as Discord shows stickers.
const STICKER_SIZE: u16 = 160;

/// Discord's `format_type` of a Lottie sticker, which has no picture to link.
const LOTTIE: u64 = 3;
/// Discord's `format_type` of a GIF sticker.
const GIF: u64 = 4;

/// The address of a custom emoji's picture at `size`: a GIF for an animated
/// one. Its name rides along, for [`shown`] to give it back.
pub fn emoji_link(id: &str, name: &str, animated: bool, size: u16) -> String {
    let format = if animated { "gif" } else { "webp" };
    format!(
        "https://cdn.discordapp.com/emojis/{id}.{format}?size={size}&quality=lossless&name={name}"
    )
}

/// The address of a sticker's picture, or none for a Lottie sticker, which
/// has none.
pub fn sticker_link(sticker: &Sticker) -> Option<String> {
    let format = match sticker.format {
        LOTTIE => return None,
        GIF => "gif",
        _ => "png",
    };
    Some(format!(
        "https://media.discordapp.net/stickers/{}.{format}?size={STICKER_SIZE}",
        sticker.id
    ))
}

/// A link as the message carries it: masked under `label`, or bare.
fn linked(label: &str, link: &str, masked: bool) -> String {
    // Brackets and parentheses would end the mask early.
    let label: String = label
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | '(' | ')'))
        .collect();
    if masked && !label.trim().is_empty() {
        format!("[{label}]({link})")
    } else {
        link.to_owned()
    }
}

/// What a message says for a sticker this account may not send: the link to
/// its picture, or none for one without a picture.
pub fn sticker_message(sticker: &Sticker, settings: FakeNitro) -> Option<String> {
    sticker_link(sticker).map(|link| linked(&sticker.name, &link, settings.masked))
}

/// `text`, as [`crate::chat::wire_emoji`] left it, with each `:name:` of an
/// emoji among `locked` as the link to its picture, apart from what's
/// around it. Code keeps its text.
pub fn wire(text: &str, locked: &[CustomEmoji], settings: FakeNitro) -> String {
    let mut out = String::with_capacity(text.len());
    let mut code = false;
    let mut rest = text;
    while let Some(next) = rest.chars().next() {
        if next == '`' {
            code = !code;
        }
        if !code
            && next == ':'
            && !out.ends_with('<')
            && !out.ends_with("<a")
            && let Some(end) = rest[1..].find(':')
            && let name = &rest[1..1 + end]
            && name.len() >= 2
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && let Some(emoji) = locked
                .iter()
                .find(|emoji| emoji.text[1..emoji.text.len() - 1] == *name)
        {
            let link = emoji_link(&emoji.id, name, emoji.animated, settings.size);
            if out.chars().next_back().is_some_and(|c| !c.is_whitespace()) {
                out.push(' ');
            }
            out.push_str(&linked(name, &link, settings.masked));
            rest = &rest[end + 2..];
            if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
                out.push(' ');
            }
            continue;
        }
        out.push(next);
        rest = &rest[next.len_utf8()..];
    }
    out
}

/// A custom emoji a link points to the picture of: its id, whether it's
/// animated, and the name the link carries, if it's one an emoji may have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Linked<'a> {
    pub id: &'a str,
    pub animated: bool,
    pub name: Option<&'a str>,
}

/// A name a custom emoji may have, as Discord writes them.
fn emoji_name(name: &str) -> bool {
    name.len() >= 2 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The emoji whose picture `link` is, on Discord's CDN or its media proxy,
/// as [`emoji_link`] makes them; none for any other link.
pub fn linked_emoji(link: &str) -> Option<Linked<'_>> {
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))?;
    let rest = rest
        .strip_prefix("cdn.discordapp.com/emojis/")
        .or_else(|| rest.strip_prefix("media.discordapp.net/emojis/"))?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (id, format) = path.split_once('.')?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !matches!(format, "webp" | "png" | "gif" | "jpg" | "jpeg") {
        return None;
    }
    let param = |key: &str| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
    };
    Some(Linked {
        id,
        animated: format == "gif" || param("animated") == Some("true"),
        name: param("name").filter(|name| emoji_name(name)),
    })
}

/// The id of the emoji whose picture an embedded picture's address shows:
/// Discord's, or its media proxy's copy of one, which keeps the path.
fn embedded_emoji(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("/emojis/")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (id, format) = rest.split_at(digits);
    let format = format.strip_prefix('.')?;
    (!id.is_empty()
        && ["webp", "png", "gif"]
            .iter()
            .any(|known| format.starts_with(known)))
    .then_some(id)
}

/// The emoji as a message's text writes it.
fn tag(name: &str, id: &str, animated: bool) -> String {
    let animated = if animated { "a" } else { "" };
    format!("<{animated}:{name}:{id}>")
}

/// `content` with each link to an emoji's picture, bare or masked, as that
/// emoji, `<:name:id>`; code and links in angle brackets keep their text.
/// The name is the one the link carries, else the mask's, else `emoji`.
pub fn shown(content: &str) -> Cow<'_, str> {
    if !content.contains("/emojis/") {
        return Cow::Borrowed(content);
    }
    let mut out = String::with_capacity(content.len());
    let mut code = false;
    let mut rest = content;
    let mut changed = false;
    while let Some(next) = rest.chars().next() {
        if next == '`' {
            code = !code;
        }
        if !code && next == '[' {
            // `[label](link)`
            if let Some((label, after)) = rest[1..].split_once("](")
                && !label.contains(['[', '\n'])
                && let Some((link, after)) = after.split_once(')')
                && let Some(emoji) = linked_emoji(link.trim())
            {
                let name = emoji
                    .name
                    .or_else(|| emoji_name(label.trim()).then(|| label.trim()))
                    .unwrap_or("emoji");
                out.push_str(&tag(name, emoji.id, emoji.animated));
                rest = after;
                changed = true;
                continue;
            }
        }
        if !code
            && (rest.starts_with("https://") || rest.starts_with("http://"))
            && !out.ends_with('<')
            && !out.ends_with("](")
        {
            let end = rest
                .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '|' | '*' | '~'))
                .unwrap_or(rest.len());
            if let Some(emoji) = linked_emoji(&rest[..end]) {
                out.push_str(&tag(
                    emoji.name.unwrap_or("emoji"),
                    emoji.id,
                    emoji.animated,
                ));
                rest = &rest[end..];
                changed = true;
                continue;
            }
        }
        out.push(next);
        rest = &rest[next.len_utf8()..];
    }
    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(content)
    }
}

/// Text that came, as [`shown`] shows it while Fake Nitro shows links to
/// emoji as emoji.
pub fn shown_text(content: &str) -> Cow<'_, str> {
    let settings = crate::extras::fake_nitro();
    if settings.on && settings.received {
        shown(content)
    } else {
        Cow::Borrowed(content)
    }
}

/// A message that came, as Fake Nitro shows it when it shows links to emoji
/// as emoji; none when it's shown as it is.
#[cfg(feature = "discord")]
pub fn shown_message(
    message: &backcord_discord::message::Message,
) -> Option<backcord_discord::message::Message> {
    let settings = crate::extras::fake_nitro();
    (settings.on && settings.received)
        .then(|| shown_message_as(message))
        .flatten()
}

/// The message with its links to emoji as emoji, and without the pictures
/// Discord embedded for them; none when it has no such link. A message of
/// the link alone comes without its text, which Discord leaves out when its
/// picture shows: its picture gives the emoji back.
#[cfg(feature = "discord")]
pub fn shown_message_as(
    message: &backcord_discord::message::Message,
) -> Option<backcord_discord::message::Message> {
    use backcord_discord::message::{Attachment, Shown};
    /// The emoji a picture Discord embedded for a link shows, if it's one.
    fn embedded(attachment: &Attachment) -> Option<&str> {
        if attachment.shown != Shown::Message {
            return None;
        }
        embedded_emoji(&attachment.url).or_else(|| embedded_emoji(attachment.link_url.as_deref()?))
    }
    let mut content = shown(&message.content).into_owned();
    if content.trim().is_empty() && message.system.is_none() {
        let mut tags: Vec<String> = Vec::new();
        for attachment in &message.attachments {
            if let Some(id) = embedded(attachment) {
                let name = linked_emoji(&attachment.url)
                    .and_then(|emoji| emoji.name)
                    .or_else(|| {
                        attachment
                            .link_url
                            .as_deref()
                            .and_then(linked_emoji)
                            .and_then(|emoji| emoji.name)
                    })
                    .unwrap_or("emoji");
                let animated = attachment.animated || attachment.name.ends_with(".gif");
                tags.push(tag(name, id, animated));
            }
        }
        content = tags.join(" ");
    }
    let mut attachments = message.attachments.clone();
    attachments.retain(|attachment| {
        embedded(attachment).is_none_or(|id| !content.contains(&format!(":{id}>")))
    });
    if content == message.content && attachments.len() == message.attachments.len() {
        return None;
    }
    let mut shown = message.clone();
    shown.content = content;
    shown.attachments = attachments;
    Some(shown)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emoji(id: &str, name: &str, animated: bool) -> CustomEmoji {
        CustomEmoji {
            text: format!(":{name}:"),
            id: id.into(),
            animated,
            usable: false,
            picture: String::new(),
        }
    }

    #[test]
    fn locked_emoji_go_as_links_to_their_pictures() {
        let locked = [emoji("5", "talk", false), emoji("6", "party", true)];
        let bare = FakeNitro {
            on: true,
            size: 64,
            masked: false,
            received: true,
        };
        assert_eq!(
            wire(
                "hi:talk: and :party:! `:talk:` <:own:7> :nope:",
                &locked,
                bare
            ),
            "hi https://cdn.discordapp.com/emojis/5.webp?size=64&quality=lossless&name=talk and \
             https://cdn.discordapp.com/emojis/6.gif?size=64&quality=lossless&name=party ! \
             `:talk:` <:own:7> :nope:"
        );
        let masked = FakeNitro {
            masked: true,
            ..bare
        };
        assert_eq!(
            wire(":talk:", &locked, masked),
            "[talk](https://cdn.discordapp.com/emojis/5.webp?size=64&quality=lossless&name=talk)"
        );
    }

    #[test]
    fn a_sticker_goes_as_its_picture_unless_it_has_none() {
        let sticker = |format| Sticker {
            id: "8".into(),
            name: "Wave [hi]".into(),
            format,
            picture: String::new(),
            usable: false,
        };
        let settings = FakeNitro {
            on: true,
            ..FakeNitro::OFF
        };
        assert_eq!(
            sticker_message(&sticker(1), settings).as_deref(),
            Some("[Wave hi](https://media.discordapp.net/stickers/8.png?size=160)")
        );
        assert_eq!(
            sticker_message(
                &sticker(4),
                FakeNitro {
                    masked: false,
                    ..settings
                }
            )
            .as_deref(),
            Some("https://media.discordapp.net/stickers/8.gif?size=160")
        );
        assert_eq!(sticker_message(&sticker(3), settings), None);
    }

    #[test]
    fn links_to_emoji_show_as_emoji() {
        assert_eq!(
            shown(
                "hi https://cdn.discordapp.com/emojis/5.webp?size=48&quality=lossless&name=talk \
                 [party](https://cdn.discordapp.com/emojis/6.gif?size=48) \
                 https://media.discordapp.net/emojis/7.png?animated=true \
                 `https://cdn.discordapp.com/emojis/8.png` <https://cdn.discordapp.com/emojis/9.png> \
                 https://cdn.discordapp.com/emojis/x.png https://example.com/emojis/1.png"
            ),
            "hi <:talk:5> <a:party:6> <a:emoji:7> \
             `https://cdn.discordapp.com/emojis/8.png` <https://cdn.discordapp.com/emojis/9.png> \
             https://cdn.discordapp.com/emojis/x.png https://example.com/emojis/1.png"
        );
        assert!(matches!(shown("nothing here"), Cow::Borrowed(_)));
        // What this side sends comes back as the emoji it was.
        let sent = wire(
            ":talk:",
            &[emoji("5", "talk", true)],
            FakeNitro {
                on: true,
                ..FakeNitro::OFF
            },
        );
        assert_eq!(shown(&sent), "<a:talk:5>");
    }

    #[test]
    #[cfg(feature = "discord")]
    fn a_message_of_emoji_links_loses_their_pictures() {
        use backcord_discord::message::{Attachment, Message, Shown};
        let picture = |url: &str, shown| Attachment {
            name: "5.webp".into(),
            size: None,
            url: url.into(),
            image: true,
            dimensions: None,
            preview_url: Some(url.into()),
            view_url: None,
            still_url: None,
            video_url: None,
            link_url: None,
            animated: false,
            shown,
            spoiler: false,
            description: None,
        };
        let link = "https://cdn.discordapp.com/emojis/5.webp?size=48&name=talk";
        let proxied = "https://media.discordapp.net/emojis/5.webp?size=48&name=talk";
        let file = "https://cdn.discordapp.com/attachments/1/2/cat.png";
        let message = Message {
            content: format!("look {link}"),
            attachments: vec![
                picture(file, Shown::Message),
                picture(proxied, Shown::Message),
            ],
            ..Default::default()
        };
        let shown = shown_message_as(&message).expect("an emoji link");
        assert_eq!(shown.content, "look <:talk:5>");
        assert_eq!(shown.attachments.len(), 1, "the emoji's picture goes");
        assert_eq!(&*shown.attachments[0].url, file);
        // A message of the link alone came without its text.
        let alone = Message {
            content: String::new(),
            attachments: vec![picture(proxied, Shown::Message)],
            ..Default::default()
        };
        let shown = shown_message_as(&alone).expect("an emoji's picture");
        assert_eq!(shown.content, "<:talk:5>");
        assert!(shown.attachments.is_empty());
        let plain = Message {
            content: "hi".into(),
            attachments: vec![picture(file, Shown::Message)],
            ..Default::default()
        };
        assert_eq!(shown_message_as(&plain), None);
    }
}
