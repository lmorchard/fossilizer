use anyhow::Result;
use megalodon;
use megalodon::entities::Status;
use megalodon::megalodon::GetAccountStatusesInputOptions;
use rusqlite::Connection;

use std::default::Default;

use std::path::PathBuf;
use url::Url;

use crate::mastodon::instance::InstanceConfig;
use crate::{
    activitystreams::{Activity, Attachments},
    db, downloader,
};
pub struct Fetcher {
    conn: Connection,
    instance: String,
    instance_config: InstanceConfig,
    media_path: PathBuf,
    page: u32,
    max: u32,
    incremental: bool,
}

impl Fetcher {
    pub fn new(
        conn: Connection,
        instance: String,
        instance_config: InstanceConfig,
        media_path: PathBuf,
        page: u32,
        max: u32,
        incremental: bool,
    ) -> Self {
        Self {
            conn,
            instance,
            instance_config,
            media_path,
            page,
            max,
            incremental,
        }
    }

    pub async fn fetch(&mut self) -> Result<()> {
        let max = self.max;
        let page = self.page;
        let incremental: bool = self.incremental;
        let media_path = self.media_path.clone();
        let instance = self.instance.clone();

        let media_downloader = downloader::Downloader::default();
        let media_download_manager = media_downloader.run();

        let access_token = self.instance_config.access_token.as_ref().unwrap().clone();

        let client = megalodon::generator(
            megalodon::SNS::Mastodon,
            format!("https://{instance}"),
            Some(access_token),
            None,
        );

        let account = client.verify_account_credentials().await?.json();
        trace!("Fetched account {:?}", account);
        info!("Fetching statuses for account {}", account.url);
        // todo: update actor from mastodon data

        let conn = &self.conn;
        let db_activities = db::activities::Activities::new(&conn);
        let db_actors = db::actors::Actors::new(&conn);
        let actors = db_actors.get_actors_by_id().unwrap();

        let mut keep_fetching = true;
        let mut fetched_count = 0;
        let mut current_fetch_options = GetAccountStatusesInputOptions {
            limit: Some(page),
            ..Default::default()
        };

        // todo: should this loop be async to cooperate with the media downloader better? or is it fine as is?
        while keep_fetching && fetched_count < max {
            let statuses_resp = client
                .get_account_statuses(String::from(&account.id), Some(&current_fetch_options))
                .await?;

            let statuses_and_activities: Vec<(Status, Activity)> = statuses_resp
                .json()
                .iter()
                .map(|status| (status.clone(), status.clone().into()))
                .collect();

            if statuses_and_activities.is_empty() {
                info!("Reached the end of available activities");
                break;
            }

            if incremental {
                let activity_ids: Vec<String> = statuses_and_activities
                    .iter()
                    .map(|item| item.1.id.clone())
                    .collect();
                let existing_activities_count =
                    db_activities.count_activities_by_ids(&activity_ids)?;
                if existing_activities_count > 0 {
                    keep_fetching = false;
                }
            }

            for (status, activity) in statuses_and_activities {
                trace!("Importing status {:?}", status.url);
                db_activities.import(&status)?;
                fetched_count = fetched_count + 1;
                current_fetch_options.max_id = Some(status.id);

                // If this is a note, import any attachments
                if activity.object.is_object() {
                    let object = activity.object.object().unwrap();
                    let actor_id: &String = activity.actor.id().unwrap();
                    let actor = actors.get(actor_id).unwrap();

                    trace!("Importing {} attachments", &object.attachments().len());
                    for &attachment in &object.attachments() {
                        media_downloader.queue(downloader::DownloadTask {
                            url: Url::parse(attachment.url.as_str())?,
                            destination: attachment.local_media_path(&media_path, &actor)?,
                        })?;
                    }
                }
            }

            info!("Fetched {fetched_count} (of {max} max)...");
            if !keep_fetching {
                info!("Stopping incremental fetch after catching up to imported activities");
            }
        }

        // Signal that we're done enqueueing and wait for any remaining downloads to finish
        media_downloader.close()?;
        media_download_manager.await??;

        Ok(())
    }
}
