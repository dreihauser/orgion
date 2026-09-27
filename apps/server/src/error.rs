use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    extra: Option<serde_json::Value>,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            extra: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request",
            message: message.into(),
            extra: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
            extra: None,
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
            extra: None,
        }
    }

    fn conflict(current: &org_storage::VersionedContent) -> Self {
        ApiError {
            status: StatusCode::CONFLICT,
            code: "conflict",
            message: "file has changed since it was last read".to_string(),
            extra: Some(json!({
                "current_version": current.version,
                "current_contents": current.contents,
            })),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut body = json!({
            "error": { "code": self.code, "message": self.message }
        });
        if let Some(extra) = self.extra {
            if let (Some(obj), Some(extra_obj)) = (body.as_object_mut(), extra.as_object()) {
                for (k, v) in extra_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        (self.status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::internal(e.to_string())
    }
}

impl From<org_storage::StorageError> for ApiError {
    fn from(e: org_storage::StorageError) -> Self {
        match e {
            org_storage::StorageError::Conflict { current } => ApiError::conflict(&current),
            org_storage::StorageError::NotFound(p) => ApiError::not_found(p),
            other => ApiError::internal(other.to_string()),
        }
    }
}

impl From<org_storage::SandboxError> for ApiError {
    fn from(e: org_storage::SandboxError) -> Self {
        ApiError::bad_request(e.to_string())
    }
}

impl From<org_index::EditError> for ApiError {
    fn from(e: org_index::EditError) -> Self {
        match e {
            org_index::EditError::NotFound(id) => ApiError::not_found(format!("node not found: {id}")),
            org_index::EditError::Storage(e) => ApiError::from(e),
            org_index::EditError::Sandbox(e) => ApiError::from(e),
            org_index::EditError::Db(e) => ApiError::from(e),
            org_index::EditError::Patch(e) => ApiError::bad_request(e.to_string()),
        }
    }
}
