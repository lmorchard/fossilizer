use anyhow::Result;
use reqwest::header;
use std::convert::From;
use std::error::Error;

use fossilizer::activitystreams::{Actor, OrderedCollection, OrderedCollectionPage, CONTENT_TYPE};
use fossilizer::db;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// List of ActivityPub outbox URLs to be fetched
    actor_urls: Vec<String>,
}

pub fn command(args: &Args) -> Result<(), Box<dyn Error>> {
    let conn = db::conn()?;
    let actors = db::actors::Actors::new(&conn);
    let activities = db::activities::Activities::new(&conn);

    // todo: support plain fediverse address - @lmorchard@hackers.town

    let mut ap_default_headers = reqwest::header::HeaderMap::new();
    ap_default_headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static(CONTENT_TYPE),
    );
    let ap_client = reqwest::blocking::ClientBuilder::new()
        .default_headers(ap_default_headers)
        .build()?;

    for actor_url in &args.actor_urls {
        let actor_raw: serde_json::Value = ap_client.get(actor_url).send()?.json()?;
        actors.import_actor(&actor_raw)?;
        // todo: fetch media and adjust URLs in actor

        let actor: Actor = serde_json::from_value(actor_raw)?;
        let outbox: OrderedCollection = ap_client.get(&actor.outbox).send()?.json()?;

        let mut page_url = Some(outbox.first);
        while page_url.is_some() {
            debug!("importing {:?}", page_url);
            let page: OrderedCollectionPage<serde_json::Value> = ap_client
                .get(page_url.ok_or("no page url")?)
                .send()?
                .json()?;

            activities.import_collection(&page)?;

            // todo: fetch media and adjust URLs in items

            page_url = page.next;
        }
    }

    Ok(())
}
