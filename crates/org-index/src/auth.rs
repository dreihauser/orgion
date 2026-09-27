//! Local accounts, sessions, and workspaces — the "platform model"
//! (docs/data-model.md §2). Unlike the content tables, this crate's
//! `users`/`sessions`/`workspaces` tables *are* the system of record:
//! there is no `.org` file to rebuild them from.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, Utc};
use rand_core::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

const SESSION_TTL_DAYS: i64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("username already exists")]
    UsernameTaken,
    #[error("not authenticated")]
    Unauthenticated,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error("password hashing failed: {0}")]
    Hash(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceRecord {
    pub id: String,
    pub key: String,
    pub name: String,
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Hash(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Create a user (used by `orgion init` to bootstrap the admin account).
/// Returns `AuthError::UsernameTaken` if the username is already in use.
pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    display_name: Option<&str>,
) -> Result<User, AuthError> {
    let existing = sqlx::query("SELECT id FROM users WHERE username = ?1")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    if existing.is_some() {
        return Err(AuthError::UsernameTaken);
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, created_at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&id)
    .bind(username)
    .bind(&password_hash)
    .bind(display_name)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;

    Ok(User {
        id,
        username: username.to_string(),
        display_name: display_name.map(String::from),
    })
}

/// Verify a username/password pair and, on success, create a new session.
/// Returns the raw session token (to be set as an HttpOnly cookie — only
/// its hash is stored) plus the authenticated user.
pub async fn login(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<(String, User), AuthError> {
    let row = sqlx::query("SELECT id, username, password_hash, display_name FROM users WHERE username = ?1")
        .bind(username)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

    let password_hash: String = row.try_get("password_hash")?;
    if !verify_password(password, &password_hash) {
        return Err(AuthError::InvalidCredentials);
    }

    let user = User {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
    };

    let token = new_token();
    let now = Utc::now();
    let expires = now + Duration::days(SESSION_TTL_DAYS);
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, created_at, expires_at) VALUES (?1,?2,?3,?4)",
    )
    .bind(hash_token(&token))
    .bind(&user.id)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(pool)
    .await?;

    Ok((token, user))
}

/// Validate a raw session token (as read from the cookie), returning the
/// authenticated user if the session exists and hasn't expired.
pub async fn authenticate(pool: &SqlitePool, token: &str) -> Result<User, AuthError> {
    let row = sqlx::query(
        "SELECT u.id, u.username, u.display_name, s.expires_at
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = ?1",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::Unauthenticated)?;

    let expires_at: String = row.try_get("expires_at")?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map_err(|_| AuthError::Unauthenticated)?;
    if expires_at < Utc::now() {
        return Err(AuthError::Unauthenticated);
    }

    Ok(User {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
    })
}

pub async fn logout(pool: &SqlitePool, token: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?1")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// Idempotently register a workspace (by its config `key`) and grant
/// `owner_user_id` membership on it. Called by `orgion init`.
pub async fn ensure_workspace(
    pool: &SqlitePool,
    key: &str,
    name: &str,
    root_path: &str,
    owner_user_id: &str,
) -> Result<WorkspaceRecord, AuthError> {
    let existing = sqlx::query("SELECT id, key, name FROM workspaces WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    let workspace = if let Some(row) = existing {
        WorkspaceRecord {
            id: row.try_get("id")?,
            key: row.try_get("key")?,
            name: row.try_get("name")?,
        }
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO workspaces (id, key, name, root_path, created_at) VALUES (?1,?2,?3,?4,?5)",
        )
        .bind(&id)
        .bind(key)
        .bind(name)
        .bind(root_path)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await?;
        WorkspaceRecord {
            id,
            key: key.to_string(),
            name: name.to_string(),
        }
    };

    sqlx::query(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES (?1,?2,'owner')
         ON CONFLICT (workspace_id, user_id) DO NOTHING",
    )
    .bind(&workspace.id)
    .bind(owner_user_id)
    .execute(pool)
    .await?;

    Ok(workspace)
}

pub async fn list_workspaces_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<WorkspaceRecord>, AuthError> {
    let rows = sqlx::query(
        "SELECT w.id, w.key, w.name FROM workspaces w
         JOIN workspace_members m ON m.workspace_id = w.id
         WHERE m.user_id = ?1
         ORDER BY w.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|row| {
            Ok(WorkspaceRecord {
                id: row.try_get("id")?,
                key: row.try_get("key")?,
                name: row.try_get("name")?,
            })
        })
        .collect()
}

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, AuthError> {
    let row = sqlx::query("SELECT id, username, display_name FROM users WHERE username = ?1")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    row.map(|row| {
        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            display_name: row.try_get("display_name")?,
        })
    })
    .transpose()
}

/// A one-time, randomly generated admin password (used by `orgion init`
/// when `--admin-password` isn't given). ~122 bits of entropy; the user
/// is expected to copy it out of the terminal once. There is no
/// "change password" flow yet in v0.1 — see docs/mvp.md.
pub fn generate_password() -> String {
    Uuid::new_v4().simple().to_string()
}

pub async fn any_user_exists(pool: &SqlitePool) -> Result<bool, AuthError> {
    let row = sqlx::query("SELECT COUNT(*) as c FROM users")
        .fetch_one(pool)
        .await?;
    let count: i64 = row.try_get("c")?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        crate::db::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn create_login_authenticate_roundtrip() {
        let pool = pool().await;
        create_user(&pool, "admin", "hunter2", None).await.unwrap();

        let (token, user) = login(&pool, "admin", "hunter2").await.unwrap();
        assert_eq!(user.username, "admin");

        let authed = authenticate(&pool, &token).await.unwrap();
        assert_eq!(authed.id, user.id);
    }

    #[tokio::test]
    async fn wrong_password_is_rejected() {
        let pool = pool().await;
        create_user(&pool, "admin", "hunter2", None).await.unwrap();
        let err = login(&pool, "admin", "wrong").await.unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn logout_invalidates_session() {
        let pool = pool().await;
        create_user(&pool, "admin", "hunter2", None).await.unwrap();
        let (token, _) = login(&pool, "admin", "hunter2").await.unwrap();
        logout(&pool, &token).await.unwrap();
        let err = authenticate(&pool, &token).await.unwrap_err();
        assert!(matches!(err, AuthError::Unauthenticated));
    }

    #[tokio::test]
    async fn duplicate_username_is_rejected() {
        let pool = pool().await;
        create_user(&pool, "admin", "hunter2", None).await.unwrap();
        let err = create_user(&pool, "admin", "other", None).await.unwrap_err();
        assert!(matches!(err, AuthError::UsernameTaken));
    }

    #[tokio::test]
    async fn ensure_workspace_is_idempotent() {
        let pool = pool().await;
        let user = create_user(&pool, "admin", "hunter2", None).await.unwrap();
        let w1 = ensure_workspace(&pool, "default", "Default", "/tmp/org", &user.id)
            .await
            .unwrap();
        let w2 = ensure_workspace(&pool, "default", "Default", "/tmp/org", &user.id)
            .await
            .unwrap();
        assert_eq!(w1.id, w2.id);

        let workspaces = list_workspaces_for_user(&pool, &user.id).await.unwrap();
        assert_eq!(workspaces.len(), 1);
    }
}
