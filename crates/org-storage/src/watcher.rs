//! Watches a workspace's `.org` files for external changes (Emacs, vim,
//! `git checkout`, sync tools, ...) via `inotify` (through the
//! cross-platform `notify` crate) and invokes a callback with the
//! relative path that changed. Writes Orgion itself just performed are
//! recognized via the workspace's [`WriteTracker`] and suppressed, so a
//! `PATCH /api/nodes/:id` doesn't cause a redundant reparse/broadcast.

use crate::sandbox::RelPath;
use crate::storage::Workspace;
use crate::version::content_hash;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}

pub struct FileWatcher {
    // Kept alive for as long as the watcher should keep running; dropping
    // it stops the watch.
    _inner: RecommendedWatcher,
}

impl FileWatcher {
    pub fn start<F>(workspace: &Workspace, mut on_change: F) -> notify::Result<Self>
    where
        F: FnMut(RelPath, ChangeKind) + Send + 'static,
    {
        let root = workspace.root().to_path_buf();
        let watch_root = root.clone();
        let tracker = workspace.tracker();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let event = match res {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!(?err, "file watcher error");
                    return;
                }
            };
            let kind = match event.kind {
                EventKind::Create(_) => ChangeKind::Created,
                EventKind::Modify(_) => ChangeKind::Modified,
                EventKind::Remove(_) => ChangeKind::Removed,
                _ => return,
            };
            for path in event.paths {
                if path.extension().and_then(|e| e.to_str()) != Some("org") {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(&root) else {
                    continue;
                };
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let Ok(rel_path) = RelPath::new(&rel_str) else {
                    continue;
                };

                if kind != ChangeKind::Removed {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        let hash = content_hash(contents.as_bytes());
                        if tracker.is_own_write(&path, &hash) {
                            continue;
                        }
                    }
                }

                on_change(rel_path, kind);
            }
        })?;

        watcher.watch(&watch_root, RecursiveMode::Recursive)?;
        Ok(FileWatcher { _inner: watcher })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn detects_external_write_but_not_own_write() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        let events: Arc<Mutex<Vec<(String, ChangeKind)>>> = Arc::new(Mutex::new(Vec::new()));
        let events2 = events.clone();

        let _watcher = FileWatcher::start(&ws, move |rel, kind| {
            events2.lock().unwrap().push((rel.to_string(), kind));
        })
        .unwrap();

        // Our own write: recorded in the tracker first, so it must be
        // suppressed.
        let rel = ws.rel_path("tasks.org").unwrap();
        ws.create(&rel, "* TODO Build Orgion\n").unwrap();

        // An external write straight to disk: not tracked, must surface.
        std::fs::write(ws.root().join("external.org"), "* NEXT Something\n").unwrap();

        // Give the OS/notify time to deliver the events.
        std::thread::sleep(Duration::from_millis(500));

        let seen = events.lock().unwrap();
        assert!(
            seen.iter().any(|(p, _)| p == "external.org"),
            "expected external.org change to be observed, saw: {seen:?}"
        );
        assert!(
            !seen.iter().any(|(p, _)| p == "tasks.org"),
            "expected our own write to tasks.org to be suppressed, saw: {seen:?}"
        );
    }
}
