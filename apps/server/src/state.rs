use org_storage::Workspace;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::events::WsEvent;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub workspace: Workspace,
    pub workspace_key: Arc<str>,
    pub events: broadcast::Sender<WsEvent>,
    /// docs/mvp.md: auth is not required to demo v0.1, so it stays
    /// toggleable. When `false`, the `AuthUser` extractor lets every
    /// request through as an anonymous user instead of checking cookies.
    pub auth_enabled: bool,
}

impl AppState {
    pub fn new(pool: SqlitePool, workspace: Workspace, workspace_key: String, auth_enabled: bool) -> Self {
        let (events, _rx) = broadcast::channel(256);
        AppState {
            pool,
            workspace,
            workspace_key: workspace_key.into(),
            events,
            auth_enabled,
        }
    }
}
