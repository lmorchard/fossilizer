use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outbox<TItem> {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub total_items: i32,
    pub ordered_items: Vec<TItem>,
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
    pub likes: String,
    pub bookmarks: String,
    pub preferred_username: String,
    pub name: String,
    pub summary: Option<String>,
    pub url: String,
    pub published: String,
    pub icon: Option<Attachment>,
    pub image: Option<Attachment>,
    pub public_key: Option<PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrObject {
    String(String),
    Object(Object),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub actor: String,
    pub published: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub object: Option<StringOrObject>,
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
    // pub attachment: Vec<Attachment>,
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

// todo: actor?
// todo: object?

pub type OutboxWithActivities = Outbox<Activity>;

pub type OutboxWithValues = Outbox<serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    const JSON_OUTBOX: &str = include_str!("./resources/test/outbox.json");
    const JSON_ACTIVITY_WITH_EMOJI: &str =
        include_str!("./resources/test/activity-with-emoji.json");

    #[test]
    fn test_outbox_parsing_mini() -> Result<(), Box<dyn Error>> {
        let outbox: Outbox<Activity> = serde_json::from_str(JSON_OUTBOX)?;

        let ordered_items = outbox.ordered_items;
        assert!(!ordered_items.is_empty());
        let item1 = ordered_items.get(0).ok_or("no item1")?;
        assert_eq!(
            item1.id,
            "https://mastodon.social/users/lmorchard/statuses/55864/activity"
        );
        assert_eq!(item1.type_field, "Create");
        assert_eq!(item1.actor, "https://mastodon.social/users/lmorchard");

        Ok(())
    }

    #[test]
    fn test_outbox_parsing_full() -> Result<(), Box<dyn Error>> {
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
