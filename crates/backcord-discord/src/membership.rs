//! A server's gate for new members: its rules or form (member
//! verification), which a member waiting on it (`pending`) fills in before
//! talking, and its onboarding questions, whose answers give roles and
//! channels. See "Joining a server's rules and questions" in
//! docs/discord/servers.md.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

/// The kind of one of a form's fields, by Discord's `field_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// The server's rules, which the member agrees to.
    Terms,
    /// A short answer, up to 150 characters.
    Text,
    /// A paragraph, up to 1000 characters.
    Paragraph,
    /// One of its `choices`, answered by index.
    Choice,
    /// One BACKCORD doesn't know, which it can't fill in.
    Other,
}

/// One field of a server's form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub kind: FieldKind,
    pub label: String,
    pub description: Option<String>,
    pub placeholder: Option<String>,
    /// The rules, for [`FieldKind::Terms`].
    pub rules: Vec<String>,
    /// The choices, for [`FieldKind::Choice`].
    pub choices: Vec<String>,
    pub required: bool,
}

/// A server's form, as `GET /guilds/{guild}/member-verification` has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    pub description: Option<String>,
    pub fields: Vec<Field>,
}

/// What the member answered to one field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Agreed,
    Text(String),
    Choice(usize),
    Nothing,
}

/// One choice of an onboarding question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Its emoji: a Unicode one's text, or a custom one's picture.
    pub emoji: Option<String>,
    pub emoji_picture: Option<String>,
}

/// One onboarding question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub title: String,
    /// Only one of its choices may be picked.
    pub single: bool,
    pub required: bool,
    /// Asked as one joins, rather than only in Channels & Roles.
    pub in_onboarding: bool,
    pub options: Vec<Choice>,
}

/// A server's onboarding, as `GET /guilds/{guild}/onboarding` has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Onboarding {
    pub enabled: bool,
    pub questions: Vec<Question>,
    /// The choices this account picked already.
    pub chosen: Vec<String>,
}

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// Reads a server's form; none when it has no fields.
pub fn form(answer: &Value) -> Option<Form> {
    let fields: Vec<Field> = answer["form_fields"]
        .as_array()?
        .iter()
        .map(|field| Field {
            kind: match field["field_type"].as_str() {
                Some("TERMS") => FieldKind::Terms,
                Some("TEXT_INPUT") => FieldKind::Text,
                Some("PARAGRAPH") => FieldKind::Paragraph,
                Some("MULTIPLE_CHOICE") => FieldKind::Choice,
                _ => FieldKind::Other,
            },
            label: text(&field["label"]).unwrap_or_default(),
            description: text(&field["description"]),
            placeholder: text(&field["placeholder"]),
            rules: strings(&field["values"]),
            choices: strings(&field["choices"]),
            required: field["required"].as_bool().unwrap_or(true),
        })
        .collect();
    (!fields.is_empty()).then(|| Form {
        description: text(&answer["description"]),
        fields,
    })
}

/// The body of `PUT /guilds/{guild}/requests/@me`: the form's fields as
/// Discord sent them, each with its `response`, and its `version`.
pub fn filled(answer: &Value, answers: &[Answer]) -> Value {
    let fields: Vec<Value> = answer["form_fields"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(at, field)| {
            let mut field = field.clone();
            let response = match answers.get(at) {
                Some(Answer::Agreed) => json!(true),
                Some(Answer::Text(typed)) => json!(typed.trim()),
                Some(Answer::Choice(index)) => json!(index),
                Some(Answer::Nothing) | None => Value::Null,
            };
            if let Some(object) = field.as_object_mut() {
                object.insert("response".into(), response);
            }
            field
        })
        .collect();
    json!({"version": answer["version"], "form_fields": fields})
}

/// Whether a join request Discord answered with lets the member in now:
/// a form of rules alone is approved as it's sent; others wait for the
/// server's moderators.
pub fn approved(answer: &Value) -> bool {
    !matches!(
        answer["application_status"].as_str(),
        Some("SUBMITTED" | "STARTED" | "REJECTED")
    )
}

/// Reads a server's onboarding.
pub fn onboarding(answer: &Value) -> Onboarding {
    let questions = answer["prompts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|prompt| {
            let options: Vec<Choice> = prompt["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|option| {
                    let emoji = &option["emoji"];
                    let custom = emoji["id"]
                        .as_str()
                        .or_else(|| option["emoji_id"].as_str())
                        .filter(|id| !id.is_empty());
                    let animated = emoji["animated"]
                        .as_bool()
                        .or_else(|| option["emoji_animated"].as_bool())
                        .unwrap_or(false);
                    Some(Choice {
                        id: option["id"].as_str()?.to_owned(),
                        title: text(&option["title"]).unwrap_or_default(),
                        description: text(&option["description"]),
                        emoji: if custom.is_some() {
                            None
                        } else {
                            text(&emoji["name"]).or_else(|| text(&option["emoji_name"]))
                        },
                        emoji_picture: custom
                            .map(|id| crate::ready::custom_emoji(id, animated, 48)),
                    })
                })
                .collect();
            (!options.is_empty()).then(|| Question {
                id: prompt["id"].as_str().unwrap_or_default().to_owned(),
                title: text(&prompt["title"]).unwrap_or_default(),
                single: prompt["single_select"].as_bool().unwrap_or(false),
                required: prompt["required"].as_bool().unwrap_or(false),
                in_onboarding: prompt["in_onboarding"].as_bool().unwrap_or(true),
                options,
            })
        })
        .collect();
    Onboarding {
        enabled: answer["enabled"].as_bool().unwrap_or(false),
        questions,
        chosen: strings(&answer["responses"]),
    }
}

