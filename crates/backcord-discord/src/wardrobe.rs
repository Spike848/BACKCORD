//! The collectibles this account owns, and the shop's, for the profile
//! editor's choices: avatar decorations, profile effects, nameplates and
//! profile frames. `GET /users/@me/collectibles-purchases` lists those owned;
//! wearing one is a field of `PATCH /users/@me` or `/users/@me/profile`, as
//! Discord's client saves it. Buying goes through Discord's own checkout,
//! which BACKCORD leaves to Discord's shop page.

use serde_json::{Value, json};

use crate::wishlist::collectible_item_picture;

/// What a collectible is, by Discord's `type` on its item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Decoration,
    Effect,
    Nameplate,
    Frame,
}

impl Kind {
    fn of(kind: u64) -> Option<Self> {
        match kind {
            0 => Some(Self::Decoration),
            1 => Some(Self::Effect),
            2 => Some(Self::Nameplate),
            3 => Some(Self::Frame),
            _ => None,
        }
    }

    /// The body that wears `sku`, or with none takes it off, sent to
    /// `PATCH /users/@me/profile` as Discord's profile editor saves all four,
    /// or failing that to `PATCH /users/@me`.
    pub fn wear(self, sku: Option<&str>) -> Value {
        match self {
            Self::Decoration => json!({"avatar_decoration_sku_id": sku}),
            Self::Nameplate => json!({"nameplate_sku_id": sku}),
            Self::Effect => json!({"profile_effect_id": sku}),
            Self::Frame => json!({"profile_frame_sku_id": sku}),
        }
    }
}

/// A collectible to choose: one owned, or one the shop sells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    /// The SKU it's worn by.
    pub sku: String,
    /// The product it's sold as, whose page the shop opens at.
    pub product: String,
    pub name: String,
    /// The address of its still picture.
    pub picture: Option<String>,
    /// A frame's layers, the back ones first, which draw it whole.
    pub frame: Vec<crate::profile::FrameLayer>,
}

/// The items of the products in an answer from
/// `GET /users/@me/collectibles-purchases`: an array of products, or
/// `{purchases: [...]}`.
pub fn owned(answer: &Value) -> Vec<Item> {
    let products = answer
        .as_array()
        .or_else(|| answer["purchases"].as_array())
        .or_else(|| answer["products"].as_array());
    let mut found = Vec::new();
    for product in products.into_iter().flatten() {
        walk(product, &mut found);
    }
    dedupe(found)
}

/// The items the shop sells now: each product of each category still
/// published, and the variants a product groups.
pub fn shop(answer: &Value, now_ms: u64) -> Vec<Item> {
    let mut found = Vec::new();
    let categories = answer
        .as_array()
        .or_else(|| answer["categories"].as_array());
    for category in categories.into_iter().flatten() {
        let ended = category["unpublished_at"]
            .as_str()
            .and_then(crate::message::iso_ms)
            .is_some_and(|until| until <= now_ms);
        if ended {
            continue;
        }
        for product in category["products"].as_array().into_iter().flatten() {
            walk(product, &mut found);
        }
    }
    dedupe(found)
}

fn walk(product: &Value, found: &mut Vec<Item>) {
    let product_sku = product["sku_id"].as_str().unwrap_or_default();
    for item in product["items"].as_array().into_iter().flatten() {
        let Some(kind) = item["type"].as_u64().and_then(Kind::of) else {
            continue;
        };
        let Some(sku) = item["sku_id"]
            .as_str()
            .or(Some(product_sku))
            .filter(|sku| !sku.is_empty())
        else {
            continue;
        };
        let name = [&product["name"], &item["label"], &item["title"]]
            .into_iter()
            .find_map(|name| name.as_str().filter(|name| !name.trim().is_empty()))
            .unwrap_or_default();
        found.push(Item {
            kind,
            sku: sku.to_owned(),
            product: if product_sku.is_empty() {
                sku
            } else {
                product_sku
            }
            .to_owned(),
            name: name.to_owned(),
            // A decoration as cards show one, by its SKU.
            picture: match (kind, sku.parse::<u64>()) {
                (Kind::Decoration, Ok(id)) => Some(crate::collectibles::picture(id)),
                _ => collectible_item_picture(sku, item),
            },
            frame: item["layers"]
                .as_array()
                .map(|layers| crate::profile::layers_of(sku, layers))
                .unwrap_or_default(),
        });
    }
    for variant in product["variants"].as_array().into_iter().flatten() {
        walk(variant, found);
    }
}

/// Each SKU once, the first found.
fn dedupe(mut items: Vec<Item>) -> Vec<Item> {
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| seen.insert(item.sku.clone()));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_owned_collectibles_of_every_kind() {
        let answer = json!([
            {"sku_id": "1", "name": "Cat ears", "items": [
                {"type": 0, "sku_id": "1", "asset": "a_cat", "label": "Cat"}]},
            {"sku_id": "2", "name": "Sparkles", "items": [
                {"type": 1, "sku_id": "2", "title": "Sparkles", "thumbnailPreviewSrc": "https://x/s.png"}]},
            {"sku_id": "3", "name": "", "items": [
                {"type": 2, "sku_id": "3", "asset": "nameplates/bonsai/", "label": "Bonsai"}]},
            {"sku_id": "4", "name": "Gold", "variants": [
                {"sku_id": "5", "name": "Gold", "items": [
                    {"type": 3, "sku_id": "5", "layers": [{"id": "9", "type": "rail"}]}]}]},
            {"sku_id": "1", "name": "Cat ears again", "items": [{"type": 0, "sku_id": "1"}]},
        ]);
        let items = owned(&answer);
        let kinds: Vec<_> = items
            .iter()
            .map(|item| (item.kind, item.sku.as_str(), item.name.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                (Kind::Decoration, "1", "Cat ears"),
                (Kind::Effect, "2", "Sparkles"),
                (Kind::Nameplate, "3", "Bonsai"),
                (Kind::Frame, "5", "Gold"),
            ]
        );
        assert!(
            items[0]
                .picture
                .as_deref()
                .unwrap()
                .ends_with("/collectibles-shop/1/static")
        );
        assert_eq!(items[1].picture.as_deref(), Some("https://x/s.png"));
        assert_eq!(items[3].product, "5");
        assert_eq!(items[3].frame.len(), 1, "a frame keeps its layers");
    }

    #[test]
    fn the_shop_leaves_out_categories_no_longer_sold() {
        let answer = json!({"categories": [
            {"unpublished_at": "2020-01-01T00:00:00+00:00", "products": [
                {"sku_id": "1", "name": "Old", "items": [{"type": 0, "sku_id": "1"}]}]},
            {"unpublished_at": null, "products": [
                {"sku_id": "2", "name": "New", "items": [{"type": 2, "sku_id": "2"}]}]},
        ]});
        let items = shop(&answer, 1_700_000_000_000);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "New");
    }

    #[test]
    fn wearing_and_taking_off_use_discords_fields() {
        assert_eq!(
            Kind::Decoration.wear(Some("1")),
            json!({"avatar_decoration_sku_id": "1"})
        );
        assert_eq!(
            Kind::Nameplate.wear(None),
            json!({"nameplate_sku_id": null})
        );
        assert_eq!(
            Kind::Effect.wear(Some("2")),
            json!({"profile_effect_id": "2"})
        );
        assert_eq!(
            Kind::Frame.wear(Some("3")),
            json!({"profile_frame_sku_id": "3"})
        );
    }
}
