# Orgion REST / WebSocket API

Base path: `/api`. All request/response bodies are JSON. Auth is a session
cookie (local/OIDC) or a bearer token (for CLI/n8n/AI-agent use, §17 of the
brief). All resource paths are scoped to a workspace implicitly via the
session, or explicitly via `?workspace=<id>` for callers with access to more
than one.

This document defines the MVP surface. It intentionally leaves room for
`views`, `backlinks`, and `webhooks` to grow without breaking changes —
new fields are additive, nothing here is final-final.

## Conventions

- Every `Node` and `File` response includes a `version` string (content
  hash). Write endpoints require the caller's last-seen `version` in the
  request body as `expected_version`.
- Timestamps are ISO-8601. Org's `<2026-10-20 Tue>` becomes
  `"2026-10-20"` (date-only) or `"2026-10-20T09:00:00"` (has a time part)
  plus a separate `active: bool` and, if present, a `repeater` string
  (`"+1w"`, `"++1d"`, `".+1m"`) passed through uninterpreted for the client
  to render but not required to understand.
- Errors are `{ "error": { "code": "...", "message": "..." } }` with the
  matching HTTP status.

## Files

```
GET  /api/files
     -> [{ id, path, title, node_count, updated_at }]

GET  /api/files/:id
     -> { id, path, title, raw_available: true, version, nodes: [Node] }

GET  /api/files/:id/raw
     -> text/plain, the exact on-disk bytes (escape hatch for full-fidelity
        editing / diffing; not used by the default Web UI editor which
        works node-by-node)
```

## Nodes

```
GET    /api/nodes?file=:file_id            list nodes in a file
GET    /api/nodes?parent=:node_id          list children of a node
GET    /api/nodes/:id                      -> Node

POST   /api/nodes
       { file_id, parent_id?, title, todo_state?, tags?, properties?,
         scheduled?, deadline?, position? }
       -> 201 Node
       Inserts a new heading into the file at the given parent/position
       (default: end of parent's children, or end of file).

PATCH  /api/nodes/:id
       { expected_version, title?, todo_state?, tags?, properties?,
         scheduled?, deadline? }
       -> 200 Node                    (see org-mapping.md §6 for the
                                        exact file mutation each field
                                        triggers)
       -> 409 Conflict
          { error, current: Node, yours: Node, diff: string }

DELETE /api/nodes/:id?expected_version=...
       -> 204
       -> 409 Conflict (same shape as PATCH)
```

`Node` shape:

```json
{
  "id": "uuid",
  "org_id": "3a42219d-9aac-4935",
  "file_id": "uuid",
  "parent_id": "uuid|null",
  "level": 1,
  "title": "Write HRI Paper",
  "todo_state": "TODO",
  "todo_type": "todo",
  "priority": null,
  "tags": ["paper"],
  "scheduled": null,
  "deadline": { "date": "2026-10-20", "active": true, "repeater": null },
  "properties": { "PROJECT": "HRI2027", "TYPE": "Paper", "OWNER": "keito" },
  "body": "...",
  "version": "sha256:...",
  "updated_at": "2026-09-28T10:00:00Z"
}
```

## Agenda

```
GET /api/agenda?from=2026-09-28&to=2026-10-05
    -> {
         today:   [AgendaItem],
         overdue: [AgendaItem],
         upcoming: { "2026-09-29": [AgendaItem], ... }
       }
```

`AgendaItem` = `Node` plus `{ agenda_date, agenda_kind:
"scheduled"|"deadline"|"timestamp", time?: "09:00" }`. Semantics mirror
org-agenda: a repeating entry appears on each occurrence date computed from
its repeater, not just its first timestamp; an overdue `DEADLINE` without
`DONE` stays in `overdue` until closed.

## Search

```
GET /api/search?q=HRI&tag=paper&todo=TODO&workspace=:id
    -> { results: [{ node: Node, snippet, score }] }
```

`q` searches headings/body/properties/tags/TODO/links/file names (§13 of
the brief) via PostgreSQL full-text search (`tsvector` column, maintained
by `org-index`) in v1. `tag=`, `todo=`, `property.KEY=`, and date-range
filters are additive query params, ANDed together.

## Views

```
GET  /api/views?file=:file_id           file-backed + DB-only views for a file
POST /api/views                          create a DB-only view
     { workspace_id, name, kind: "table"|"board"|"calendar"|"list",
       filter?, sort?, group_by? }
     -> 201 View
```

A file-backed view (declared via `#+ORGION_VIEW:` etc., see
org-mapping.md §4) is read-only through this API — edit it by editing the
file's directive lines. Only DB-only views accept `POST`/`PATCH`/`DELETE`.

## Backlinks

```
GET /api/backlinks/:node_id
    -> [{ source_node: Node, context_snippet }]
```

## WebSocket

```
WS /api/ws?workspace=:id
```

Server -> client events, one JSON object per message:

```json
{ "type": "node.updated", "node": Node }
{ "type": "node.created", "node": Node }
{ "type": "node.deleted", "node_id": "uuid" }
{ "type": "file.changed", "file_id": "uuid" }
```

These are the same event kinds the future webhook system (§18 of the
brief, see below) delivers over HTTP instead of a socket — one event
model, two transports.

## Webhooks (post-MVP scaffold, not implemented in v0.1)

```
POST   /api/webhooks     { url, events: ["todo.changed", "deadline.changed"] }
GET    /api/webhooks
DELETE /api/webhooks/:id
```

Event payload shape matches the WebSocket events above, plus
`{ "delivered_at", "webhook_id" }`. Not built in MVP; documented now so the
event system (`apps/server`'s internal event bus) is designed against a
stable contract from day one instead of being retrofitted.

## Auth

```
POST /api/auth/login       { email, password }        -> session cookie
POST /api/auth/logout
GET  /api/auth/me          -> { user, workspaces }
GET  /api/auth/oidc/start  -> redirect
GET  /api/auth/oidc/callback
```

Reverse-proxy auth (Cloudflare Access, or any proxy setting a trusted
header) is configured server-side (`[auth] trusted_header_proxy = true` in
`orgion.toml`) and is never inferred from a client-supplied header — see
security.md.

## CLI-facing / automation

Every endpoint above is usable by the CLI, n8n, or an AI agent with a
bearer token (`POST /api/auth/tokens`, workspace-scoped, revocable). There
is deliberately no separate "automation API" — one API, multiple
credential types.
