//! Bridges `org_storage::FileWatcher` (inotify) to the index and the
//! WebSocket event bus: an external edit (Emacs, vim, `git checkout`)
//! triggers a full reindex and a `file.changed` broadcast, so connected
//! Web UI clients refetch and pick up the change without a manual
//! reload — see docs/mvp.md's "Initial Success Condition".

use crate::events::WsEvent;
use crate::state::AppState;
use org_storage::FileWatcher;

/// `handle` lets the (synchronous, notify-crate-owned) watcher thread
/// spawn async reindex work back onto the Tokio runtime — `tokio::spawn`
/// only works from a thread that's already inside a runtime context,
/// which the watcher's callback thread is not.
pub fn start_watching(state: AppState, handle: tokio::runtime::Handle) -> notify::Result<FileWatcher> {
    FileWatcher::start(&state.workspace.clone(), move |rel, _kind| {
        let state = state.clone();
        handle.spawn(async move {
            if let Err(e) =
                org_index::reindex_workspace(&state.pool, &state.workspace, &state.workspace_key).await
            {
                tracing::error!(error = %e, path = %rel, "reindex after external file change failed");
                return;
            }
            let files = match org_index::queries::list_files(&state.pool, &state.workspace_key).await {
                Ok(files) => files,
                Err(e) => {
                    tracing::error!(error = %e, "listing files after reindex failed");
                    return;
                }
            };
            let file_id = files
                .iter()
                .find(|f| f.path == rel.as_str())
                .map(|f| f.id.clone())
                .unwrap_or_default();
            tracing::info!(path = %rel, "external change reindexed");
            let _ = state.events.send(WsEvent::file_changed(file_id, rel.to_string()));
        });
    })
}
