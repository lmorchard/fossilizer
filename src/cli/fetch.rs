use anyhow::Result;
use reqwest::header;
use std::convert::From;
use std::error::Error;

use fossilizer::activitystreams::{
    Activity, Actor, Attachments, OrderedCollection, OrderedCollectionPage, OrderedItems, IdOrObject,
    CONTENT_TYPE,
};
use fossilizer::{config, db};

#[derive(Debug, clap::Args)]
pub struct Args {
    /// List of ActivityPub outbox URLs to be fetched
    actor_urls: Vec<String>,
}

pub async fn command(args: &Args) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let media_path = config.media_path();

    let conn = db::conn()?;
    let actors = db::actors::Actors::new(&conn);
    let activities = db::activities::Activities::new(&conn);

    // todo: support plain fediverse address via webfinger lookup - @lmorchard@hackers.town

    let mut ap_default_headers = reqwest::header::HeaderMap::new();
    ap_default_headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static(CONTENT_TYPE),
    );
    let ap_client = reqwest::blocking::ClientBuilder::new()
        .default_headers(ap_default_headers)
        .build()?;

    // todo: need a threaded URL download queue? async queue?

    for actor_url in &args.actor_urls {
        let actor_raw: serde_json::Value = ap_client.get(actor_url).send()?.json()?;
        actors.import_actor(&actor_raw)?;
        // todo: fetch media and adjust URLs in actor

        let actor: Actor = serde_json::from_value(actor_raw)?;
        for attachment in actor.attachments() {
            debug!(
                "ATTACHMENT {:?} - {:?}",
                attachment.url,
                attachment.local_media_path(&media_path, &actor)
            );
        }

        let outbox: OrderedCollection = ap_client.get(&actor.outbox).send()?.json()?;

        let mut page_url = Some(outbox.first);
        while page_url.is_some() {
            debug!("importing {:?}", page_url);
            let page: OrderedCollectionPage<serde_json::Value> =
                ap_client.get(page_url.unwrap()).send()?.json()?;

            activities.import_collection(&page)?;

            // todo: fetch media and adjust URLs in items
            for activity in page.ordered_items() {
                let activity: Activity = serde_json::from_value(activity.clone())?;
                if let IdOrObject::Object(object) = &activity.object {
                    for attachment in object.attachments() {
                        debug!(
                            "OBJECT ATTACHMENT {:?} - {:?}",
                            attachment.url,
                            attachment.local_media_path(&media_path, &actor)
                        );
                    }
                }
            }

            page_url = page.next;
        }
    }

    Ok(())
}
