use anyhow::Result;
use reqwest::header;
use std::convert::From;
use std::error::Error;

use fossilizer::db;

static ACTIVITYSTREAMS_CONTENT_TYPE: &str = "application/activity+json";

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
        header::HeaderValue::from_static(ACTIVITYSTREAMS_CONTENT_TYPE),
    );
    let ap_client = reqwest::blocking::ClientBuilder::new()
        .default_headers(ap_default_headers)
        .build()?;

    for actor_url in &args.actor_urls {
        let actor_raw: serde_json::Value = ap_client.get(actor_url).send()?.json()?;
        actors.import_actor(&actor_raw)?;

        let actor: fossilizer::activitystreams::Actor = serde_json::from_value(actor_raw)?;

        let outbox_raw: serde_json::Value = ap_client.get(&actor.outbox).send()?.json()?;

        let outbox: fossilizer::activitystreams::OrderedCollection =
            serde_json::from_value(outbox_raw)?;

        let mut page_url = Some(outbox.first);
        while page_url.is_some() {
            debug!("importing {:?}", page_url);

            let page_raw: serde_json::Value = ap_client
                .get(page_url.ok_or("no page url")?)
                .send()?
                .json()?;

            let page: fossilizer::activitystreams::OrderedCollectionPage<serde_json::Value> =
                serde_json::from_value(page_raw)?;

            activities.import_collection(&page)?;

            // todo: fetch media and adjust URLs in items

            page_url = page.next;
        }
    }

    Ok(())
}
