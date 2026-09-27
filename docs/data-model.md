# Orgion Data Model

Two data models coexist and must be kept conceptually separate:

1. **Content model** — derived entirely from `.org` files, rebuildable at
   any time. Lives in the index DB only as a cache.
2. **Platform model** — has no org-mode representation (users, permissions,
   sessions, webhooks, views' saved filters when they're not expressed as
   `#+ORGION_*` directives). Lives in the DB as the system of record.

Losing the content model's tables is an inconvenience (`orgion index
--rebuild` fixes it). Losing the platform model's tables loses real data
(who can see what, saved logins) — back it up like a normal database.

## 1. Content model (derived from files)

### `File`

| field         | type      | notes                                   |
|---------------|-----------|------------------------------------------|
| id            | uuid      | stable, generated on first index         |
| workspace_id  | uuid      | FK -> Workspace                          |
| path          | text      | relative to workspace root, canonical    |
| content_hash  | text      | sha256 of current on-disk bytes          |
| mtime         | timestamp | last observed mtime                      |
| indexed_at    | timestamp | last successful reindex                  |

### `Node` (an org heading, or the file's zeroth section)

| field         | type       | notes |
|---------------|------------|-------|
| id            | uuid       | from the `:ID:` property if present, else a deterministic uuid derived from `(file_id, position)` — **unstable** until the user lets Orgion (or `org-id`) assign a real `:ID:`. Nodes without a real ID cannot be safely linked to (`[[id:...]]`) or targeted by external tools. |
| file_id       | uuid       | FK -> File |
| parent_id     | uuid?      | FK -> Node, null for top-level headings |
| org_id        | text?      | raw `:ID:` property value, if present |
| level         | int        | org heading level (1 = `*`) |
| position      | int        | byte offset in file, for stable ordering pre-ID |
| title         | text       | heading text with TODO/priority/tags stripped |
| todo_state    | text?      | e.g. `TODO`, `NEXT`, `DONE`, or null for a plain heading |
| todo_type     | enum       | `todo` \| `done` \| `none` (from the keyword's configured class) |
| priority      | text?      | `A`/`B`/`C`/... |
| tags          | text[]     | own tags only; org's inherited-tag semantics are computed at query time, not stored, so re-tagging a parent doesn't require rewriting children |
| scheduled     | timestamp? | from the planning line |
| deadline      | timestamp? | from the planning line |
| closed        | timestamp? | from the planning line |
| properties    | jsonb      | the `:PROPERTIES:` drawer as a flat map (last-write-wins per key, matching org semantics) |
| body          | text       | raw body text (everything under the heading, excluding child subtrees), kept for full-text search |

### `Link`

| field      | type | notes |
|------------|------|-------|
| id         | uuid |
| source_node_id | uuid | FK -> Node the link appears in |
| target_kind | enum | `id` \| `file` \| `heading` \| `web` \| `other` |
| target_raw | text | the raw link target, e.g. `id:3a42219d-...`, `file:foo.org` |
| target_node_id | uuid? | resolved FK -> Node, when `target_kind = id` and resolvable |

`Link` rows are what power backlinks (§14 of the brief) and, later, the
graph view (§15) — the model is already `Node`/`Link` (node/edge), so a
graph view is additive, not a schema change.

### `AgendaEntry` (materialized, not authoritative)

A query-time view over `Node`, not a separate stored table in the strict
sense — but treated as a first-class read model: "does this node have a
SCHEDULED/DEADLINE/plain timestamp in range, is it repeating, is it
overdue" is exactly org-agenda's job and is re-derived from `Node` fields
plus the recurrence rule embedded in the timestamp string
(`<2026-10-20 Tue +1w>`), never stored as separately-maintained state.

## 2. Platform model (system of record in the DB)

### `User`

id, email, display_name, password_hash (nullable if OIDC-only),
auth_provider (`local` \| `oidc` \| `proxy`), created_at.

### `Workspace`

id, name, root_path (absolute, on the server's filesystem, sandboxed — see
security.md), created_at.

### `WorkspaceMember`

workspace_id, user_id, role (`owner` \| `member` \| `viewer`), added_at.

### `Resource`

Generic pointer so permissions can attach to things at different
granularities without a table-per-kind explosion:

| field | type | notes |
|-------|------|-------|
| id | uuid |
| workspace_id | uuid |
| kind | enum | `file` \| `node` \| `view` |
| ref_id | uuid | the File.id / Node.id / View.id being protected |

### `Permission`

| field | type | notes |
|-------|------|-------|
| id | uuid |
| resource_id | uuid | FK -> Resource |
| principal | enum+id | `user:<id>` \| `workspace:<id>` (= all members) \| `public` |
| level | enum | `read` \| `comment` \| `write` \| `admin` |

Effective permission = most specific `Resource` row that matches, most
permissive `Permission` among matching principals, with an explicit `Private`
default (no rows = only the creator, via workspace `owner`/creation ACL, can
see it). See security.md for the exact resolution algorithm and the
sandboxing rule that content paths are *never* taken from client input
directly.

### `View`

Saved database view (table/board/calendar/list). Stores the filter/sort/
grouping either as a reference to a file's `#+ORGION_VIEW:` block (so it
travels with the file) or, for a view a user builds in the Web UI without
touching the file, as DB-only rows (`View.definition jsonb`). The
file-backed case is authoritative-in-the-file; the DB-only case is
explicitly a "platform model" object with no org representation, and must
be labeled as such in the UI so users aren't surprised it doesn't show up
when they open the file in Emacs.

### `Session`, `WebhookSubscription`, `AuditLogEntry`

Standard shapes; see security.md and api.md for `WebhookSubscription`'s
event-type enum (mirrors §18 of the brief: `node.created`, `node.updated`,
`node.deleted`, `todo.changed`, `deadline.changed`, `file.changed`).

## 3. Why properties are `jsonb` and not a normalized table

Org properties are an open, user-defined key/value set per heading (`:ID:`,
`:STATUS:`, `:PROJECT:`, arbitrary user keys). Normalizing them into a
`(node_id, key, value)` table is *also* reasonable and is the more
"queryable" choice for filtering (`WHERE key = 'PROJECT' AND value =
'HRI2027'`); Orgion does both: `Node.properties` jsonb holds the authoritative
denormalized copy (cheap to rebuild, easy to reserialize back to a
`:PROPERTIES:` drawer), and `org-index` additionally maintains a
`NodeProperty(node_id, key, value)` table purely as a query index over the
same data, rebuilt alongside everything else. The jsonb column is the
source of truth *within the DB layer*; the file is still the source of
truth overall.
