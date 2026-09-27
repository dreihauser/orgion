//! Sandboxed, optimistically-locked read/write access to `.org` files
//! under a workspace root. See docs/architecture.md §6 and
//! docs/security.md §2.

use crate::sandbox::{resolve_within_root, RelPath, SandboxError};
use crate::version::content_hash;
use crate::write_tracker::WriteTracker;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sandbox violation: {0}")]
    Sandbox(#[from] SandboxError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("conflict: file has changed since it was last read")]
    Conflict { current: VersionedContent },
    #[error("file not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedContent {
    pub contents: String,
    pub version: String,
}

/// A registered workspace: a canonicalized root directory that all
/// `.org` file access is confined to.
#[derive(Clone)]
pub struct Workspace {
    root: PathBuf,
    tracker: Arc<WriteTracker>,
}

impl Workspace {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into().canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("workspace root is not a directory: {}", root.display()),
            ));
        }
        Ok(Workspace {
            root,
            tracker: Arc::new(WriteTracker::default()),
        })
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub fn tracker(&self) -> Arc<WriteTracker> {
        self.tracker.clone()
    }

    /// Validate a client-supplied path string against this workspace,
    /// including a filesystem-level symlink-escape check.
    pub fn rel_path(&self, candidate: &str) -> Result<RelPath, SandboxError> {
        let rel = RelPath::new(candidate)?;
        resolve_within_root(&self.root, &rel)?;
        Ok(rel)
    }

    fn abs_path(&self, rel: &RelPath) -> Result<PathBuf, SandboxError> {
        resolve_within_root(&self.root, rel)
    }

    /// Recursively list every `*.org` file under the workspace root,
    /// following no symlinks outside the root (walkdir does not follow
    /// symlinks by default).
    pub fn list_org_files(&self) -> Result<Vec<RelPath>, StorageError> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("org")
            {
                let rel = entry
                    .path()
                    .strip_prefix(&self.root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(RelPath::new(&rel)?);
            }
        }
        Ok(out)
    }

    pub fn read(&self, rel: &RelPath) -> Result<VersionedContent, StorageError> {
        let path = self.abs_path(rel)?;
        let contents = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                StorageError::NotFound(rel.to_string())
            } else {
                StorageError::Io(e)
            }
        })?;
        let version = content_hash(contents.as_bytes());
        Ok(VersionedContent { contents, version })
    }

    /// Write `new_contents` to `rel`, but only if the file's current
    /// on-disk version still matches `expected_version`. Writes are
    /// atomic (write to a sibling temp file, then rename) so a reader
    /// never observes a partial write. Records the resulting hash in the
    /// write tracker so the file watcher can recognize this as our own
    /// write rather than an external edit.
    pub fn write_if_unchanged(
        &self,
        rel: &RelPath,
        new_contents: &str,
        expected_version: &str,
    ) -> Result<VersionedContent, StorageError> {
        let path = self.abs_path(rel)?;

        let current = match std::fs::read_to_string(&path) {
            Ok(c) => Some(c),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(StorageError::Io(e)),
        };

        if let Some(current) = &current {
            let current_version = content_hash(current.as_bytes());
            if current_version != expected_version {
                return Err(StorageError::Conflict {
                    current: VersionedContent {
                        contents: current.clone(),
                        version: current_version,
                    },
                });
            }
        } else if !expected_version.is_empty() {
            return Err(StorageError::NotFound(rel.to_string()));
        }

        let new_version = content_hash(new_contents.as_bytes());
        atomic_write(&path, new_contents)?;
        self.tracker.record(path, new_version.clone());

        Ok(VersionedContent {
            contents: new_contents.to_string(),
            version: new_version,
        })
    }

    /// Create a brand-new file. Fails with `Conflict` if it already
    /// exists (use `write_if_unchanged` with that file's real version to
    /// edit an existing file instead).
    pub fn create(&self, rel: &RelPath, contents: &str) -> Result<VersionedContent, StorageError> {
        let path = self.abs_path(rel)?;
        if path.exists() {
            let existing = std::fs::read_to_string(&path)?;
            return Err(StorageError::Conflict {
                current: VersionedContent {
                    version: content_hash(existing.as_bytes()),
                    contents: existing,
                },
            });
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let version = content_hash(contents.as_bytes());
        atomic_write(&path, contents)?;
        self.tracker.record(path, version.clone());
        Ok(VersionedContent {
            contents: contents.to_string(),
            version,
        })
    }
}

fn atomic_write(path: &std::path::Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    use std::io::Write;
    tmp.write_all(contents.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> (tempfile::TempDir, Workspace) {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        (dir, ws)
    }

    #[test]
    fn create_then_read() {
        let (_dir, ws) = workspace();
        let rel = ws.rel_path("tasks.org").unwrap();
        ws.create(&rel, "* TODO Build Orgion\n").unwrap();
        let content = ws.read(&rel).unwrap();
        assert_eq!(content.contents, "* TODO Build Orgion\n");
    }

    #[test]
    fn write_if_unchanged_succeeds_with_correct_version() {
        let (_dir, ws) = workspace();
        let rel = ws.rel_path("tasks.org").unwrap();
        let v1 = ws.create(&rel, "* TODO Build Orgion\n").unwrap();
        let v2 = ws
            .write_if_unchanged(&rel, "* DONE Build Orgion\n", &v1.version)
            .unwrap();
        assert_ne!(v1.version, v2.version);
        assert_eq!(ws.read(&rel).unwrap().contents, "* DONE Build Orgion\n");
    }

    #[test]
    fn write_if_unchanged_conflicts_on_stale_version() {
        let (_dir, ws) = workspace();
        let rel = ws.rel_path("tasks.org").unwrap();
        let v1 = ws.create(&rel, "* TODO Build Orgion\n").unwrap();
        // Simulate an external (Emacs) edit landing first.
        ws.write_if_unchanged(&rel, "* NEXT Build Orgion\n", &v1.version)
            .unwrap();
        // Our stale write, still using v1, must be rejected.
        let err = ws
            .write_if_unchanged(&rel, "* DONE Build Orgion\n", &v1.version)
            .unwrap_err();
        match err {
            StorageError::Conflict { current } => {
                assert_eq!(current.contents, "* NEXT Build Orgion\n");
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn create_conflicts_if_file_already_exists() {
        let (_dir, ws) = workspace();
        let rel = ws.rel_path("tasks.org").unwrap();
        ws.create(&rel, "* TODO A\n").unwrap();
        let err = ws.create(&rel, "* TODO B\n").unwrap_err();
        assert!(matches!(err, StorageError::Conflict { .. }));
    }

    #[test]
    fn list_org_files_finds_nested_files() {
        let (_dir, ws) = workspace();
        ws.create(&ws.rel_path("a.org").unwrap(), "* A\n").unwrap();
        ws.create(&ws.rel_path("projects/b.org").unwrap(), "* B\n")
            .unwrap();
        ws.create(&ws.rel_path("notes.txt").unwrap(), "not org").unwrap();
        let mut files: Vec<String> = ws
            .list_org_files()
            .unwrap()
            .into_iter()
            .map(|r| r.to_string())
            .collect();
        files.sort();
        assert_eq!(files, vec!["a.org".to_string(), "projects/b.org".to_string()]);
    }

    #[test]
    fn rejects_traversal_at_the_workspace_boundary() {
        let (_dir, ws) = workspace();
        assert!(ws.rel_path("../outside.org").is_err());
    }
}
