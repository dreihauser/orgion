//! Login/logout/me + workspace listing — docs/api.md "Auth". Local
//! accounts only in v0.1 (no OIDC/proxy auth wiring yet — see
//! docs/security.md §3); a single admin account is created by
//! `orgion init`.

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::request::Parts;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use org_index::{User, WorkspaceRecord};
use serde::{Deserialize, Serialize};

pub const SESSION_COOKIE: &str = "orgion_session";

/// Extractor requiring an authenticated request. When
/// `AppState::auth_enabled` is `false` (the v0.1 default for quick local
/// trials — see docs/mvp.md), every request is treated as an anonymous
/// user instead of being rejected.
pub struct AuthUser(pub User);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if !state.auth_enabled {
            return Ok(AuthUser(User {
                id: "anonymous".to_string(),
                username: "anonymous".to_string(),
                display_name: None,
            }));
        }

        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or_else(|| ApiError::unauthorized("login required"))?;

        org_index::auth::authenticate(&state.pool, &token)
            .await
            .map(AuthUser)
            .map_err(|_| ApiError::unauthorized("session expired or invalid, please log in again"))
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

fn session_cookie(token: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        // Not marked `.secure(true)`: v0.1 targets a typical self-hosted
        // setup where TLS is terminated by a reverse proxy/tunnel in
        // front of Orgion (docs/security.md §8), and forcing `Secure`
        // here would silently drop the cookie for a plain-http local
        // trial. Revisit once orgion.toml can declare "I am reachable
        // only over https".
        .max_age(time::Duration::days(30))
        .build()
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    let (token, user) = org_index::auth::login(&state.pool, &req.username, &req.password)
        .await
        .map_err(|_| ApiError::unauthorized("invalid username or password"))?;
    let jar = CookieJar::new().add(session_cookie(token));
    Ok((jar, Json(user)))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<CookieJar, ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let _ = org_index::auth::logout(&state.pool, cookie.value()).await;
    }
    Ok(jar.remove(Cookie::from(SESSION_COOKIE)))
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: User,
    pub workspaces: Vec<WorkspaceRecord>,
}

pub async fn me(State(state): State<AppState>, AuthUser(user): AuthUser) -> Result<Json<MeResponse>, ApiError> {
    let workspaces = if state.auth_enabled {
        org_index::auth::list_workspaces_for_user(&state.pool, &user.id)
            .await
            .map_err(|e| ApiError::bad_request(e.to_string()))?
    } else {
        Vec::new()
    };
    Ok(Json(MeResponse { user, workspaces }))
}
