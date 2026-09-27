use crate::model::Backlink;
use crate::row::{map_row, NODE_JOIN_FILE};
use sqlx::SqlitePool;

pub async fn backlinks(pool: &SqlitePool, node_id: &str) -> Result<Vec<Backlink>, sqlx::Error> {
    let sql = format!(
        "{NODE_JOIN_FILE} WHERE n.id IN (SELECT source_node_id FROM links WHERE target_node_id = ?1)"
    );
    let rows = sqlx::query(&sql).bind(node_id).fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let source_node = map_row(row)?;
            let context_snippet = source_node
                .body
                .lines()
                .find(|l| l.contains("[["))
                .unwrap_or(&source_node.title)
                .trim()
                .to_string();
            Ok(Backlink {
                source_node,
                context_snippet,
            })
        })
        .collect()
}
