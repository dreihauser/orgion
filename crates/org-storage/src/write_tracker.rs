//! Lets the file watcher tell "we just wrote this" apart from "someone
//! else (Emacs, vim, git) changed this file", so the watcher can skip the
//! redundant reparse/rebroadcast for the former (see docs/architecture.md
//! §2).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub struct WriteTracker {
    inner: Mutex<HashMap<PathBuf, String>>,
}

impl WriteTracker {
    pub fn record(&self, path: PathBuf, version: String) {
        self.inner.lock().unwrap().insert(path, version);
    }

    /// Returns `true` if `path`'s last known on-disk version, as recorded
    /// by `record`, equals `version` — i.e. this change was our own
    /// write, already reflected in the index.
    ///
    /// Deliberately does *not* consume the entry: a single atomic write
    /// (temp file + rename) can surface as more than one filesystem event
    /// for the same path, and every one of them needs to match so none
    /// leaks through as a spurious "external change". The (accepted)
    /// tradeoff is that a later external edit which happens to reproduce
    /// byte-identical content is also treated as our own write once, until
    /// the next real write overwrites this entry.
    pub fn is_own_write(&self, path: &Path, version: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.get(path).map(|v| v.as_str()) == Some(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_own_write_repeatedly() {
        let tracker = WriteTracker::default();
        let path = PathBuf::from("/tmp/tasks.org");
        tracker.record(path.clone(), "sha256:abc".to_string());
        // A single atomic write can surface as more than one FS event;
        // every one of them must match, not just the first.
        assert!(tracker.is_own_write(&path, "sha256:abc"));
        assert!(tracker.is_own_write(&path, "sha256:abc"));
    }

    #[test]
    fn does_not_match_a_different_version() {
        let tracker = WriteTracker::default();
        let path = PathBuf::from("/tmp/tasks.org");
        tracker.record(path.clone(), "sha256:abc".to_string());
        assert!(!tracker.is_own_write(&path, "sha256:different"));
    }
}
