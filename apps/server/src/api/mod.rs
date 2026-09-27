pub mod agenda;
pub mod backlinks;
pub mod files;
pub mod nodes;
pub mod search;
pub mod ws;

use crate::state::AppState;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/files", get(files::list_files))
        .route("/api/files/:id", get(files::get_file))
        .route("/api/files/:id/raw", get(files::get_file_raw))
        .route("/api/nodes", get(nodes::list_nodes))
        .route("/api/nodes/:id", get(nodes::get_node).patch(nodes::patch_node))
        .route("/api/agenda", get(agenda::get_agenda))
        .route("/api/search", get(search::search))
        .route("/api/backlinks/:node_id", get(backlinks::get_backlinks))
        .route("/api/ws", get(ws::ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
