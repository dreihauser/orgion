pub mod agenda;
pub mod auth;
pub mod backlinks;
pub mod files;
pub mod nodes;
pub mod search;
pub mod ws;

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use axum::http::{header, Method};
use axum_extra::extract::CookieJar;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Rejects with 401 unless a valid session cookie is present — skipped
/// entirely when `AppState::auth_enabled` is `false` (docs/mvp.md: auth
/// isn't required to try Orgion locally). Wraps every route except
/// `/api/auth/*`.
async fn require_auth(
    State(state): State<AppState>,
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Response {
    if !state.auth_enabled {
        return next.run(request).await;
    }
    let Some(cookie) = jar.get(auth::SESSION_COOKIE) else {
        return ApiError::unauthorized("login required").into_response();
    };
    match org_index::auth::authenticate(&state.pool, cookie.value()).await {
        Ok(_user) => next.run(request).await,
        Err(_) => {
            ApiError::unauthorized("session expired or invalid, please log in again").into_response()
        }
    }
}

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route("/api/auth/logout", axum::routing::post(auth::logout));

    let protected = Router::new()
        .route("/api/auth/me", get(auth::me))
        .route("/api/files", get(files::list_files))
        .route("/api/files/:id", get(files::get_file))
        .route("/api/files/:id/raw", get(files::get_file_raw))
        .route("/api/nodes", get(nodes::list_nodes))
        .route("/api/nodes/:id", get(nodes::get_node).patch(nodes::patch_node))
        .route("/api/agenda", get(agenda::get_agenda))
        .route("/api/search", get(search::search))
        .route("/api/backlinks/:node_id", get(backlinks::get_backlinks))
        .route("/api/ws", get(ws::ws_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .layer(
            // `Access-Control-Allow-Credentials: true` can't be combined
            // with a wildcard (`Any`) origin/headers/methods per the CORS
            // spec — tower-http enforces this at construction time — so
            // the origin is reflected per-request and headers/methods are
            // spelled out explicitly instead of using `Any`.
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::mirror_request())
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_headers([header::CONTENT_TYPE]),
        )
        .with_state(state)
}
