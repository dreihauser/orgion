//! The indexing pipeline: `.org` files on disk -> `org-parser` AST ->
//! index DB rows. This is the only writer of the content-model tables
//! (`files`, `nodes`, `node_properties`, `links`) — see
//! docs/data-model.md. Always safe to re-run from scratch
//! (`reindex_workspace` wipes and rebuilds), which is what makes the DB a
//! disposable cache rather than a second source of truth.

use crate::model::TimestampRecord;
use chrono::Utc;
use org_model::{Node, Planning};
use org_parser::NodeSpans;
use org_storage::Workspace;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("storage error: {0}")]
    Storage(#[from] org_storage::StorageError),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A stable-ish node identity: derived from the `:ID:` property when
/// present, otherwise from `(workspace_key, file path, byte position)` —
/// see docs/data-model.md's explicit caveat that the latter form is not
/// stable across edits that shift byte offsets.
pub(crate) fn node_uuid(workspace_key: &str, file_rel: &str, node: &Node, position: usize) -> Uuid {
    let seed = match &node.org_id {
        Some(oid) => format!("orgion:id:{oid}"),
        None => format!("orgion:pos:{workspace_key}:{file_rel}:{position}"),
    };
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes())
}

pub(crate) fn file_uuid(workspace_key: &str, file_rel: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("orgion:file:{workspace_key}:{file_rel}").as_bytes(),
    )
}

struct PendingNode {
    id: Uuid,
    parent_id: Option<Uuid>,
    file_id: Uuid,
    org_id: Option<String>,
    level: i64,
    position: i64,
    title: String,
    todo_state: Option<String>,
    todo_type: Option<String>,
    priority: Option<String>,
    tags: Vec<String>,
    scheduled: Option<TimestampRecord>,
    deadline: Option<TimestampRecord>,
    closed_date: Option<chrono::NaiveDate>,
    properties: Vec<(String, String)>,
    body: String,
    links: Vec<org_model::Link>,
}

fn planning_field(p: &Planning, pick: impl Fn(&Planning) -> &Option<org_model::Timestamp>) -> Option<TimestampRecord> {
    pick(p).as_ref().map(TimestampRecord::from)
}

fn flatten(
    nodes: &[Node],
    spans: &[NodeSpans],
    counter: &mut usize,
    parent_id: Option<Uuid>,
    file_id: Uuid,
    workspace_key: &str,
    file_rel: &str,
    out: &mut Vec<PendingNode>,
) {
    for node in nodes {
        let span = &spans[*counter];
        let position = span.heading_line.start;
        let id = node_uuid(workspace_key, file_rel, node, position);

        let mut links = node.links.clone();
        // Resolve `id:` link kind textual target against later lookups —
        // resolution to a concrete node id happens in a second pass once
        // every node in the workspace has been assigned an id.
        links.retain(|l| !l.target_raw.is_empty());

        out.push(PendingNode {
            id,
            parent_id,
            file_id,
            org_id: node.org_id.clone(),
            level: node.level as i64,
            position: position as i64,
            title: node.title.clone(),
            todo_state: node.todo_state.clone(),
            todo_type: node.todo_type.map(|t| match t {
                org_model::TodoType::Todo => "todo".to_string(),
                org_model::TodoType::Done => "done".to_string(),
            }),
            priority: node.priority.map(|c| c.to_string()),
            tags: node.tags.clone(),
            scheduled: planning_field(&node.planning, |p| &p.scheduled),
            deadline: planning_field(&node.planning, |p| &p.deadline),
            closed_date: node.planning.closed.as_ref().map(|t| t.date),
            properties: node.properties.clone(),
            body: node.body.clone(),
            links,
        });
        *counter += 1;
        flatten(
            &node.children,
            spans,
            counter,
            Some(id),
            file_id,
            workspace_key,
            file_rel,
            out,
        );
    }
}

