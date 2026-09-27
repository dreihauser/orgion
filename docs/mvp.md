# Orgion v0.1 — MVP Scope

## Vertical slice / success condition

From the project brief, this is the concrete bar for "v0.1 works":

1. User has `~/org/tasks.org` with a few headings (`TODO`, `NEXT`, `DONE`,
   with a `DEADLINE` and a `:PROPERTIES:` drawer on one of them).
2. `orgion serve` starts the server against that directory.
3. The Web UI shows those headings as tasks with their TODO state and
   deadline.
4. Changing a task's state in the Web UI (`TODO` -> `DONE`) rewrites the
   actual heading in `tasks.org`.
5. Editing the file externally (simulating Emacs: change `TODO` ->
   `NEXT` with a text editor / `sed`) is picked up by the file watcher and
   reflected in the Web UI without a manual refresh trigger from the
   client (server-pushed via WebSocket, or at minimum reflected on next
   poll/reload for the very first cut).

## MUST (v0.1)

- User login (local accounts to start; OIDC/proxy auth are structurally
  supported per security.md but not required to demo v0.1)
- Workspace + org directory registration (`orgion init <dir>`)
- File tree
- Org parser (`crates/org-parser`) — headings, TODO keywords, tags,
  properties, planning timestamps (SCHEDULED/DEADLINE/CLOSED), plain
  timestamps, `:ID:`, links, round-trip serialization
- Heading browser (list nodes in a file / under a parent)
- Heading editor (title, tags, properties via API; full-body rich editor
  is a stretch goal, not required for v0.1)
- TODO editing
- Tags
- Properties
- Timestamp / Deadline / Scheduled
- Agenda (today / overdue / upcoming)
- Search (headings/body/tags/TODO at minimum; full property search is a
  stretch goal for v0.1)
- Table database view
- Kanban/board database view (grouped by Status)
- Calendar database view (month grid keyed by scheduled/deadline date)
- File watcher (inotify via the `notify` crate)
- Optimistic locking (`expected_version` / `409 Conflict`)
- Basic ACL (Private / Workspace / Public Link — "Specific Users" sharing
  can land slightly after v0.1 if needed, but the schema supports it from
  day one)
- REST API
- Docker Compose

## SHOULD (v0.1, nice-to-have but not blocking)

- Backlinks
- Quick Capture (Ctrl+K -> new note/TODO/event)
- Public read-only links
- Dark mode

## NOT YET (explicitly out of scope for v0.1)

- Email integration
- Google Calendar / CalDAV / ICS sync
- CRDT-based realtime collaborative editing
- AI / RAG
- Graph visualization (index model is graph-shaped already — see
  data-model.md `Link` table — so this is additive later)
- Native mobile app
- Plugin system
- Complex automation / webhooks delivery (the event bus and API shape
  exist; actual HTTP delivery + retry/backoff is post-MVP)

## Build order (matches project brief §31)

1. Org parser / AST — `crates/org-parser` + `crates/org-model`
2. Filesystem storage layer — `crates/org-storage` (sandboxing +
   optimistic locking + file watching)
3. PostgreSQL (or SQLite for dev) index — `crates/org-index`
4. REST API — `apps/server`
5. Basic Web UI — `apps/web`
6. Agenda
7. Database views (Table -> Board -> Calendar)

This repository's first commits follow that order; see the crate/app
READMEs for what's implemented vs. stubbed at any given point.

## Dependency budget for v0.1

No Kafka, no Elasticsearch, no Redis, no Kubernetes. The whole system runs
as: Web (Next.js) + Backend (Rust/Axum) + PostgreSQL + filesystem. SQLite
is an accepted swap-in for Postgres in single-user/dev mode (same schema,
via `sqlx`), not an additional moving part.
