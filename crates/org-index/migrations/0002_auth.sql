-- Platform-model tables (docs/data-model.md §2): unlike files/nodes/
-- links, these have no `.org` representation and this DB *is* their
-- system of record. Losing them loses real data (accounts, sessions) —
-- back this database up, unlike the content tables which are a
-- disposable, rebuildable cache.

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    -- Matches `[workspace].key` in orgion.toml; namespaces the content
    -- tables (files.workspace_root) this workspace owns.
    key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE workspace_members (
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'owner',
    PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE sessions (
    -- sha256 of the raw bearer/cookie token; the raw token is never
    -- stored (docs/security.md §3).
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members (user_id);
