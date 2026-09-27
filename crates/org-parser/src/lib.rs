//! `org-parser`: lossless-ish org-mode text <-> AST parsing and rendering.
//!
//! This crate has **no I/O** — it only ever touches `&str` in, `String`
//! out. Reading/writing files, sandboxing, and locking live in
//! `org-storage`; this crate is kept pure and heavily unit tested per
//! `docs/mvp.md`'s "tests first for parser" principle.
//!
//! Two ways to turn a `Document` back into text:
//! - [`render::render_document`] — whole-file, from-scratch rendering.
//!   Use for brand-new files.
//! - [`patch::apply_edit`] / [`patch::apply_edits`] — rewrites only the
//!   header region (heading/planning/properties) of one node, preserving
//!   every other byte of the file untouched. Use for editing existing
//!   files, so Emacs users don't see spurious whole-file diffs.

pub mod heading;
pub mod links;
pub mod parse;
pub mod patch;
pub mod planning;
pub mod render;
pub mod timestamp;

pub use org_model::{Document, Node, Planning, Timestamp as OrgTimestamp, TodoType};
pub use parse::{parse, NodeSpans, ParsedDocument, Span};
pub use patch::{apply_edit, apply_edits, find_node_index_by_org_id, NodeEdit, PatchError};
pub use render::render_document;
