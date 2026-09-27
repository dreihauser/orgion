-- Orgion index schema (SQLite dev-mode; kept simple enough to be
-- straightforwardly portable to PostgreSQL for production use).
--
-- Every table here is a disposable cache derived from the `.org` files on
-- disk. Nothing here is ever the only copy of user content — see
-- docs/data-model.md. `orgion index --rebuild` drops and recreates all of
-- it from scratch.

CREATE TABLE files (
    id TEXT PRIMARY KEY,
    workspace_root TEXT NOT NULL,
    path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    UNIQUE (workspace_root, path)
);

CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES nodes (id) ON DELETE CASCADE,
    org_id TEXT,
    level INTEGER NOT NULL,
    position INTEGER NOT NULL,
    title TEXT NOT NULL,
    todo_state TEXT,
    todo_type TEXT,
    priority TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    scheduled_date TEXT,
    scheduled_time TEXT,
    scheduled_repeater TEXT,
    scheduled_active INTEGER,
    deadline_date TEXT,
    deadline_time TEXT,
    deadline_repeater TEXT,
    deadline_active INTEGER,
    closed_date TEXT,
    properties_ordered TEXT NOT NULL DEFAULT '[]',
    body TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_nodes_file_id ON nodes (file_id);
CREATE INDEX idx_nodes_parent_id ON nodes (parent_id);
CREATE INDEX idx_nodes_org_id ON nodes (org_id);
CREATE INDEX idx_nodes_scheduled_date ON nodes (scheduled_date);
CREATE INDEX idx_nodes_deadline_date ON nodes (deadline_date);
CREATE INDEX idx_nodes_todo_type ON nodes (todo_type);

CREATE TABLE node_properties (
    node_id TEXT NOT NULL REFERENCES nodes (id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL
);

CREATE INDEX idx_node_properties_key_value ON node_properties (key, value);
CREATE INDEX idx_node_properties_node_id ON node_properties (node_id);

CREATE TABLE links (
    id TEXT PRIMARY KEY,
    source_node_id TEXT NOT NULL REFERENCES nodes (id) ON DELETE CASCADE,
    target_kind TEXT NOT NULL,
    target_raw TEXT NOT NULL,
    target_node_id TEXT REFERENCES nodes (id)
);

CREATE INDEX idx_links_source ON links (source_node_id);
CREATE INDEX idx_links_target ON links (target_node_id);
