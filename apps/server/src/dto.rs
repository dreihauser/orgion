//! JSON shapes returned by the API, matching docs/api.md. Mostly a
//! straight pass-through of `org_index::IndexedNode`, except properties
//! are exposed as a `{"KEY": "value"}` object (last-write-wins) rather
//! than the ordered `Vec<(String, String)>` the index keeps internally,
//! to match the documented API contract.

use org_index::{AgendaItem, IndexedNode, TimestampRecord};
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize)]
pub struct NodeDto {
    pub id: String,
    pub org_id: Option<String>,
    pub file_id: String,
    pub file_path: String,
    pub parent_id: Option<String>,
    pub level: i64,
    pub title: String,
    pub todo_state: Option<String>,
    pub todo_type: Option<String>,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub scheduled: Option<TimestampRecord>,
    pub deadline: Option<TimestampRecord>,
    pub closed: Option<TimestampRecord>,
    pub properties: Map<String, Value>,
    pub body: String,
    pub version: String,
}

impl From<&IndexedNode> for NodeDto {
    fn from(n: &IndexedNode) -> Self {
        let mut properties = Map::new();
        for (k, v) in &n.properties {
            properties.insert(k.clone(), Value::String(v.clone()));
        }
        NodeDto {
            id: n.id.clone(),
            org_id: n.org_id.clone(),
            file_id: n.file_id.clone(),
            file_path: n.file_path.clone(),
            parent_id: n.parent_id.clone(),
            level: n.level,
            title: n.title.clone(),
            todo_state: n.todo_state.clone(),
            todo_type: n.todo_type.clone(),
            priority: n.priority.clone(),
            tags: n.tags.clone(),
            scheduled: n.scheduled.clone(),
            deadline: n.deadline.clone(),
            closed: n.closed.clone(),
            properties,
            body: n.body.clone(),
            version: n.version.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgendaItemDto {
    #[serde(flatten)]
    pub node: NodeDto,
    pub agenda_date: chrono::NaiveDate,
    pub agenda_kind: &'static str,
}

impl From<&AgendaItem> for AgendaItemDto {
    fn from(item: &AgendaItem) -> Self {
        AgendaItemDto {
            node: NodeDto::from(&item.node),
            agenda_date: item.agenda_date,
            agenda_kind: match item.agenda_kind {
                org_index::AgendaKind::Scheduled => "scheduled",
                org_index::AgendaKind::Deadline => "deadline",
            },
        }
    }
}
