//! File system watcher, that is aware of the project structure, and triggers [`ChangeType`] events
//! based on the paths modified.

use std::{
    path::{Path, PathBuf},
    thread,
};

use anyhow::Result;
use flume::Selector;
use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    WalkBuilder,
};
use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecursiveMode, Watcher,
};
use tracing::{debug, error, trace, warn};

use super::ChangeType;

/// Background file watcher that handles raw file system events from the [`notify`] crate.
///
/// It automatically takes care of watching any newly created files, and folders, unwatching
/// deleted ones, and properly adjusting to renames.
struct ProjectWatcher {
    /// Path to the project directory.
    project: PathBuf,
    /// The underlying [`notify`] watcher, needed to watch/unwatch, after initially building it.
    watcher: notify::RecommendedWatcher,
    /// Loaded gitignore patterns, to filter out any folders or files added later on.
    gitignore: Gitignore,
    /// Receiver for events from [`notify`].
    notify_rx: flume::Receiver<Result<Event, notify::Error>>,
    /// Sender for notify events converted to [`ChangeType`].
    change_tx: flume::Sender<ChangeType>,
    /// Listener for a shut down signal from the [`Handle`], which will halt the event loop and
    /// stop all watching machinery.
    shutdown: flume::Receiver<()>,
}

impl ProjectWatcher {
    /// Handle a single file system event from [`notify`], converting it into one or potentially
    /// multiple [`ChangeType`]s. Also adds any newly created files or folder to the watcher, and
    /// removes deleted ones.
    fn handle_event(&mut self, ev: Result<Event, notify::Error>) {
        let ev = match ev {
            Ok(ev) => ev,
            Err(e) => {
                error!(error = %e, "fs event error");
                return;
            }
        };

        match ev.kind {
            EventKind::Access(_) => {
                // access events aren't important to us
                return;
            }
            EventKind::Create(_) => {
                self.add_paths(&ev.paths);
            }
            EventKind::Modify(modify) => match modify {
                ModifyKind::Any | ModifyKind::Other | ModifyKind::Data(_) => {}
                ModifyKind::Metadata(_) => {
                    // metadata changes aren't important to us
                    return;
                }
                ModifyKind::Name(name) => match name {
                    RenameMode::To | RenameMode::From => {
                        // should get a `Both` event after `To` and `From` are fired.
                        return;
                    }
                    RenameMode::Both => {
                        debug_assert!(ev.paths.len() == 2, "there should be two paths");
                        if ev.paths.len() == 2 {
                            self.remove_paths(&[&ev.paths[0]]);
                            self.add_paths(&[&ev.paths[1]]);
                        }
                    }
                    RenameMode::Any | RenameMode::Other => {
                        debug!(paths = ?ev.paths, "got RenameMode::Any/Other event");
                        return;
                    }
                },
            },
            EventKind::Remove(_) => {
                self.remove_paths(&ev.paths);
            }
            EventKind::Any | EventKind::Other => {
                debug!(paths = ?ev.paths, "got EventKind::Any/Other");
                return;
            }
        }

        for change in ev
            .paths
            .into_iter()
            .filter(|path| !self.gitignore.matched(path, path.is_dir()).is_ignore())
            .filter_map(|path| self.to_build_trigger(path))
        {
            self.change_tx.send(change).ok();
        }
    }

    /// Decide what part of the project to rebuild, based on the given path.
    fn to_build_trigger(&self, full_path: PathBuf) -> Option<ChangeType> {
        // Only fails if we get a path that's not within the project dir and don't really have to
        // bother those. Shouldn't ever happen in the first place, as we only watch files inside
        // the project.
        let path = full_path.strip_prefix(&self.project).ok()?;

        Some(if path == Path::new("index.html") {
            ChangeType::Html
        } else if path == Path::new("assets/main.sass")
            || path == Path::new("assets/main.scss")
            || path == Path::new("assets/main.css")
            || path.starts_with("assets/sass/")
            || path.starts_with("assets/scss/")
            || path.starts_with("assets/css/")
        {
            ChangeType::Css
        } else if path.starts_with(Path::new("assets/")) {
            ChangeType::Static(full_path)
        } else {
            ChangeType::Rust
        })
    }

    /// Add the paths to the file watcher, filtering out any that should be ignored by the
    /// `.gitignore` patterns.
    fn add_paths(&mut self, paths: &[impl AsRef<Path>]) {
        for path in paths {
            let path = path.as_ref();

            if self.gitignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            if let Err(e) = self.watcher.watch(path, RecursiveMode::NonRecursive) {
                error!(error = %e, "failed adding path to watcher");
            }
        }
    }

    /// Remove the given paths from the watcher again.
    fn remove_paths(&mut self, paths: &[impl AsRef<Path>]) {
        for path in paths {
            if let Err(e) = self.watcher.unwatch(path.as_ref()) {
                warn!(error = %e, "failed removing path from watcher");
            }
        }
    }
}

