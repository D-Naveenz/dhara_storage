use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, info};

use crate::error::StorageError;

/// High-level change type reported by directory watchers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageChangeType {
    /// A file-system entry was created.
    Created,
    /// A file-system entry was deleted.
    Deleted,
    /// A file-system entry was modified in place.
    Modified,
    /// A file-system entry was moved or renamed.
    Relocated,
}

/// A debounced change notification produced by a directory watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageChangeEvent {
    /// The kind of change that was observed.
    pub change_type: StorageChangeType,
    /// The current path of the affected entry.
    pub path: PathBuf,
    /// The previous path when the change was a relocation.
    pub previous_path: Option<PathBuf>,
    /// The time at which the debounced event was emitted.
    pub observed_at: SystemTime,
}

/// Configuration for [`DirectoryWatchHandle`] creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageWatchConfig {
    /// Whether to watch the directory tree recursively.
    pub recursive: bool,
    /// The debounce window used to coalesce bursts of low-level events.
    pub debounce_window: Duration,
}

impl Default for StorageWatchConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            debounce_window: Duration::from_millis(500),
        }
    }
}

/// A live directory watcher with a blocking event receiver.
///
/// Dropping the handle stops the underlying watcher and joins the debounce worker.
pub struct DirectoryWatchHandle {
    receiver: Receiver<StorageChangeEvent>,
    watcher: Option<RecommendedWatcher>,
    worker: Option<JoinHandle<()>>,
}

impl DirectoryWatchHandle {
    /// Start watching a directory for debounced change events.
    pub fn watch(path: impl AsRef<Path>, config: StorageWatchConfig) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        info!(
            target: "dhara_storage::watch",
            path = %path.display(),
            recursive = config.recursive,
            debounce_window_ms = config.debounce_window.as_millis() as u64,
            "starting directory watcher"
        );
        if !path.exists() {
            return Err(StorageError::NotFound { path });
        }
        if !path.is_dir() {
            return Err(StorageError::NotADirectory { path });
        }

        let (raw_tx, raw_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |result| {
            let _ = raw_tx.send(result);
        })
        .map_err(|err| StorageError::watch("create watcher", err.to_string()))?;

        watcher
            .watch(
                &path,
                if config.recursive {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                },
            )
            .map_err(|err| StorageError::watch("watch directory", err.to_string()))?;

        let worker =
            thread::spawn(move || debounce_worker(raw_rx, event_tx, config.debounce_window));

        Ok(Self {
            receiver: event_rx,
            watcher: Some(watcher),
            worker: Some(worker),
        })
    }

    /// Block until the next debounced event arrives.
    pub fn recv(&self) -> Result<StorageChangeEvent, StorageError> {
        debug!(target: "dhara_storage::watch", "waiting for watcher event");
        self.receiver
            .recv()
            .map_err(|_| StorageError::watch("receive watcher event", "watcher closed"))
    }

    /// Wait for the next event for at most the provided duration.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<StorageChangeEvent>, StorageError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(StorageError::watch(
                "receive watcher event",
                "watcher closed",
            )),
        }
    }

    /// Attempt to receive an event without blocking.
    pub fn try_recv(&self) -> Result<Option<StorageChangeEvent>, StorageError> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(StorageError::watch(
                "receive watcher event",
                "watcher closed",
            )),
        }
    }
}

impl Drop for DirectoryWatchHandle {
    fn drop(&mut self) {
        info!(target: "dhara_storage::watch", "stopping directory watcher");
        self.watcher.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn debounce_worker(
    raw_rx: Receiver<notify::Result<Event>>,
    event_tx: mpsc::Sender<StorageChangeEvent>,
    debounce_window: Duration,
) {
    while let Ok(first) = raw_rx.recv() {
        let mut batch = Vec::new();
        if let Ok(first) = first {
            batch.push(first);
        }

        loop {
            match raw_rx.recv_timeout(debounce_window) {
                Ok(next) => {
                    if let Ok(next) = next {
                        batch.push(next);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    flush_batch(&batch, &event_tx);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    flush_batch(&batch, &event_tx);
                    return;
                }
            }
        }
    }
}

fn flush_batch(batch: &[Event], event_tx: &mpsc::Sender<StorageChangeEvent>) {
    let mut emitted = Vec::<StorageChangeEvent>::new();
    for event in batch {
        for mapped in map_notify_event(event) {
            if !emitted.iter().any(|existing| existing == &mapped) {
                emitted.push(mapped);
            }
        }
    }

    for event in emitted {
        debug!(
            target: "dhara_storage::watch",
            change_type = ?event.change_type,
            path = %event.path.display(),
            previous_path = event.previous_path.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
            "emitting debounced watcher event"
        );
        if event_tx.send(event).is_err() {
            break;
        }
    }
}

fn map_notify_event(event: &Event) -> Vec<StorageChangeEvent> {
    let observed_at = SystemTime::now();
    match &event.kind {
        EventKind::Create(_) => event
            .paths
            .iter()
            .cloned()
            .map(|path| StorageChangeEvent {
                change_type: StorageChangeType::Created,
                path,
                previous_path: None,
                observed_at,
            })
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .iter()
            .cloned()
            .map(|path| StorageChangeEvent {
                change_type: StorageChangeType::Deleted,
                path,
                previous_path: None,
                observed_at,
            })
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() >= 2 => {
            vec![StorageChangeEvent {
                change_type: StorageChangeType::Relocated,
                previous_path: Some(event.paths[0].clone()),
                path: event.paths[1].clone(),
                observed_at,
            }]
        }
        EventKind::Modify(ModifyKind::Name(_)) if event.paths.len() >= 2 => {
            vec![StorageChangeEvent {
                change_type: StorageChangeType::Relocated,
                previous_path: Some(event.paths[0].clone()),
                path: event.paths[1].clone(),
                observed_at,
            }]
        }
        EventKind::Modify(_) => event
            .paths
            .iter()
            .cloned()
            .map(|path| StorageChangeEvent {
                change_type: StorageChangeType::Modified,
                path,
                previous_path: None,
                observed_at,
            })
            .collect(),
        _ => Vec::new(),
    }
}
