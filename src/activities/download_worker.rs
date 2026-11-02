//! Background worker that monitors the system Downloads directory for active downloads.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::download::{downloads_dir, DownloadInfo, DownloadTracker};

#[derive(Clone, Debug, Default)]
pub struct DownloadBox {
    pub current: Option<DownloadInfo>,
    pub finished: Vec<DownloadInfo>,
}

pub type DownloadMailbox = Arc<Mutex<DownloadBox>>;

const POLL_INTERVAL: Duration = Duration::from_millis(750);

pub fn start(mailbox: DownloadMailbox, wake: fn()) {
    thread::Builder::new()
        .name("download-watcher".into())
        .spawn(move || {
            let mut tracker = DownloadTracker::default();
            loop {
                thread::sleep(POLL_INTERVAL);
                let Some(dir) = downloads_dir() else {
                    continue;
                };

                let (now, done) = tracker.scan(&dir);
                let mut changed = false;

                if let Ok(mut lock) = mailbox.lock() {
                    if lock.current != now {
                        lock.current = now;
                        changed = true;
                    }
                    if let Some(finished_item) = done {
                        lock.finished.push(finished_item);
                        changed = true;
                    }
                }

                if changed {
                    wake();
                }
            }
        })
        .expect("spawn download-watcher");
}