/// Handle to the file watcher that is run in a background thread. Allows to receive change events
/// over a channel, and can shutdown the watcher.
pub struct Handle {
    /// Sender to signal the background file watcher to shut down.
    shutdown: flume::Sender<()>,
    /// Handle to the thread that manages file watcher events. Used to wait for the thread to
    /// finish processing, after sending it a shutdown signal.
    thread: thread::JoinHandle<()>,
    /// Receiver for [`ChangeType`] events from the background file watcher thread.
    receiver: flume::Receiver<ChangeType>,
}

impl Handle {
    /// Signal the background file watcher to shut down and wait until it's fully stopped.
    pub fn shutdown(self) {
        self.shutdown.send(()).ok();
        self.thread.join().expect("task to shut down properly");
    }

    /// Get access to the receiver that can pull new change events.
    pub fn receiver(&self) -> &flume::Receiver<ChangeType> {
        &self.receiver
    }

    #[cfg(test)]
    pub fn recv(&self) -> Option<ChangeType> {
        self.receiver.recv().ok()
    }

    #[cfg(test)]
    pub fn try_recv(&self) -> Option<ChangeType> {
        self.receiver.try_recv().ok()
    }
}

/// Create a watcher over the given project, triggering change events for the different components
/// of it. This takes the project's `.gitignore` file into account.
pub fn watch(project: PathBuf) -> Result<Handle> {
    let (notify_tx, notify_rx) = flume::bounded(super::CHANNEL_SIZE);
    let mut watcher = notify::recommended_watcher(move |ev| {
        notify_tx.send(ev).ok();
    })?;

    // Disable the default filters, and only really care about .gitignore patterns for
    // path exclusion.
    let walker = WalkBuilder::new(&project)
        .standard_filters(false)
        .require_git(false)
        // .git_exclude(true) // TODO: maybe worth activating this later on
        .git_ignore(true)
        .build();

    let gitignore = {
        let mut builder = GitignoreBuilder::new(&project);
        builder.add_line(None, ".git/")?;

        if let Some(error) = builder.add(project.join(".gitignore")) {
            return Err(error.into());
        }

        builder.build()?
    };

    for entry in walker {
        let entry = entry?;
        let path = entry.path().strip_prefix(&project).unwrap_or(entry.path());

        trace!(path = %path.display(), "added watch path");

        watcher.watch(entry.path(), RecursiveMode::NonRecursive)?;
    }

    let (change_tx, change_rx) = flume::bounded(super::CHANNEL_SIZE);
    let (shutdown_tx, shutdown_rx) = flume::bounded(0);

    let mut watcher = ProjectWatcher {
        project,
        watcher,
        gitignore,
        notify_rx,
        change_tx,
        shutdown: shutdown_rx,
    };

    let task = thread::spawn(move || {
        while let Some(ev) = Selector::new()
            .recv(&watcher.shutdown, |_| None)
            .recv(&watcher.notify_rx, |ev| ev.ok())
            .wait()
        {
            watcher.handle_event(ev);
        }

        debug!("watcher shut down");
    });

    let handle = Handle {
        shutdown: shutdown_tx,
        thread: task,
        receiver: change_rx,
    };

    Ok(handle)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{env, fs};

    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn create_watcher() -> Result<()> {
        let dir = env::current_dir()?.join("sample");
        watch(dir)?.shutdown();
        Ok(())
    }

    #[test]
    fn watch_once() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child(".gitignore").touch()?;
        let test_txt = temp.child("test.txt");
        test_txt.touch()?;

        let watcher = watch(temp.path().to_owned())?;

        test_txt.write_str("hello")?;

        assert_eq!(Some(ChangeType::Rust), watcher.recv());
        assert_eq!(None, watcher.try_recv());

        watcher.shutdown();

        Ok(())
    }

    #[test]
    fn move_file() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child(".gitignore").touch()?;
        temp.child("a").create_dir_all()?;

        let watcher = watch(temp.path().to_owned())?;

        fs::rename(temp.join("a"), temp.join("b"))?;

        assert_eq!(Some(ChangeType::Rust), watcher.recv());
        assert_eq!(Some(ChangeType::Rust), watcher.recv());
        assert_eq!(None, watcher.try_recv());

        watcher.shutdown();

        Ok(())
    }

    #[test]
    fn delete_file() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child(".gitignore").touch()?;
        temp.child("a").create_dir_all()?;

        let watcher = watch(temp.path().to_owned())?;

        fs::remove_dir_all(temp.join("a"))?;

        assert_eq!(Some(ChangeType::Rust), watcher.recv());
        assert_eq!(Some(ChangeType::Rust), watcher.recv());
        assert_eq!(None, watcher.try_recv());

        Ok(())
    }
}
