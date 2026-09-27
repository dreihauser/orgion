use crate::dto::NodeDto;
use crate::error::ApiError;
use crate::events::WsEvent;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use org_parser::NodeEdit;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct ListNodesQuery {
    pub file: Option<String>,
    pub parent: Option<String>,
}

pub async fn list_nodes(
    State(state): State<AppState>,
    Query(q): Query<ListNodesQuery>,
) -> Result<Json<Vec<NodeDto>>, ApiError> {
    let nodes = if let Some(parent) = q.parent {
        org_index::queries::list_children(&state.pool, &parent).await?
    } else if let Some(file) = q.file {
        org_index::queries::list_nodes_for_file(&state.pool, &file).await?
    } else {
        return Err(ApiError::bad_request("either ?file= or ?parent= is required"));
    };
    Ok(Json(nodes.iter().map(NodeDto::from).collect()))
}

pub async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NodeDto>, ApiError> {
    let node = org_index::queries::get_node(&state.pool, &id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("node not found: {id}")))?;
    Ok(Json(NodeDto::from(&node)))
}

/// A timestamp as accepted from the API. `active` defaults to `true`
/// (mirroring an org `<...>` timestamp) since that's what a Web UI date
/// picker means in practice; pass `active: false` for an inactive
/// `[...]` timestamp.
#[derive(Debug, Deserialize)]
pub struct TimestampInput {
    pub date: chrono::NaiveDate,
    pub time: Option<chrono::NaiveTime>,
    #[serde(default = "default_active")]
    pub active: bool,
    pub repeater: Option<String>,
}

fn default_active() -> bool {
    true
}

impl From<TimestampInput> for org_model::Timestamp {
    fn from(t: TimestampInput) -> Self {
        org_model::Timestamp {
            date: t.date,
            time: t.time,
            end_time: None,
            active: t.active,
            repeater: t.repeater,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PatchNodeRequest {
    pub expected_version: String,
    pub title: Option<String>,
    /// Set the TODO keyword. Pass `""` to clear it (make this a plain
    /// heading with no TODO state).
    pub todo_state: Option<String>,
    pub tags: Option<Vec<String>>,
    /// Properties to upsert (existing keys not mentioned are left alone;
    /// there is no way to *remove* a property through this endpoint in
    /// v0.1 — see docs/api.md).
    pub properties: Option<BTreeMap<String, String>>,
    pub scheduled: Option<TimestampInput>,
    pub deadline: Option<TimestampInput>,
}

pub async fn patch_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchNodeRequest>,
) -> Result<Json<NodeDto>, ApiError> {
    let mut edits: Vec<NodeEdit> = Vec::new();
    if let Some(title) = req.title {
        edits.push(NodeEdit::SetTitle(title));
    }
    if let Some(todo) = req.todo_state {
        edits.push(NodeEdit::SetTodoState(if todo.is_empty() { None } else { Some(todo) }));
    }
    if let Some(tags) = req.tags {
        edits.push(NodeEdit::SetTags(tags));
    }
    if let Some(properties) = req.properties {
        for (k, v) in properties {
            edits.push(NodeEdit::SetProperty(k, v));
        }
    }
    if let Some(scheduled) = req.scheduled {
        edits.push(NodeEdit::SetScheduled(Some(scheduled.into())));
    }
    if let Some(deadline) = req.deadline {
        edits.push(NodeEdit::SetDeadline(Some(deadline.into())));
    }

    if edits.is_empty() {
        return Err(ApiError::bad_request("no fields to update"));
    }

    let updated = org_index::edit_node(
        &state.pool,
        &state.workspace,
        &state.workspace_key,
        &id,
        &edits,
        &req.expected_version,
    )
    .await?;

    let _ = state.events.send(WsEvent::node_updated(&updated));
    Ok(Json(NodeDto::from(&updated)))
}
