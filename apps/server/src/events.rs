//! WebSocket / (future) webhook event shapes — docs/api.md "WebSocket"
//! and "Webhooks" sections. One event model, multiple transports.

use crate::dto::NodeDto;
use org_index::IndexedNode;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[serde(rename = "node.updated")]
    NodeUpdated { node: NodeDto },
    #[serde(rename = "node.created")]
    NodeCreated { node: NodeDto },
    #[serde(rename = "node.deleted")]
    NodeDeleted { node_id: String },
    #[serde(rename = "file.changed")]
    FileChanged { file_id: String, file_path: String },
}

impl WsEvent {
    pub fn node_updated(node: &IndexedNode) -> Self {
        WsEvent::NodeUpdated {
            node: NodeDto::from(node),
        }
    }

    pub fn file_changed(file_id: impl Into<String>, file_path: impl Into<String>) -> Self {
        WsEvent::FileChanged {
            file_id: file_id.into(),
            file_path: file_path.into(),
        }
    }
}
