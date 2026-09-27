//! `org-storage`: sandboxed filesystem I/O, optimistic locking, and file
//! watching for `.org` files. See docs/architecture.md §§2, 6 and
//! docs/security.md §2.

pub mod sandbox;
pub mod storage;
pub mod version;
pub mod watcher;
pub mod write_tracker;

pub use sandbox::{RelPath, SandboxError};
pub use storage::{StorageError, VersionedContent, Workspace};
pub use version::content_hash;
pub use watcher::{ChangeKind, FileWatcher};
pub use write_tracker::WriteTracker;
