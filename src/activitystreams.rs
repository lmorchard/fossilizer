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
    pub object: serde_json::Value,
}

pub type OutboxWithActivities = Outbox<Activity>;

pub type OutboxWithValues = Outbox<serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

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

    const JSON_OUTBOX: &str = r#"
    {
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": "outbox.json",
        "type": "OrderedCollection",
        "totalItems": 1,
        "orderedItems": [    
            {
                "id": "https://mastodon.social/users/lmorchard/statuses/55864/activity",
                "type": "Create",
                "actor": "https://mastodon.social/users/lmorchard",
                "published": "2016-11-01T19:18:43Z",
                "to": ["https://www.w3.org/ns/activitystreams#Public"],
                "cc": ["https://mastodon.social/users/lmorchard/followers"],
                "object": {
                    "id": "https://mastodon.social/users/lmorchard/statuses/55864",
                    "type": "Note",
                    "summary": null,
                    "inReplyTo": null,
                    "published": "2016-11-01T19:18:43Z",
                    "url": "https://mastodon.social/@lmorchard/55864",
                    "attributedTo": "https://mastodon.social/users/lmorchard",
                    "to": ["https://www.w3.org/ns/activitystreams#Public"],
                    "cc": ["https://mastodon.social/users/lmorchard/followers"],
                    "sensitive": false,
                    "atomUri": "tag:mastodon.social,2016-11-01:objectId=55864:objectType=Status",
                    "inReplyToAtomUri": null,
                    "conversation": null,
                    "content": "\u003cp\u003eHello world!\u003c/p\u003e",
                    "contentMap": { "en": "\u003cp\u003eHello world!\u003c/p\u003e" },
                    "attachment": [],
                    "tag": [],
                    "replies": {
                        "id": "https://mastodon.social/users/lmorchard/statuses/55864/replies",
                        "type": "Collection",
                        "first": {
                            "type": "CollectionPage",
                            "next": "https://mastodon.social/users/lmorchard/statuses/55864/replies?only_other_accounts=true\u0026page=true",
                            "partOf": "https://mastodon.social/users/lmorchard/statuses/55864/replies",
                            "items": []
                        }
                    }
                },
                "signature": {
                    "type": "RsaSignature2017",
                    "creator": "https://mastodon.social/users/lmorchard#main-key",
                    "created": "2023-01-15T04:14:32Z",
                    "signatureValue": "UYcMjb8l0j9zol/Ljjaxo+aEylaAAAD+Iw6hpohFr9zxb56K9j4fIWDVqYwnHX1JR7a92R6Ybn9dobonXzHQo/oKviIJhwxDW6qkqvYHV3iOZG3raA9wGa6JLDPwdl1MYdpuLmZneEo4BtHHLVsj3lGbNPjFjMGRbkmyczV37Sz/Hm6fqLzLRCfBOAC1GY83RsV04C25asZrPZTRNUDoU94bni81dubUR8pZYNH0OVSLAJH02B+N0YmP/ti3dyg8XUXLIXM6u1eW1IIU0L+e459BhLhTNvVH/ISnHn/n1QMZDuQ1G9VBU0NSdt7jnTrykdd2yv7pNbRxJ7HrvUtnkg=="
                }
            },
            {
                "id": "https://mastodon.social/users/lmorchard/statuses/237393/activity",
                "type": "Create",
                "actor": "https://mastodon.social/users/lmorchard",
                "published": "2016-11-27T23:56:46Z",
                "to": ["https://www.w3.org/ns/activitystreams#Public"],
                "cc": ["https://mastodon.social/users/lmorchard/followers"],
                "object": {
                    "id": "https://mastodon.social/users/lmorchard/statuses/237393",
                    "type": "Note",
                    "summary": null,
                    "inReplyTo": null,
                    "published": "2016-11-27T23:56:46Z",
                    "url": "https://mastodon.social/@lmorchard/237393",
                    "attributedTo": "https://mastodon.social/users/lmorchard",
                    "to": ["https://www.w3.org/ns/activitystreams#Public"],
                    "cc": ["https://mastodon.social/users/lmorchard/followers"],
                    "sensitive": false,
                    "atomUri": "tag:mastodon.social,2016-11-27:objectId=237393:objectType=Status",
                    "inReplyToAtomUri": null,
                    "conversation": null,
                    "content": "\u003cp\u003eI should post here more, but I\u0026#39;ve been procrastinating installing something GNU-Social-ish on my own domain\u003c/p\u003e",
                    "contentMap": {
                        "en": "\u003cp\u003eI should post here more, but I\u0026#39;ve been procrastinating installing something GNU-Social-ish on my own domain\u003c/p\u003e"
                    },
                    "attachment": [],
                    "tag": [],
                    "replies": {
                        "id": "https://mastodon.social/users/lmorchard/statuses/237393/replies",
                        "type": "Collection",
                        "first": {
                            "type": "CollectionPage",
                            "next": "https://mastodon.social/users/lmorchard/statuses/237393/replies?only_other_accounts=true\u0026page=true",
                            "partOf": "https://mastodon.social/users/lmorchard/statuses/237393/replies",
                            "items": []
                        }
                    }
                },
                "signature": {
                    "type": "RsaSignature2017",
                    "creator": "https://mastodon.social/users/lmorchard#main-key",
                    "created": "2023-01-15T04:14:32Z",
                    "signatureValue": "BByb/GjzI/JAYONGumaySNWvwyyRX9NacsPlgppOb2MTAp6Qy1wUPA58vCeaa6zd5ItRBSYNJtx9TT7UVnpjNihlEZ4HEGA4IkHi1f8J9v6pNVD+5RfP8q5GTnlGqPul68dSKh/FOdjCwjoQiaGz/llOBcq+8lGbtdEw018cvcFDAaoxznJ8iIjtIj5IbmrRGwAgEZJFyB3F4jwn0sCve8gJL6x6qZpAb/nFXGgpoEcozBu0Hzb9yBIXrQOARcCMw54eQX2MrqqBvuzF1Y4w+uDuTu8OmFQS7RhBOqumJciokGkN4ZB/jRTTnNUZ4sEhklRbJ7Zq0+QAK42bod1WWA=="
                }
            }
        ]
    }
    "#;
}