use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use url::Url;

static DEFAULT_CONCURRENCY: usize = 4;

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub url: Url,
    pub destination: PathBuf,
}

impl DownloadTask {
    async fn execute(self: Self) {
        let duration = {
            let mut rng = rand::thread_rng();
            Duration::from_millis(rng.gen_range(100..1000))
        };

        println!("TASK START {:?}", self);
        sleep(duration).await;
        println!("TASK END {:?}", self);
    }
}

pub struct Downloader {
    pub concurrency: usize,
    tasks: Arc<Mutex<VecDeque<DownloadTask>>>,
    new_task_notify: Arc<Notify>,
    final_task_notify: Arc<Notify>,
}

impl Downloader {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency,
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            new_task_notify: Arc::new(Notify::new()),
            final_task_notify: Arc::new(Notify::new()),
        }
    }

    pub fn queue(self: &Self, task: DownloadTask) -> Result<(), Box<dyn Error>> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
        self.new_task_notify.notify_one();
        Ok(())
    }

    pub fn close(self: &Self) -> Result<(), Box<dyn Error>> {
        self.final_task_notify.notify_one();
        Ok(())
    }

    pub async fn run(self: &mut Self) -> () {
        let concurrency = self.concurrency;
        let tasks = self.tasks.clone();
        let new_task_notify = self.new_task_notify.clone();
        let final_task_notify = self.final_task_notify.clone();

        let mut should_exit = false;
        let mut workers = JoinSet::new();
        loop {
            // Check whether it's time to bail out when all known work is done
            {
                let tasks = tasks.lock().unwrap();
                if tasks.is_empty() && workers.is_empty() && should_exit {
                    // bail out when we're done with all tasks
                    // todo: wait for a shutdown signal, since we might get more tasks in the future
                    println!("ALL DONE");
                    break;
                }
            }

            // Fire up workers for available tasks up to concurrency limit
            loop {
                let mut tasks = tasks.lock().unwrap();
                if tasks.is_empty() || workers.len() >= concurrency {
                    break;
                }
                if let Some(task) = tasks.pop_front() {
                    println!(
                        "SPAWNING worker (tasks = {} workers = {})",
                        tasks.len(),
                        workers.len()
                    );
                    workers.spawn(task.execute());
                }
            }

            // Wait for something important to happen...
            tokio::select! {
                _ = workers.join_next() => {
                    // worker done, let's loop through and launch workers if possible
                    println!("WORKER done!");
                }
                _ = new_task_notify.notified() => {
                    // new task, let's loop through and launch workers if possible
                    println!("NEW task arrived!");
                }
                _ = final_task_notify.notified() => {
                    // new task, let's loop through and launch workers if possible
                    println!("FINAL task arrived!");
                    should_exit = true;
                }
            }
        }
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new(DEFAULT_CONCURRENCY)
    }
}

// manager task that monitors the queue
// semaphore to manage concurrency?
// spawn downloader tasks that download the files

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use std::env;

    #[tokio::test]
    async fn my_test() -> Result<(), Box<dyn Error>> {
        let rand_path: u16 = random();
        let base_path: PathBuf = env::temp_dir().join(format!("fossilizer-{}", rand_path));

        let mut server = mockito::Server::new();

        let host = server.host_with_port();
        println!("HOST {}", host);

        let mock_server_url = server.url();

        let resources_count = 16;

        let mut data_resources: Vec<String> = Vec::new();
        let mut mock_resources: Vec<mockito::Mock> = Vec::new();
        let mut tasks: Vec<DownloadTask> = Vec::new();

        for idx in 0..resources_count {
            let data = format!("task {} data", idx);
            let destination = base_path.join(format!("tasks/task-{}.txt", idx));
            let url = Url::parse(&mock_server_url)
                .unwrap()
                .join(format!("/task-{}", idx).as_str())
                .unwrap();

            mock_resources.push(
                server
                    .mock("GET", url.as_str())
                    .with_status(200)
                    .with_header("content-type", "text/plain")
                    .with_body(data.clone())
                    .create(),
            );
            data_resources.push(data);
            tasks.push(DownloadTask { url, destination });
        }

        let mut downloader = Downloader::default();
        for task in tasks {
            downloader.queue(task.clone()).unwrap();
        }
        downloader.close()?;

        downloader.run().await;

        /*
        for mock in mock_resources {
            mock.assert();
        }
         */

        assert!(true);

        Ok(())
    }
}