/// The body of `POST /guilds/{guild}/onboarding-responses`: the choices
/// picked, and every question and choice as seen `now`, in Unix
/// milliseconds, as Discord's client sends them.
pub fn responses(onboarding: &Onboarding, chosen: &[String], now: u64) -> Value {
    let mut prompts = BTreeMap::new();
    let mut options = BTreeMap::new();
    for question in &onboarding.questions {
        prompts.insert(question.id.clone(), json!(now));
        for option in &question.options {
            options.insert(option.id.clone(), json!(now));
        }
    }
    let known: Vec<&String> = chosen
        .iter()
        .filter(|id| {
            onboarding
                .questions
                .iter()
                .any(|question| question.options.iter().any(|option| &option.id == *id))
        })
        .collect();
    json!({
        "onboarding_responses": known,
        "onboarding_prompts_seen": prompts.into_iter().collect::<Map<_, _>>(),
        "onboarding_responses_seen": options.into_iter().collect::<Map<_, _>>(),
    })
}

/// Whether this account's member object says it hasn't passed the
/// server's form yet.
pub fn pending(member: &Value) -> bool {
    member["pending"].as_bool().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verification() -> Value {
        json!({
            "version": "2026-01-01T00:00:00+00:00",
            "description": "Be nice",
            "form_fields": [
                {"field_type": "TERMS", "label": "Read and agree to the server rules",
                 "values": ["No spam", "Be kind"], "required": true},
                {"field_type": "MULTIPLE_CHOICE", "label": "Age?", "choices": ["<18", "18+"],
                 "required": true},
                {"field_type": "TEXT_INPUT", "label": "Why?", "required": false}
            ]
        })
    }

    #[test]
    fn reads_a_form_and_fills_it_as_sent() {
        let form = form(&verification()).unwrap();
        assert_eq!(form.fields.len(), 3);
        assert_eq!(form.fields[0].kind, FieldKind::Terms);
        assert_eq!(form.fields[0].rules, ["No spam", "Be kind"]);
        assert_eq!(form.fields[1].choices, ["<18", "18+"]);
        assert!(!form.fields[2].required);
        let body = filled(
            &verification(),
            &[
                Answer::Agreed,
                Answer::Choice(1),
                Answer::Text(" hi ".into()),
            ],
        );
        assert_eq!(body["version"], "2026-01-01T00:00:00+00:00");
        assert_eq!(body["form_fields"][0]["response"], true);
        assert_eq!(body["form_fields"][0]["values"][1], "Be kind");
        assert_eq!(body["form_fields"][1]["response"], 1);
        assert_eq!(body["form_fields"][2]["response"], "hi");
        assert!(super::form(&json!({"form_fields": []})).is_none());
    }

    #[test]
    fn a_submitted_request_waits() {
        assert!(approved(&json!({"application_status": "APPROVED"})));
        assert!(!approved(&json!({"application_status": "SUBMITTED"})));
    }

    #[test]
    fn reads_questions_and_answers_them() {
        let answer = json!({
            "enabled": true,
            "responses": ["21"],
            "prompts": [
                {"id": "1", "title": "Games?", "single_select": false, "required": true,
                 "in_onboarding": true, "options": [
                    {"id": "21", "title": "Chess", "emoji": {"id": null, "name": "♟️"}},
                    {"id": "22", "title": "Go", "description": "Baduk",
                     "emoji": {"id": "99", "name": "go", "animated": false}}
                 ]},
                {"id": "2", "title": "Empty", "options": []}
            ]
        });
        let onboarding = onboarding(&answer);
        assert!(onboarding.enabled);
        assert_eq!(onboarding.questions.len(), 1);
        let question = &onboarding.questions[0];
        assert!(question.required && !question.single);
        assert_eq!(question.options[0].emoji.as_deref(), Some("♟️"));
        assert!(question.options[1].emoji.is_none());
        assert!(
            question.options[1]
                .emoji_picture
                .as_deref()
                .unwrap()
                .contains("/emojis/99")
        );
        assert_eq!(onboarding.chosen, ["21"]);
        let body = responses(&onboarding, &["22".into(), "gone".into()], 7);
        assert_eq!(body["onboarding_responses"], json!(["22"]));
        assert_eq!(body["onboarding_prompts_seen"]["1"], 7);
        assert_eq!(body["onboarding_responses_seen"]["21"], 7);
    }
}
