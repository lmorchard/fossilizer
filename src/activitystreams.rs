use anyhow::Result;
use megalodon;
use megalodon::entities::attachment::AttachmentType;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::path::PathBuf;
use url::Url;

pub static PUBLIC_ID: &str = "https://www.w3.org/ns/activitystreams#Public";
pub static CONTENT_TYPE: &str = "application/activity+json";

pub trait OrderedItems<TItem: Serialize> {
    fn ordered_items(&self) -> &Vec<TItem>;
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(rename = "type")]
    pub type_field: String,
    pub media_type: String,
    pub url: String,
    pub blurhash: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub name: Option<String>,
    pub summary: Option<String>,
}

impl Attachment {
    pub fn local_media_path(&self, dest_path: &PathBuf, actor: &Actor) -> Result<PathBuf> {
        let id_hash = &actor.id_hash();
        let attachment_url = Url::parse(&actor.id)?.join(&self.url)?;
        let attachment_path = attachment_url.path();

        Ok(PathBuf::new()
            .join(dest_path)
            .join(id_hash)
            .join(&attachment_path[1..]))
    }
}

impl From<megalodon::entities::Attachment> for Attachment {
    fn from(attachment: megalodon::entities::Attachment) -> Self {
        // todo: make this access less awkward? 😅
        let meta_original = attachment.meta.and_then(|meta| meta.original);
        let meta_original = meta_original.as_ref();
        let width = meta_original.and_then(|original| original.width);
        let height = meta_original.and_then(|original| original.height);

        /*
            Convert from Mastodon media type to mime-type as expected in ActivityPub
            https://docs.joinmastodon.org/entities/MediaAttachment/#type

            unknown = unsupported or unrecognized file type
            image = Static image
            gifv = Looping, soundless animation
            video = Video clip
            audio = Audio track

            these are very stupid mime-type conversions, mainly to make the static site templates happy
            should be okay, though, since we're not persisting this and will re-generate from raw JSON if we improve later
        */
        let media_type = match attachment.r#type {
            AttachmentType::Audio => "audio/mp3",
            AttachmentType::Image => "image/png",
            AttachmentType::Video => "video/mp4",
            AttachmentType::Gifv => "video/mp4",
            AttachmentType::Unknown => "unknown",
        }
        .to_string();

        Self {
            // todo use Image based on attachment.type?
            type_field: "Document".to_string(),
            url: attachment.url,
            summary: attachment.description,
            blurhash: attachment.blurhash,
            media_type,
            width,
            height,
            ..Default::default()
        }
    }
}

pub trait Attachments {
    fn attachments(&self) -> Vec<&Attachment>;
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
    pub published: chrono::DateTime<chrono::offset::Utc>,
    pub icon: Option<Attachment>,
    pub image: Option<Attachment>,
    pub public_key: Option<PublicKey>,
}

impl Actor {
    pub fn id_hash(&self) -> String {
        sha256::digest(self.id.as_str())
    }
}

impl Attachments for Actor {
    fn attachments(&self) -> Vec<&Attachment> {
        let mut attachments = Vec::new();
        if let Some(icon) = &self.icon {
            attachments.push(icon);
        }
        if let Some(image) = &self.image {
            attachments.push(image);
        }
        attachments
    }
}

