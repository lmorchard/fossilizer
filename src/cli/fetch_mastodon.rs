use anyhow::{anyhow, Result};
use megalodon;
use megalodon::megalodon::GetAccountStatusesInputOptions;
use std::default::Default;
use std::error::Error;
use url::Url;

use fossilizer::{
    activitystreams::{Activity, Actor, Attachment, Attachments},
    config, db, downloader,
};

#[derive(Debug, clap::Args)]

pub struct Args {}

pub async fn command(args: &Args) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let media_path = config.media_path();

    let media_downloader = downloader::Downloader::default();
    let media_download_manager = media_downloader.run();

    let mastodon_fetcher = tokio::spawn(async move {
        let access_token = config.mastodon_access_token.unwrap();
        let client = megalodon::generator(
            megalodon::SNS::Mastodon,
            String::from("https://hackers.town"),
            Some(access_token),
            None,
        );
        let account = client.verify_account_credentials().await?.json();

        let statuses = client
            .get_account_statuses(
                String::from(&account.id),
                Some(&GetAccountStatusesInputOptions {
                    limit: Some(100),
                    ..Default::default()
                }),
            )
            .await?
            .json();

        let conn = db::conn().or(Err(anyhow!("database connection failed")))?;
        let db_activities = db::activities::Activities::new(&conn);
        let db_actors = db::actors::Actors::new(&conn);
        let actors = db_actors.get_actors_by_id().unwrap();

        for status in statuses {
            db_activities.import(&status)?;

            let activity: Activity = status.into();
            let object = activity.object.object().unwrap();
            let actor_id: &String = activity.actor.id().unwrap();
            let actor = actors.get(actor_id).unwrap();

            for &attachment in &object.attachments() {
                media_downloader.queue(downloader::DownloadTask {
                    url: Url::parse(attachment.url.as_str())?,
                    destination: attachment.local_media_path(&media_path, &actor)?,
                })?;
            }
        }

        media_downloader.close()?;
        
        anyhow::Ok(())
    });

    mastodon_fetcher.await??;
    media_download_manager.await??;

    Ok(())
}
