//! Read-model types returned by `org-index` queries. These mirror
//! docs/data-model.md's `Node`/`File` tables and are what `apps/server`
//! serializes (close to 1:1) as the API's `Node`/`AgendaItem` JSON shapes
//! (docs/api.md).

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampRecord {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
    pub active: bool,
    pub repeater: Option<String>,
}

impl From<&org_model::Timestamp> for TimestampRecord {
    fn from(ts: &org_model::Timestamp) -> Self {
        TimestampRecord {
            date: ts.date,
            time: ts.time,
            active: ts.active,
            repeater: ts.repeater.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub path: String,
    pub content_hash: String,
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedNode {
    pub id: String,
    pub org_id: Option<String>,
    pub file_id: String,
    pub file_path: String,
    pub parent_id: Option<String>,
    pub level: i64,
    /// Byte offset of this node's heading line at last index time. Part
    /// of the node's identity when `org_id` is absent — see
    /// docs/data-model.md's `Node.id` caveat.
    pub position: i64,
    pub title: String,
    pub todo_state: Option<String>,
    pub todo_type: Option<String>,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub scheduled: Option<TimestampRecord>,
    pub deadline: Option<TimestampRecord>,
    pub closed: Option<TimestampRecord>,
    pub properties: Vec<(String, String)>,
    pub body: String,
    /// The owning file's current content hash — used by clients as the
    /// `expected_version` for a subsequent write (docs/architecture.md §6).
    pub version: String,
}

impl IndexedNode {
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgendaItem {
    #[serde(flatten)]
    pub node: IndexedNode,
    pub agenda_date: NaiveDate,
    pub agenda_kind: AgendaKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgendaKind {
    Scheduled,
    Deadline,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub node: IndexedNode,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Backlink {
    pub source_node: IndexedNode,
    pub context_snippet: String,
}
