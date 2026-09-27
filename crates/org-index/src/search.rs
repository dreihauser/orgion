//! Search: v0.1 uses SQLite `LIKE` matching over title/body/tags/
//! properties/TODO state. docs/api.md documents PostgreSQL full-text
//! search (`tsvector`) as the intended production implementation for
//! Postgres-backed deployments; this is the SQLite dev-mode fallback,
//! functionally equivalent for the MVP's scale but not what ships behind
//! Postgres.

use crate::model::{IndexedNode, SearchResult};
use crate::row::{map_row, NODE_JOIN_FILE};
use sqlx::SqlitePool;

#[derive(Debug, Default, Clone)]
pub struct SearchFilters {
    pub tag: Option<String>,
    pub todo: Option<String>,
}

pub async fn search(
    pool: &SqlitePool,
    workspace_key: &str,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    let mut sql = format!("{NODE_JOIN_FILE} WHERE f.workspace_root = ?");
    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let mut binds: Vec<String> = vec![workspace_key.to_string()];

    if !query.is_empty() {
        sql.push_str(
            " AND (n.title LIKE ? ESCAPE '\\' OR n.body LIKE ? ESCAPE '\\' \
             OR n.tags LIKE ? ESCAPE '\\' OR n.properties_ordered LIKE ? ESCAPE '\\' \
             OR n.todo_state LIKE ? ESCAPE '\\')",
        );
        for _ in 0..5 {
            binds.push(like.clone());
        }
    }
    if let Some(tag) = &filters.tag {
        sql.push_str(" AND n.tags LIKE ?");
        binds.push(format!("%\"{tag}\"%"));
    }
    if let Some(todo) = &filters.todo {
        sql.push_str(" AND n.todo_state = ?");
        binds.push(todo.clone());
    }
    sql.push_str(" ORDER BY n.title LIMIT 200");

    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = q.bind(b);
    }
    let rows = q.fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let node: IndexedNode = map_row(row)?;
            let snippet = snippet_for(&node, query);
            Ok(SearchResult { node, snippet })
        })
        .collect()
}

fn snippet_for(node: &IndexedNode, query: &str) -> String {
    if query.is_empty() {
        return node.title.clone();
    }
    let lower_body = node.body.to_lowercase();
    let lower_query = query.to_lowercase();
    if let Some(idx) = lower_body.find(&lower_query) {
        let start = floor_char_boundary(&node.body, idx.saturating_sub(30));
        let end = ceil_char_boundary(&node.body, (idx + query.len() + 30).min(node.body.len()));
        format!("...{}...", &node.body[start..end])
    } else {
        node.title.clone()
    }
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}
