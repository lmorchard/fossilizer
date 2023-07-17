use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{
    fs::File,
    io::{copy, Cursor},
};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use url::Url;

static DEFAULT_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub url: Url,
    pub destination: PathBuf,
}

impl DownloadTask {
    async fn execute(self) -> Result<DownloadTask> {
        // todo: download with progress narration? https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
        let client = reqwest::ClientBuilder::new().build().unwrap();
        let response = client.get(self.url.clone()).send().await?;

        let file_parent_path = self.destination.parent().ok_or(anyhow!("no parent path"))?;
        fs::create_dir_all(file_parent_path)?;

        let mut file = File::create(&self.destination)?;
        let mut content = Cursor::new(response.bytes().await?);

        copy(&mut content, &mut file)?;

        Ok(self)
    }
}

pub struct Downloader {
    // todo: make concurrency adjustable during run via channels?
    pub concurrency: usize,
    tasks: Arc<Mutex<VecDeque<DownloadTask>>>,
    new_task_notify: Arc<Notify>,
    queue_closed: Arc<Notify>,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new(DEFAULT_CONCURRENCY)
    }
}

impl Downloader {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency,
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            new_task_notify: Arc::new(Notify::new()),
            queue_closed: Arc::new(Notify::new()),
        }
    }

    pub fn queue(&self, task: DownloadTask) -> Result<(), Box<dyn Error>> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
        self.new_task_notify.notify_one();
        Ok(())
    }

    pub fn close(&self) -> Result<(), Box<dyn Error>> {
        self.queue_closed.notify_one();
        Ok(())
    }

    pub fn run(&self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let concurrency = self.concurrency;
        let tasks = self.tasks.clone();
        let new_task_notify = self.new_task_notify.clone();
        let queue_closed = self.queue_closed.clone();

        tokio::spawn(async move {
            let mut should_exit_when_empty = false;
            let mut workers = JoinSet::new();
            loop {
                // Check whether it's time to bail out when all known work is done
                {
                    let tasks = tasks.lock().or(Err(anyhow!("failed to lock tasks")))?;
                    if tasks.is_empty() && workers.is_empty() && should_exit_when_empty {
                        trace!("Exiting after last task");
                        break;
                    }
                }

                // Fire up workers for available tasks up to concurrency limit
                loop {
                    let mut tasks = tasks.lock().or(Err(anyhow!("failed to lock tasks")))?;
                    if tasks.is_empty() || workers.len() >= concurrency {
                        trace!(
                            "Not spawning worker - tasks.is_empty = {}; workers.len() = {}",
                            tasks.is_empty(),
                            workers.len()
                        );
                        break;
                    }
                    if let Some(task) = tasks.pop_front() {
                        trace!("Spawning worker for task - tasks.len() = {}; workers.len() = {} - {:?}", tasks.len(), workers.len(), task);
                        workers.spawn(task.execute());
                    }
                }

                // Wait for something important to happen...
                tokio::select! {
                    // todo: report progress via some channel
                    _ = workers.join_next(), if !workers.is_empty() => {
                        trace!("Worker done - workers.len() = {}", workers.len());
                    }
                    _ = new_task_notify.notified() => {
                        trace!("New task queued");
                    }
                    _ = queue_closed.notified() => {
                        trace!("Queue closed");
                        should_exit_when_empty = true;
                    }
                }

                // Yield, so we're less of a hot loop here
                tokio::task::yield_now().await;
            }
            anyhow::Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use std::env;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_downloadtask_execute_downloads_url() -> Result<()> {
        let base_path = generate_base_path();
        let mut server = mockito::Server::new();
        let (task, mock, expected_data) = generate_download_task(&base_path, &mut server);
        task.clone().execute().await?;
        mock.assert();
        let result_data = fs::read_to_string(task.destination)?;
        assert_eq!(result_data, expected_data);
        fs::remove_dir_all(base_path)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_producer_consumer_tasks() -> Result<()> {
        let base_path = generate_base_path();

        let mut server = mockito::Server::new();

        let task_count = 32;
        let mut test_downloads = Vec::new();
        for _ in 0..task_count {
            test_downloads.push(generate_download_task(&base_path, &mut server));
        }

        let tasks: Vec<DownloadTask> = test_downloads
            .iter()
            .map(|(task, _, _)| task.clone())
            .collect();

        let downloader = Downloader::default();
        let consumer = downloader.run();
        let producer = tokio::spawn(async move {
            for task in tasks {
                downloader
                    .queue(task)
                    .or(Err(anyhow!("downloader queue")))?;
                random_sleep(10, 100).await;
            }
            downloader
                .close()
                .or(Err(anyhow!("downloader close failed")))?;
            anyhow::Ok(())
        });

        let result = tokio::join!(consumer, producer,);
        result.0??;
        result.1??;

        for (task, mock, expected_data) in test_downloads {
            mock.assert();
            let result_data = fs::read_to_string(task.destination)?;
            assert_eq!(result_data, expected_data);
        }

        fs::remove_dir_all(base_path)?;

        Ok(())
    }

    async fn random_sleep(min: u64, max: u64) {
        let duration = {
            let mut rng = rand::thread_rng();
            Duration::from_millis(rng.gen_range(min..max))
        };
        sleep(duration).await;
    }

    fn generate_base_path() -> PathBuf {
        let rand_path: u16 = random();
        let base_path = env::temp_dir().join(format!("fossilizer-{rand_path}"));
        base_path
    }

    fn generate_download_task(
        base_path: &PathBuf,
        server: &mut mockito::ServerGuard,
    ) -> (DownloadTask, mockito::Mock, std::string::String) {
        let rand_path: u16 = random();

        let data = format!("task {rand_path} data");

        let url = Url::parse(&server.url())
            .unwrap()
            .join(format!("/task-{rand_path}").as_str())
            .unwrap();

        let destination = base_path.join(format!("tasks/task-{rand_path}.txt"));

        let server_mock = server
            .mock("GET", url.path())
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(data.clone())
            .create();

        let task = DownloadTask { url, destination };

        (task, server_mock, data)
    }
}
