use crate::dto::NodeDto;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BacklinkDto {
    pub source_node: NodeDto,
    pub context_snippet: String,
}

pub async fn get_backlinks(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<Vec<BacklinkDto>>, ApiError> {
    let backlinks = org_index::backlinks(&state.pool, &node_id).await?;
    Ok(Json(
        backlinks
            .iter()
            .map(|b| BacklinkDto {
                source_node: NodeDto::from(&b.source_node),
                context_snippet: b.context_snippet.clone(),
            })
            .collect(),
    ))
}
