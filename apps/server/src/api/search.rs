use crate::dto::NodeDto;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use org_index::SearchFilters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    pub tag: Option<String>,
    pub todo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultDto {
    pub node: NodeDto,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultDto>,
}

pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    let filters = SearchFilters {
        tag: q.tag,
        todo: q.todo,
    };
    let results = org_index::search(&state.pool, &state.workspace_key, &q.q, &filters).await?;
    Ok(Json(SearchResponse {
        results: results
            .iter()
            .map(|r| SearchResultDto {
                node: NodeDto::from(&r.node),
                snippet: r.snippet.clone(),
            })
            .collect(),
    }))
}
