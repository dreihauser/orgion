use crate::dto::AgendaItemDto;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct AgendaQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct AgendaResponse {
    pub today: Vec<AgendaItemDto>,
    pub overdue: Vec<AgendaItemDto>,
    pub upcoming: BTreeMap<NaiveDate, Vec<AgendaItemDto>>,
}

pub async fn get_agenda(
    State(state): State<AppState>,
    Query(q): Query<AgendaQuery>,
) -> Result<Json<AgendaResponse>, ApiError> {
    let from = q.from.unwrap_or_else(|| chrono::Local::now().date_naive());
    let to = q.to.unwrap_or(from + chrono::Duration::days(7));

    let result = org_index::query_agenda(&state.pool, &state.workspace_key, from, to).await?;

    Ok(Json(AgendaResponse {
        today: result.today.iter().map(AgendaItemDto::from).collect(),
        overdue: result.overdue.iter().map(AgendaItemDto::from).collect(),
        upcoming: result
            .upcoming
            .iter()
            .map(|(date, items)| (*date, items.iter().map(AgendaItemDto::from).collect()))
            .collect(),
    }))
}
