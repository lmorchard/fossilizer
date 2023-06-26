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

    #[test]
    fn test_outbox_parsing() -> Result<(), Box<dyn Error>> {
        let outbox: OutboxWithActivities = serde_json::from_str(JSON_OUTBOX)?;

        let ordered_items = outbox.ordered_items;
        assert_eq!(ordered_items.len(), 2);

        let item1 = ordered_items.get(0).ok_or("no item1")?;
        assert_eq!(
            item1.id,
            "https://mastodon.social/users/lmorchard/statuses/55864/activity"
        );
        assert_eq!(item1.type_field, "Create");
        assert_eq!(item1.actor, "https://mastodon.social/users/lmorchard");

        Ok(())
    }
}
