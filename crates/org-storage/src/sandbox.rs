//! Path sandboxing: API/CLI input is never trusted as a filesystem path
//! directly (see docs/security.md §2). Every path that reaches disk goes
//! through [`RelPath::new`], which is the single choke point rejecting
//! `..`, absolute paths, null bytes, and symlink escapes.

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SandboxError {
    #[error("path must be relative: {0}")]
    NotRelative(String),
    #[error("path escapes the workspace root: {0}")]
    Escapes(String),
    #[error("path contains a null byte or is otherwise invalid: {0}")]
    Invalid(String),
}

/// A path that has been validated as staying within a `Workspace` root.
/// Stored internally as a normalized, forward-slash relative path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelPath(String);

impl RelPath {
    /// Validate a client-supplied relative path string. Rejects absolute
    /// paths, `.`/`..` components, and null bytes. Does **not** check the
    /// filesystem — see `Workspace::rel_path` for the symlink-aware
    /// version used when a `Workspace` root is available.
    pub fn new(candidate: &str) -> Result<Self, SandboxError> {
        if candidate.contains('\0') {
            return Err(SandboxError::Invalid(candidate.to_string()));
        }
        let path = Path::new(candidate);
        let mut normalized = Vec::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => {
                    let part = part.to_str().ok_or_else(|| {
                        SandboxError::Invalid(candidate.to_string())
                    })?;
                    normalized.push(part.to_string());
                }
                Component::CurDir => {}
                Component::ParentDir => return Err(SandboxError::Escapes(candidate.to_string())),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(SandboxError::NotRelative(candidate.to_string()))
                }
            }
        }
        if normalized.is_empty() {
            return Err(SandboxError::Invalid(candidate.to_string()));
        }
        Ok(RelPath(normalized.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RelPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resolve `rel` against `root` and confirm the result — following
/// symlinks — is still within `root`. `root` must already be
/// canonicalized. Works for paths that don't exist yet by walking up to
/// the nearest existing ancestor, canonicalizing *that*, and checking
/// containment there (new path components can't be symlinks by
/// definition — they don't exist).
pub fn resolve_within_root(root: &Path, rel: &RelPath) -> Result<PathBuf, SandboxError> {
    let candidate = root.join(rel.as_str());

    let mut existing = candidate.as_path();
    // Components of `candidate` that don't exist yet, nearest-first;
    // reversed and joined onto the canonicalized existing ancestor below.
    let mut trailing_components: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        match existing.parent() {
            Some(parent) if parent != existing => {
                if let Some(name) = existing.file_name() {
                    trailing_components.push(name.to_os_string());
                }
                existing = parent;
            }
            _ => break,
        }
    }

    let canonical_existing = existing
        .canonicalize()
        .map_err(|_| SandboxError::Invalid(rel.as_str().to_string()))?;
    if !canonical_existing.starts_with(root) {
        return Err(SandboxError::Escapes(rel.as_str().to_string()));
    }
    let mut result = canonical_existing;
    for component in trailing_components.into_iter().rev() {
        result.push(component);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_relative_path() {
        assert_eq!(RelPath::new("tasks.org").unwrap().as_str(), "tasks.org");
        assert_eq!(
            RelPath::new("projects/hri/tasks.org").unwrap().as_str(),
            "projects/hri/tasks.org"
        );
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        assert_eq!(
            RelPath::new("../../etc/passwd"),
            Err(SandboxError::Escapes("../../etc/passwd".to_string()))
        );
        assert_eq!(
            RelPath::new("projects/../../etc/passwd"),
            Err(SandboxError::Escapes("projects/../../etc/passwd".to_string()))
        );
    }

    #[test]
    fn rejects_absolute_path() {
        assert_eq!(
            RelPath::new("/etc/passwd"),
            Err(SandboxError::NotRelative("/etc/passwd".to_string()))
        );
    }

    #[test]
    fn rejects_null_byte() {
        assert!(RelPath::new("foo\0.org").is_err());
    }

    #[test]
    fn normalizes_current_dir_components() {
        assert_eq!(RelPath::new("./tasks.org").unwrap().as_str(), "tasks.org");
    }

    #[test]
    fn symlink_escape_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.org"), "* secret\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.join("escape")).unwrap();

        #[cfg(unix)]
        {
            let rel = RelPath::new("escape/secret.org").unwrap();
            let result = resolve_within_root(&root, &rel);
            assert!(matches!(result, Err(SandboxError::Escapes(_))));
        }
    }

    #[test]
    fn new_file_path_resolves_under_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let rel = RelPath::new("brand-new.org").unwrap();
        let resolved = resolve_within_root(&root, &rel).unwrap();
        assert_eq!(resolved, root.join("brand-new.org"));
    }
}
