//! `org-index`: the indexing pipeline and query engine over parsed org
//! files (docs/architecture.md, docs/data-model.md). Backed by SQLite by
//! default (fine for single-user/dev per docs/mvp.md); the schema
//! (`migrations/0001_init.sql`) is written to be straightforwardly
//! portable to PostgreSQL for multi-user production deployments.
//!
//! Every table this crate writes is a disposable cache: [`reindex_workspace`]
//! can always rebuild it from the `.org` files on disk via `org-storage`
//! and `org-parser`. Nothing here is a second source of truth.

pub mod agenda;
pub mod backlinks;
pub mod db;
pub mod edit;
pub mod indexer;
pub mod model;
pub mod queries;
pub mod recurrence;
mod row;
pub mod search;

pub use agenda::{agenda as query_agenda, AgendaResult};
pub use backlinks::backlinks;
pub use db::connect;
pub use edit::{edit_node, EditError};
pub use indexer::{reindex_workspace, IndexError};
pub use model::{AgendaItem, AgendaKind, Backlink, FileRecord, IndexedNode, SearchResult, TimestampRecord};
pub use search::{search, SearchFilters};
