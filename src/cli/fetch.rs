use anyhow::{anyhow, Result};
use reqwest::header;
use std::convert::From;
use std::error::Error;
use url::Url;

use fossilizer::activitystreams::{
    Activity, Actor, Attachments, IdOrObject, OrderedCollection, OrderedCollectionPage,
    OrderedItems, CONTENT_TYPE,
};
use fossilizer::{config, db, downloader};

#[derive(Debug, clap::Args)]
pub struct Args {
    /// List of ActivityPub outbox URLs to be fetched
    actor_urls: Vec<String>,
}

pub async fn command(args: &Args) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let media_path = config.media_path();
    let actor_urls = args.actor_urls.clone();

    // todo: build a single-threaded activity import background task?

    let media_downloader = downloader::Downloader::default();
    let media_download_manager = media_downloader.run();

    let actor_downloader = tokio::spawn(async move {
        // todo: support plain fediverse address via webfinger lookup - @lmorchard@hackers.town

        let mut count = 0;

        let mut ap_default_headers = reqwest::header::HeaderMap::new();
        ap_default_headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static(CONTENT_TYPE),
        );
        let ap_client = reqwest::ClientBuilder::new()
            .default_headers(ap_default_headers)
            .build()?;

        for actor_url in actor_urls {
            let actor_raw: serde_json::Value =
                ap_client.get(actor_url).send().await?.json().await?;
            // todo: watch for "request not signed" error here!

            {
                let conn = db::conn().or(Err(anyhow!("database connection failed")))?;
                let actors = db::actors::Actors::new(&conn);
                actors.import_actor(&actor_raw)?;
                trace!("imported actor {:?}", actor_raw);
            }

            trace!("outside actor import block");

            // todo: fetch media and adjust URLs in actor

            let actor = serde_json::from_value(actor_raw);
            trace!("parsed actor {:?}", actor);
            let actor: Actor = actor?;

            for attachment in actor.attachments() {
                let task = downloader::DownloadTask{ 
                    url: Url::parse(attachment.url.as_str())?,
                    destination: attachment.local_media_path(&media_path, &actor)?,
                };
                trace!("enqueue actor attachment {:?}", task);
                media_downloader.queue(task)?;
            }

            let outbox: OrderedCollection =
                ap_client.get(&actor.outbox).send().await?.json().await?;

            let mut page_url = Some(outbox.first);
            while page_url.is_some() {
                debug!("importing {:?}", page_url);
                let page: OrderedCollectionPage<serde_json::Value> = ap_client
                    .get(page_url.unwrap())
                    .send()
                    .await?
                    .json()
                    .await?;

                {
                    let conn = db::conn().or(Err(anyhow!("database connection failed")))?;
                    let activities = db::activities::Activities::new(&conn);
                    activities.import_collection(&page)?;
                }

                // todo: fetch media and adjust URLs in items
                for activity in page.ordered_items() {
                    let activity: Activity = serde_json::from_value(activity.clone())?;
                    if let IdOrObject::Object(object) = &activity.object {
                        for attachment in object.attachments() {
                            media_downloader.queue(downloader::DownloadTask{ 
                                url: Url::parse(attachment.url.as_str())?,
                                destination: attachment.local_media_path(&media_path, &actor)?,
                            })?;
                        }
                    }
                }

                page_url = page.next;

                count = count + 1;
                if count > 25 {
                    //break;
                }
            }
        }

        media_downloader.close()?;

        anyhow::Ok(())
    });

    actor_downloader.await??;
    media_download_manager.await??;

    Ok(())
}
