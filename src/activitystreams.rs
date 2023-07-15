use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref PUBLIC_ID: String = "https://www.w3.org/ns/activitystreams#Public".to_string();
}

pub trait OrderedItems<TItem: Serialize> {
    fn ordered_items(&self) -> &Vec<TItem>;
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outbox<TItem: Serialize> {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub total_items: i32,
    pub ordered_items: Vec<TItem>,
}
impl<TItem: Serialize> OrderedItems<TItem> for Outbox<TItem> {
    fn ordered_items(&self) -> &Vec<TItem> {
        &self.ordered_items
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub total_items: i32,
    pub first: String,
    pub last: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollectionPage<TItem: Serialize> {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub next: Option<String>,
    pub prev: Option<String>,
    pub ordered_items: Vec<TItem>,
}
impl<TItem: Serialize> OrderedItems<TItem> for OrderedCollectionPage<TItem> {
    fn ordered_items(&self) -> &Vec<TItem> {
        &self.ordered_items
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub following: String,
    pub followers: String,
    pub inbox: String,
    pub outbox: String,
    pub likes: Option<String>,
    pub bookmarks: Option<String>,
    pub preferred_username: String,
    pub name: String,
    pub summary: Option<String>,
    pub url: String,
    pub published: String,
    pub icon: Option<Attachment>,
    pub image: Option<Attachment>,
    pub public_key: Option<PublicKey>,
}

impl Actor {
    pub fn id_hash(&self) -> String {
        sha256::digest(self.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {}

#[allow(clippy::large_enum_variant)]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdOrObject<T> {
    #[default]
    None,
    Id(String),
    Object(T),
}
impl<T> IdOrObject<T> {
    pub fn is_none(&self) -> bool {
        matches!(self, IdOrObject::None)
    }
    pub fn id(&self) -> Option<&String> {
        match &self {
            IdOrObject::Id(v) => Some(v),
            _ => None,
        }
    }
    pub fn object(&self) -> Option<&T> {
        match &self {
            IdOrObject::Object(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub published: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub actor: IdOrObject<Actor>,
    pub object: IdOrObject<Object>,
}

impl Activity {
    pub fn is_public(&self) -> bool {
        self.to.contains(&PUBLIC_ID) || self.cc.contains(&PUBLIC_ID)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub url: String,
    pub published: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub in_reply_to: Option<String>,
    pub tag: Vec<Tag>,
    pub attachment: Vec<Attachment>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    #[serde(rename = "type")]
    pub type_field: String,
    pub id: Option<String>,
    pub href: Option<String>,
    pub name: String,
    pub icon: Option<Attachment>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(rename = "type")]
    pub type_field: String,
    pub media_type: String,
    pub url: String,
    pub name: Option<String>,
    pub blurhash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    const JSON_OUTBOX: &str = include_str!("./resources/test/outbox.json");
    const JSON_ACTIVITY_WITH_EMOJI: &str =
        include_str!("./resources/test/activity-with-emoji.json");

    #[test]
    fn test_outbox_parsing_with_local_model() -> Result<(), Box<dyn Error>> {
        let outbox: Outbox<Activity> = serde_json::from_str(JSON_OUTBOX)?;

        let ordered_items = outbox.ordered_items;
        assert!(!ordered_items.is_empty());
        let item1 = ordered_items.get(0).ok_or("no item1")?;
        assert_eq!(
            item1.id,
            "https://mastodon.social/users/lmorchard/statuses/55864/activity"
        );
        assert_eq!(item1.type_field, "Create");
        assert_eq!(
            item1.actor.id().ok_or("no actor id")?,
            "https://mastodon.social/users/lmorchard"
        );

        Ok(())
    }

    #[test]
    fn test_outbox_parsing_with_external_model() -> Result<(), Box<dyn Error>> {
        let outbox: Outbox<activitystreams::activity::ActivityBox> =
            serde_json::from_str(JSON_OUTBOX)?;

        let ordered_items = outbox.ordered_items;
        assert!(!ordered_items.is_empty());

        let item1 = &ordered_items[0];
        let item1: activitystreams::activity::Create = item1.clone().into_concrete()?;

        assert_eq!(
            item1.object_props.id.ok_or("no id")?.as_str(),
            "https://mastodon.social/users/lmorchard/statuses/55864/activity"
        );

        // assert_eq!(item1.kind, activitystreams::activity::kind::CreateType);
        assert_eq!(
            item1
                .create_props
                .get_actor_xsd_any_uri()
                .ok_or("no actor")?
                .as_str(),
            "https://mastodon.social/users/lmorchard"
        );
        Ok(())
    }

    #[test]
    fn test_activity_parsing_with_emoji() -> Result<(), Box<dyn Error>> {
        let activity: Activity = serde_json::from_str(JSON_ACTIVITY_WITH_EMOJI)?;
        assert_eq!(
            activity.id,
            "https://toot.cafe/users/lmorchard/statuses/100599986688993237/activity",
        );
        Ok(())
    }
}
