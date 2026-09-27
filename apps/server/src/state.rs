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
}

impl AppState {
    pub fn new(pool: SqlitePool, workspace: Workspace, workspace_key: String) -> Self {
        let (events, _rx) = broadcast::channel(256);
        AppState {
            pool,
            workspace,
            workspace_key: workspace_key.into(),
            events,
        }
    }
}
