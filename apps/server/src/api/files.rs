use crate::dto::NodeDto;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FileSummaryDto {
    pub id: String,
    pub path: String,
    pub node_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_files(State(state): State<AppState>) -> Result<Json<Vec<FileSummaryDto>>, ApiError> {
    let files = org_index::queries::list_files(&state.pool, &state.workspace_key).await?;
    let mut out = Vec::with_capacity(files.len());
    for f in files {
        let nodes = org_index::queries::list_nodes_for_file(&state.pool, &f.id).await?;
        out.push(FileSummaryDto {
            id: f.id,
            path: f.path,
            node_count: nodes.len() as i64,
            updated_at: f.indexed_at,
        });
    }
    Ok(Json(out))
}

#[derive(Debug, Serialize)]
pub struct FileDetailDto {
    pub id: String,
    pub path: String,
    pub version: String,
    pub nodes: Vec<NodeDto>,
}

pub async fn get_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FileDetailDto>, ApiError> {
    let files = org_index::queries::list_files(&state.pool, &state.workspace_key).await?;
    let file = files
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| ApiError::not_found(format!("file not found: {id}")))?;
    let nodes = org_index::queries::list_nodes_for_file(&state.pool, &file.id).await?;
    Ok(Json(FileDetailDto {
        id: file.id,
        path: file.path,
        version: file.content_hash,
        nodes: nodes.iter().map(NodeDto::from).collect(),
    }))
}

pub async fn get_file_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, ApiError> {
    let files = org_index::queries::list_files(&state.pool, &state.workspace_key).await?;
    let file = files
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| ApiError::not_found(format!("file not found: {id}")))?;
    let rel = state.workspace.rel_path(&file.path)?;
    Ok(state.workspace.read(&rel)?.contents)
}
