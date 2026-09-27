use crate::model::{FileRecord, IndexedNode};
use crate::row::{map_row, NODE_JOIN_FILE};
use sqlx::{Row, SqlitePool};

pub async fn list_files(pool: &SqlitePool, workspace_key: &str) -> Result<Vec<FileRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, path, content_hash, indexed_at FROM files WHERE workspace_root = ?1 ORDER BY path",
    )
    .bind(workspace_key)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(FileRecord {
                id: row.try_get("id")?,
                path: row.try_get("path")?,
                content_hash: row.try_get("content_hash")?,
                indexed_at: chrono::DateTime::parse_from_rfc3339(row.try_get::<String, _>("indexed_at")?.as_str())
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    .with_timezone(&chrono::Utc),
            })
        })
        .collect()
}

pub async fn get_node(pool: &SqlitePool, node_id: &str) -> Result<Option<IndexedNode>, sqlx::Error> {
    let sql = format!("{NODE_JOIN_FILE} WHERE n.id = ?1");
    let row = sqlx::query(&sql).bind(node_id).fetch_optional(pool).await?;
    row.as_ref().map(map_row).transpose()
}

pub async fn find_node_by_org_id(
    pool: &SqlitePool,
    org_id: &str,
) -> Result<Option<IndexedNode>, sqlx::Error> {
    let sql = format!("{NODE_JOIN_FILE} WHERE n.org_id = ?1");
    let row = sqlx::query(&sql).bind(org_id).fetch_optional(pool).await?;
    row.as_ref().map(map_row).transpose()
}

pub async fn list_nodes_for_file(
    pool: &SqlitePool,
    file_id: &str,
) -> Result<Vec<IndexedNode>, sqlx::Error> {
    let sql = format!("{NODE_JOIN_FILE} WHERE n.file_id = ?1 ORDER BY n.position");
    let rows = sqlx::query(&sql).bind(file_id).fetch_all(pool).await?;
    rows.iter().map(map_row).collect()
}

pub async fn list_children(
    pool: &SqlitePool,
    parent_id: &str,
) -> Result<Vec<IndexedNode>, sqlx::Error> {
    let sql = format!("{NODE_JOIN_FILE} WHERE n.parent_id = ?1 ORDER BY n.position");
    let rows = sqlx::query(&sql).bind(parent_id).fetch_all(pool).await?;
    rows.iter().map(map_row).collect()
}
