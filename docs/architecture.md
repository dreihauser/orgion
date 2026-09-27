# Orgion Architecture

## 1. Core principle

`.org` files are the canonical data. Everything else — PostgreSQL, the search
index, the ACL tables, the WebSocket event bus — is a derived, disposable
cache that can be rebuilt from the files at any time with `orgion index
--rebuild`. If Orgion disappears, the user is left with an ordinary directory
of org-mode files that Emacs, vim, `rg`, and `git` already know how to work
with. Orgion must never become a new information silo.

This constraint drives every decision below: the database is never the
system of record for *content*. It may be the system of record for things
that have no org-mode representation (permissions, sessions, webhook
subscriptions) — see [data-model.md](data-model.md).

## 2. Data flow

```text
.org files (disk, canonical)
      │  read (file watcher / API-triggered load)
      ▼
Org Parser (crates/org-parser)      — text ⇄ AST, lossless-ish
      │  AST
      ▼
Domain Model (crates/org-model)     — Heading/Node, Property, Timestamp, ...
      │  domain events (node.created/updated/deleted, todo.changed, ...)
      ▼
Index / Query Engine (crates/org-index) — PostgreSQL (or SQLite) tables
      │  SQL
      ▼
REST / WebSocket API (apps/server)
      │  JSON
      ▼
Web Workspace (apps/web)  /  CLI  /  n8n  /  other clients
```

Writes flow the other way: an API `PATCH` on a node is turned back into an
AST mutation, the AST is serialized back to org syntax by
`org-parser::serialize`, and the file is written through
`crates/org-storage` (which enforces workspace-root sandboxing and optimistic
locking). The file watcher then sees the write, and — to avoid a redundant
reparse/rebroadcast loop — the storage layer records the hash it just wrote
so the watcher can recognize "this is our own write" vs. an external edit.

External edits (Emacs, vim, `git checkout`, a sync tool) go through the same
watcher → parser → index → WebSocket path as Orgion's own writes, which is
what makes "Emacs is a native client, the browser is another client" true in
practice and not just in the pitch.

## 3. Process / crate layout

```text
apps/server          Axum HTTP+WS server, CLI (serve/init/index/check/doctor)
apps/web             Next.js Notion-like workspace UI
crates/org-parser    .org text <-> AST. No I/O, no DB. Heavily unit tested.
crates/org-model     Domain types shared by parser, index, storage, server.
crates/org-storage   Sandboxed filesystem I/O + optimistic locking + file
                     watching (notify crate).
crates/org-index     SQL schema, indexing pipeline, query engine (agenda,
                     search, views), backlinks.
packages/api-client  Generated/typed TS client used by apps/web (and, later,
                     external tools).
packages/ui          Shared React components.
```

Everything in `crates/*` is a plain Rust library with no server or database
dependency baked into its public API where avoidable — `org-parser` in
particular has zero I/O so it can be fuzzed and unit tested in isolation
(see "Development Principles" #5 in the project brief).

## 4. Runtime architecture

```text
                    ┌──────────────────────┐
                    │      Web / PWA        │
                    │   Notion-like UI      │
                    └──────────┬────────────┘
                               │ REST / WebSocket
                    ┌──────────▼────────────┐
                    │     Orgion Core       │
                    │  (apps/server, Axum)  │
                    │                       │
                    │ Org Parser            │
                    │ Query Engine          │
                    │ Indexer               │
                    │ Search                │
                    │ Auth / ACL            │
                    │ File Watcher          │
                    │ Event System          │
                    └──────┬────────┬───────┘
                           │        │
                 ┌─────────▼───┐ ┌──▼──────────┐
                 │ .org files  │ │ PostgreSQL  │
                 │ canonical   │ │ index/cache │
                 │ (sandboxed  │ │ (SQLite OK  │
                 │ workspace   │ │  for single-│
                 │ root)       │ │  user/dev)  │
                 └─────────┬───┘ └─────────────┘
                           │
                    ┌──────▼──────┐
                    │ Emacs       │
                    │ org-mode    │
                    │ org-agenda  │
                    │ org-roam    │
                    └─────────────┘
```

## 5. Reindexing / disaster recovery

`orgion index --rebuild` (also `orgion doctor` when it detects drift) drops
and recreates every index table from the files on disk. This is a
correctness requirement, not a nice-to-have: it is the mechanism that keeps
the DB honestly a cache. It must be safe to run at any time, including while
the server is serving traffic (it runs against a shadow schema/table set and
swaps over, or takes a brief write-lock — implementation detail left to
`org-index`, but the *guarantee* — no data loss, no split-brain with the
files — is architectural).

## 6. Conflict handling (v1: optimistic locking, no CRDT)

Every read of a node/file returns a `version` (a hash of the file's bytes at
the time it was parsed). A write must present the `version` it read. If the
file has changed on disk since (Emacs edited it, another browser tab saved
it), the server rejects with `409 Conflict` and returns `{current, yours,
diff}` so the client can show a merge UI. See [api.md](api.md) for the exact
shape. CRDTs (Yjs or similar) are an explicit non-goal for v1 but the
storage layer's write path is isolated behind a trait
(`org_storage::WriteBackend`) so a CRDT-based backend can be added later
without touching the parser or index.

## 7. Permissions

Permissions are never stored inside `.org` files — see
[security.md](security.md) and [data-model.md](data-model.md) for the
`User` / `Workspace` / `WorkspaceMember` / `Resource` / `Permission` schema.
A file's *content* is portable on its own; who may see it is metadata that
lives only in the index DB, exactly like everything else that isn't
representable as org-mode text.

## 8. Why Rust for the core, TypeScript for the UI

Bulk `.org` parsing, `inotify`-based file watching, and long-running index
maintenance benefit from Rust's performance and lack of GC pauses under
sustained file-watch load, and from a memory-safe systems language for a
process that will often run unattended on a home server for months. The web
workspace is a UI problem, where React/Next.js/TanStack is the pragmatic
choice. The boundary between them is the REST/WebSocket API — nothing
Rust-specific leaks into the client contract.

## 9. Non-goals for v1 (see [mvp.md](mvp.md) for the authoritative list)

CRDT realtime collaboration, email integration, external calendar sync,
graph visualization, AI/RAG, mobile native apps, plugins. The index schema
and event system are designed so these can be added later without a
rewrite (see §15 "Graph" and §18 "Webhooks" in the project brief).