/// Wipe and rebuild every content-model row belonging to
/// `workspace_key` from the files currently on disk under `workspace`.
/// `workspace_key` is an opaque caller-chosen string identifying this
/// workspace in a multi-workspace deployment (e.g. the `Workspace.id`
/// from the platform model) — it only namespaces rows here, it is not
/// interpreted.
pub async fn reindex_workspace(
    pool: &SqlitePool,
    workspace: &Workspace,
    workspace_key: &str,
) -> Result<(), IndexError> {
    let mut tx = pool.begin().await?;

    // Deleting files cascades to nodes/node_properties/links via FK
    // (foreign_keys pragma is enabled in db::connect).
    sqlx::query("DELETE FROM files WHERE workspace_root = ?1")
        .bind(workspace_key)
        .execute(&mut *tx)
        .await?;

    let mut all_nodes: Vec<PendingNode> = Vec::new();
    let mut org_id_to_uuid: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();

    for rel in workspace.list_org_files()? {
        let content = workspace.read(&rel)?;
        let parsed = org_parser::parse(&content.contents);
        let file_id = file_uuid(workspace_key, rel.as_str());

        sqlx::query(
            "INSERT INTO files (id, workspace_root, path, content_hash, indexed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(file_id.to_string())
        .bind(workspace_key)
        .bind(rel.as_str())
        .bind(&content.version)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await?;

        let mut counter = 0usize;
        let start_len = all_nodes.len();
        flatten(
            &parsed.doc.nodes,
            &parsed.spans,
            &mut counter,
            None,
            file_id,
            workspace_key,
            rel.as_str(),
            &mut all_nodes,
        );
        for n in &all_nodes[start_len..] {
            if let Some(oid) = &n.org_id {
                org_id_to_uuid.insert(oid.clone(), n.id);
            }
        }
    }

    for n in &all_nodes {
        sqlx::query(
            "INSERT INTO nodes (
                id, file_id, parent_id, org_id, level, position, title,
                todo_state, todo_type, priority, tags,
                scheduled_date, scheduled_time, scheduled_repeater, scheduled_active,
                deadline_date, deadline_time, deadline_repeater, deadline_active,
                closed_date, properties_ordered, body
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
        )
        .bind(n.id.to_string())
        .bind(n.file_id.to_string())
        .bind(n.parent_id.map(|u| u.to_string()))
        .bind(&n.org_id)
        .bind(n.level)
        .bind(n.position)
        .bind(&n.title)
        .bind(&n.todo_state)
        .bind(&n.todo_type)
        .bind(&n.priority)
        .bind(serde_json::to_string(&n.tags)?)
        .bind(n.scheduled.as_ref().map(|t| t.date.to_string()))
        .bind(n.scheduled.as_ref().and_then(|t| t.time).map(|t| t.to_string()))
        .bind(n.scheduled.as_ref().and_then(|t| t.repeater.clone()))
        .bind(n.scheduled.as_ref().map(|t| t.active as i64))
        .bind(n.deadline.as_ref().map(|t| t.date.to_string()))
        .bind(n.deadline.as_ref().and_then(|t| t.time).map(|t| t.to_string()))
        .bind(n.deadline.as_ref().and_then(|t| t.repeater.clone()))
        .bind(n.deadline.as_ref().map(|t| t.active as i64))
        .bind(n.closed_date.map(|d| d.to_string()))
        .bind(serde_json::to_string(&n.properties)?)
        .bind(&n.body)
        .execute(&mut *tx)
        .await?;

        for (key, value) in &n.properties {
            sqlx::query("INSERT INTO node_properties (node_id, key, value) VALUES (?1,?2,?3)")
                .bind(n.id.to_string())
                .bind(key)
                .bind(value)
                .execute(&mut *tx)
                .await?;
        }
    }

    for n in &all_nodes {
        for link in &n.links {
            let target_kind = match link.target_kind {
                org_model::LinkKind::Id => "id",
                org_model::LinkKind::File => "file",
                org_model::LinkKind::Web => "web",
                org_model::LinkKind::Other => "other",
            };
            let target_node_id = if link.target_kind == org_model::LinkKind::Id {
                org_id_to_uuid.get(&link.target_raw).map(|u| u.to_string())
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO links (id, source_node_id, target_kind, target_raw, target_node_id) VALUES (?1,?2,?3,?4,?5)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(n.id.to_string())
            .bind(target_kind)
            .bind(&link.target_raw)
            .bind(target_node_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}