impl From<serde_json::Value> for Actor {
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
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
    pub fn is_id(&self) -> bool {
        matches!(self, IdOrObject::Id(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, IdOrObject::Object(_))
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
    pub published: chrono::DateTime<chrono::offset::Utc>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub actor: IdOrObject<Actor>,
    pub object: IdOrObject<Object>,
}

impl Activity {
    pub fn is_public(&self) -> bool {
        let public_id = PUBLIC_ID.to_string();
        self.to.contains(&public_id) || self.cc.contains(&public_id)
    }
}

impl From<megalodon::entities::Status> for Activity {
    fn from(status: megalodon::entities::Status) -> Self {
        // todo: better error handling here?
        let uri = url::Url::parse(status.uri.as_str()).unwrap();

        let mut to = Vec::new();
        if let megalodon::entities::StatusVisibility::Public = status.visibility {
            to.push(PUBLIC_ID.to_string());
        };

        // todo: better account for polls, retoots, etc?
        let activity_type_field = if status.reblog.is_some() {
            "Announce"
        } else {
            "Create"
        }
        .to_string();

        let object = if status.reblog.is_some() {
            IdOrObject::Id(status.reblog.unwrap().uri)
        } else {
            IdOrObject::Object(Object {
                id: status.uri.clone(),
                url: status.url.or_else(|| Some(status.uri.clone())).unwrap(),
                type_field: "Note".to_string(),
                published: status.created_at,
                content: Some(status.content),
                summary: if !status.spoiler_text.is_empty() {
                    Some(status.spoiler_text)
                } else {
                    None
                },
                attachment: status
                    .media_attachments
                    .iter()
                    .map(|media_attachment| Attachment::from(media_attachment.clone()))
                    .collect(),
                ..Default::default()
            })
        };

        Self {
            id: format!("{}/activity", status.uri),
            type_field: activity_type_field,
            published: status.created_at,
            to,
            // cc
            actor: IdOrObject::Id({
                // hack: this is some grungy butchery to derive an activitypub actor URL for mastodon
                let mut uri = uri;
                uri.set_path(format!("/users/{}", status.account.acct).as_str());
                uri.into()
            }),
            object,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub url: String,
    pub published: chrono::DateTime<chrono::offset::Utc>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub in_reply_to: Option<String>,
    pub tag: Vec<Tag>,
    pub attachment: Vec<Attachment>,
}

impl Attachments for Object {
    fn attachments(&self) -> Vec<&Attachment> {
        let mut attachments = Vec::new();
        for attachment in &self.attachment {
            attachments.push(attachment);
        }
        for tag in &self.tag {
            if let Some(icon) = &tag.icon {
                attachments.push(icon);
            }
        }
        attachments
    }
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

impl Attachments for Tag {
    fn attachments(&self) -> Vec<&Attachment> {
        let mut attachments = Vec::new();
        if let Some(icon) = &self.icon {
            attachments.push(icon);
        }
        attachments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::path::Path;
    use std::str::FromStr;
    use test_log::test;

    const JSON_OUTBOX: &str = include_str!("./resources/test/outbox.json");
    const JSON_ACTIVITY_WITH_EMOJI: &str =
        include_str!("./resources/test/activity-with-emoji.json");
    const JSON_ACTIVITY_WITH_ATTACHMENT: &str =
        include_str!("./resources/test/activity-with-attachment.json");
    const JSON_REMOTE_ACTOR: &str = include_str!("./resources/test/actor-remote.json");
    const JSON_MASTODON_STATUS_WITH_ATTACHMENT: &str =
        include_str!("./resources/test/mastodon-status-with-attachment.json");

    #[test]
    fn test_from_megalodon_status_to_activity() -> Result<()> {
        let status: megalodon::entities::Status =
            serde_json::from_str(JSON_MASTODON_STATUS_WITH_ATTACHMENT).unwrap();
        let activity: Activity = status.clone().into();

        trace!("STATUS {:?}", status);
        trace!("ACTIVITY {:?}", activity);

        assert_eq!(
            activity.id,
            "https://hackers.town/users/lmorchard/statuses/110726017288384411/activity"
        );

        assert_eq!(
            activity.actor.id().unwrap(),
            "https://hackers.town/users/lmorchard"
        );

        let object = activity.object.object().unwrap();
        assert_eq!(
            object.id,
            "https://hackers.town/users/lmorchard/statuses/110726017288384411"
        );

        let expected_published: chrono::DateTime<chrono::offset::Utc> =
            chrono::DateTime::from_str("2023-07-16T22:02:21.535Z").unwrap();
        assert_eq!(activity.published, expected_published);
        assert_eq!(object.published, expected_published);

        Ok(())
    }

    #[test]
    fn test_remote_actor_attachments() -> Result<(), Box<dyn Error>> {
        let actor: Actor = serde_json::from_str(JSON_REMOTE_ACTOR)?;
        let icon = actor.icon.as_ref().unwrap();
        let image = actor.image.as_ref().unwrap();
        let media_path = PathBuf::new().join("media");
        assert_eq!(
            icon.local_media_path(&media_path, &actor)?,
            Path::new("media/acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184/system/accounts/avatars/000/136/533/original/1a8c651efe14fcd6.png"),
        );
        assert_eq!(
            image.local_media_path(&media_path, &actor)?,
            Path::new("media/acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184/system/accounts/headers/000/136/533/original/60af00520bbf3704.jpg"),
        );
        Ok(())
    }

    #[test]
    fn test_activity_with_attachments() -> Result<(), Box<dyn Error>> {
        let actor: Actor = serde_json::from_str(JSON_REMOTE_ACTOR)?;
        let activity: Activity = serde_json::from_str(JSON_ACTIVITY_WITH_ATTACHMENT)?;
        let media_path = PathBuf::new().join("media");

        let object = activity.object;
        assert!(object.is_object());
        let object = object.object().unwrap();

        let result: Vec<PathBuf> = object
            .attachments()
            .iter()
            .map(|attachment| attachment.local_media_path(&media_path, &actor).unwrap())
            .collect();

        let expected = vec![
            Path::new("media/acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184/users/media_attachments/files/002/337/518/original/ebbb5d342877102f.jpg"),
            Path::new("media/acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184/users/media_attachments/files/002/337/520/original/63a81769839a7ef6.jpg"),
            Path::new("media/acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184/system/custom_emojis/images/000/043/882/original/5cd6640bb919cf64.png"),
        ];

        assert_eq!(result, expected);

        Ok(())
    }

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
