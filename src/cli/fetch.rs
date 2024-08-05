use anyhow::{anyhow, Result};
use chrono::offset::Utc;
use chrono::DateTime;
use reqwest::header;
use ring::digest::{Context, Digest, SHA256};
use ring::{rand, rsa, signature};
use std::convert::From;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use url::Url;
use std::{thread, time};

use base64::{engine::general_purpose, Engine as _};
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

    let key_id = "https://toot.lmorchard.com/users/lmorchard#main-key";

    let mut private_key_filepath = PathBuf::new();
    // TODO: convert from legacy RSA PKCS1 to PKCS8?
    //private_key_filepath.push("./lmorchard-private-key");
    private_key_filepath.push("./lmorchard-private-key.pem");
    let private_key: Vec<u8> = fs::read(private_key_filepath)?;
    let private_key = pem::parse(private_key)?;
    let private_key = private_key.contents();

    let key_pair = rsa::KeyPair::from_pkcs8(&private_key).expect("failed key parse");

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
            // TODO fix error handling
            let fetch_result = get_signed(&actor_url, key_id, &key_pair, &ap_client)
                .await
                .unwrap(); //?;

            debug!("RESULT {:?}", fetch_result.status());

            let actor_raw: serde_json::Value = fetch_result.json().await?;
            debug!("RERESULT {:?}", actor_raw);

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
                let task = downloader::DownloadTask {
                    url: Url::parse(attachment.url.as_str())?,
                    destination: attachment.local_media_path(&media_path, &actor)?,
                };
                trace!("enqueue actor attachment {:?}", task);
                media_downloader.queue(task)?;
            }

            /*
            let outbox: OrderedCollection =
                ap_client.get(&actor.outbox).send().await?.json().await?;
             */
            let outbox: OrderedCollection =
                get_signed(&actor.outbox, key_id, &key_pair, &ap_client)
                    .await.unwrap() //?
                    .json()
                    .await?;

            let mut page_url = Some(outbox.first);
            while page_url.is_some() {
                debug!("importing {:?}", page_url);
                /*
                let page: OrderedCollectionPage<serde_json::Value> = ap_client
                    .get(page_url.unwrap())
                    .send()
                    .await?
                    .json()
                    .await?;
                 */
                let fetch_result = get_signed(&page_url.unwrap(), key_id, &key_pair, &ap_client)
                    .await
                    .unwrap();
                let fetch_json: serde_json::Value = fetch_result.json().await?;
                debug!("PAGE JSON {:?}", fetch_json);
                let page: OrderedCollectionPage<serde_json::Value> = serde_json::from_value(fetch_json)?;
                
                //let page: OrderedCollectionPage<serde_json::Value> = fetch_result.json().await?;

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
                            media_downloader.queue(downloader::DownloadTask {
                                url: Url::parse(attachment.url.as_str())?,
                                destination: attachment.local_media_path(&media_path, &actor)?,
                            })?;
                        }
                    }
                }

                thread::sleep(time::Duration::from_millis(1000));

                page_url = page.next;
                debug!("NEXT {:?}", page_url);

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

async fn get_signed(
    url: &String,
    key_id: &str,
    key_pair: &rsa::KeyPair,
    ap_client: &reqwest::Client,
) -> Result<reqwest::Response, Box<dyn Error>> {
    debug!("GET SIGNED {:?}", url);
    let now: DateTime<Utc> = Utc::now();
    let date = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let method = "get";
    let url_parsed = Url::parse(url.as_str())?;
    let host = url_parsed.host_str().unwrap();
    let path = url_parsed.path();
    let signature_header =
        generate_request_signature(key_id, key_pair, method, path, host, &date).unwrap();
    let fetch_request = ap_client
        .get(url)
        .header("Date", date)
        .header("Host", host)
        .header("Signature", signature_header)
        .build()
        .expect("bad request somehow?");
    let fetch_result = ap_client.execute(fetch_request).await?;
    Ok(fetch_result)
}

fn generate_request_signature(
    key_id: &str,
    key_pair: &rsa::KeyPair,
    method: &str,
    path: &str,
    host: &str,
    date: &String,
) -> Result<String, Box<dyn Error>> {
    let algorithm = "rsa-sha256";
    let headers_to_sign: String = "(request-target) host date".into();

    let mut string_to_sign: Vec<String> = Vec::new();
    string_to_sign.push(format!("(request-target): {} {}", method, path).into());
    string_to_sign.push(format!("host: {}", host).into());
    string_to_sign.push(format!("date: {}", date).into());
    let string_to_sign = string_to_sign.join("\n");

    let rng = rand::SystemRandom::new();
    let mut signature = vec![0; key_pair.public().modulus_len()];
    key_pair
        .sign(
            &signature::RSA_PKCS1_SHA256,
            &rng,
            string_to_sign.as_bytes(),
            &mut signature,
        )
        .expect("signing failed");

    let signature = general_purpose::STANDARD.encode(&signature);
    let signature_header = format!(
        r#"keyId="{}",algorithm="{}",headers="{}",signature="{}""#,
        key_id, algorithm, headers_to_sign, signature
    );
    Ok(signature_header)
}
