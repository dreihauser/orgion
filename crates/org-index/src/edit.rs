//! High-level "edit a node" operation: read -> patch (via `org-parser`)
//! -> write (via `org-storage`, optimistically locked) -> reindex ->
//! return the fresh row. This is the function `apps/server`'s
//! `PATCH /api/nodes/:id` calls; see docs/api.md and docs/architecture.md
//! §6.

use crate::indexer::{file_uuid, reindex_workspace};
use crate::model::IndexedNode;
use crate::queries;
use org_storage::Workspace;
use sqlx::SqlitePool;

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("node not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Storage(#[from] org_storage::StorageError),
    #[error(transparent)]
    Sandbox(#[from] org_storage::SandboxError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Patch(#[from] org_parser::PatchError),
}

/// Apply `edits` to the node identified by `node_id` (an `IndexedNode.id`
/// as returned by any query in this crate). `expected_version` must equal
/// the owning file's current content hash (`IndexedNode.version`) or the
/// write is rejected — surfaced here as
/// `EditError::Storage(StorageError::Conflict { current })`, which the
/// server layer turns into an HTTP 409 per docs/api.md.
pub async fn edit_node(
    pool: &SqlitePool,
    workspace: &Workspace,
    workspace_key: &str,
    node_id: &str,
    edits: &[org_parser::NodeEdit],
    expected_version: &str,
) -> Result<IndexedNode, EditError> {
    let existing = queries::get_node(pool, node_id)
        .await?
        .ok_or_else(|| EditError::NotFound(node_id.to_string()))?;

    let siblings = queries::list_nodes_for_file(pool, &existing.file_id).await?;
    let node_index = siblings
        .iter()
        .position(|n| n.id == existing.id)
        .ok_or_else(|| EditError::NotFound(node_id.to_string()))?;

    let rel = workspace.rel_path(&existing.file_path)?;
    let content = workspace.read(&rel)?;
    let parsed = org_parser::parse(&content.contents);
    let new_text = org_parser::apply_edits(&content.contents, &parsed, node_index, edits)?;

    workspace.write_if_unchanged(&rel, &new_text, expected_version)?;

    reindex_workspace(pool, workspace, workspace_key)
        .await
        .map_err(|e| match e {
            crate::indexer::IndexError::Db(e) => EditError::Db(e),
            crate::indexer::IndexError::Storage(e) => EditError::Storage(e),
            crate::indexer::IndexError::Json(e) => {
                EditError::Db(sqlx::Error::Decode(Box::new(e)))
            }
        })?;

    let file_id = file_uuid(workspace_key, rel.as_str()).to_string();
    let updated_siblings = queries::list_nodes_for_file(pool, &file_id).await?;
    updated_siblings
        .into_iter()
        .nth(node_index)
        .ok_or_else(|| EditError::NotFound(node_id.to_string()))
}
