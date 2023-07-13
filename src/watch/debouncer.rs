//! Debouncing logic for [`watcher`](super::watcher) events, as these can occur very frequently.
//!
//! This _throttles_ said events. If an event is received, it is first put into a hash map,
//! together with the timestamp of the arrival. Any event of the same type that is received, in the
//! meantime, is simply dropped.
//!
//! On a regular basis, the hash map is checked for any "expired" events, meaning events that
//! passed a time threshold from the point they were received until now. These are taken out of the
//! map and send over a channel to the receiver.

use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use color_eyre::eyre::Result;
use flume::Selector;
use tracing::{debug, trace};

use super::{watcher, ChangeType};

struct EventDebouncer {
    handle: watcher::Handle,
    tx: flume::Sender<ChangeType>,
    debounce: Duration,
    shutdown: flume::Receiver<()>,
    changes: HashMap<ChangeType, Instant>,
}

pub struct Handle {
    shutdown: flume::Sender<()>,
    thread: thread::JoinHandle<watcher::Handle>,
    receiver: flume::Receiver<ChangeType>,
}

impl Handle {
    pub fn shutdown(self) -> watcher::Handle {
        self.shutdown.send(()).ok();
        self.thread.join().expect("task to shut down properly")
    }

    pub fn receiver(&self) -> &flume::Receiver<ChangeType> {
        &self.receiver
    }
}

pub fn debounce(watcher: watcher::Handle, debounce: Duration) -> Result<Handle> {
    let (change_tx, change_rx) = flume::bounded(super::CHANNEL_SIZE);
    let (shutdown_tx, shutdown_rx) = flume::bounded(0);

    let mut debouncer = EventDebouncer {
        handle: watcher,
        tx: change_tx,
        debounce,
        shutdown: shutdown_rx,
        changes: HashMap::new(),
    };

    let task = thread::spawn(move || {
        loop {
            let res = Selector::new()
                .recv(&debouncer.shutdown, |_| None)
                .recv(debouncer.handle.receiver(), |change| change.ok())
                .wait_timeout(Duration::from_millis(500));

            match res {
                // Got new FS event, just store it
                Ok(Some(change)) => {
                    debouncer.changes.entry(change).or_insert_with(Instant::now);
                }
                // Shutdown signal, or event channel closed
                Ok(None) => break,
                // Timeout, collect expired changes and send
                Err(_) => {
                    let now = Instant::now();
                    let expired = debouncer
                        .changes
                        .iter()
                        .filter_map(|(change, time)| {
                            (now.duration_since(*time) >= debouncer.debounce)
                                .then(|| change.clone())
                        })
                        .collect::<HashSet<_>>();

                    for change in expired {
                        trace!(%change, "sending change");
                        debouncer.changes.remove(&change);
                        debouncer.tx.send(change).ok();
                    }
                }
            }
        }

        debug!("debouncer shut down");
        debouncer.handle
    });

    Ok(Handle {
        shutdown: shutdown_tx,
        thread: task,
        receiver: change_rx,
    })
}
