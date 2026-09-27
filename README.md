# Orgion

**Org-mode as a data foundation for a Notion-like web workspace.**

Emacs users keep editing plain `.org` files with `org-mode`, `org-agenda`,
and `org-roam`, exactly as before. Everyone else gets a web workspace —
tables, boards, calendars, an agenda dashboard — backed by the *same*
files. Orgion is a workspace platform built **on top of** Org, not one
that merely imports it.

```
Emacs is a native client.
Browser is another client.
.org is the universal data format.
```

## Core principle

```
.org files = canonical data
database   = index / cache / permissions / search / metadata
```

PostgreSQL (or, for single-user/dev setups, SQLite) is never the only copy
of your content. Delete the database and run `orgion index` — everything
rebuilds from the files on disk. If the Orgion server disappears entirely,
you're left with an ordinary directory of `.org` files that Emacs, vim,
`git`, and `rg` already know how to work with.

See [`docs/architecture.md`](docs/architecture.md) for the full design and
[`docs/mvp.md`](docs/mvp.md) for exactly what v0.1 does and doesn't do yet.

## Status

This is v0.1: an early, working vertical slice, not a finished product.
Implemented and tested:

- **`org-parser`** — org-mode text ⇄ AST, with a byte-span-preserving
  patch/render path so editing one heading's TODO state, title, tags,
  properties, or scheduled/deadline touches only that heading's lines in
  the file (34 unit tests).
- **`org-storage`** — sandboxed filesystem access (no path traversal, no
  symlink escapes), optimistic locking (content-hash `expected_version` /
  `409 Conflict`), and an `inotify`-based file watcher that tells its own
  writes apart from external edits (18 tests, including a real
  filesystem-watch integration test).
- **`org-index`** — SQLite-backed indexing pipeline, agenda queries (with
  basic repeater expansion), `LIKE`-based search, and backlinks. Always
  rebuildable from disk (12 tests, including an end-to-end test of the
  project's "Initial Success Condition": parse → index → agenda → edit
  via the API rewrites the file → an external edit is picked up on
  reindex).
- **`apps/server`** — Axum REST API + WebSocket + `orgion` CLI
  (`init`/`serve`/`index`/`check`/`doctor`). Manually verified end-to-end,
  including the live browser test below.
- **`apps/web`** — a minimal Next.js dashboard: file sidebar, a table view
  over one file's headings with a click-to-cycle Status badge, and an
  Agenda panel (Today/Overdue/Upcoming). It builds, type-checks, and was
  exercised in a real headless-Chromium session: loading the demo file,
  clicking a Status badge (which rewrote the file on disk), and — without
  a manual reload — seeing an external `sed`-style edit to the file
  propagate into the UI over the WebSocket.

Not yet built: authentication/ACL enforcement, Kanban/Calendar views
beyond the table, backlinks UI, Quick Capture, dark mode, PostgreSQL (the
index schema is written to be portable to it, but only SQLite is wired
up), webhooks delivery, and the richer per-field `409` diff shape
docs/api.md sketches. See [`docs/mvp.md`](docs/mvp.md) for the full
MUST/SHOULD/NOT YET breakdown.

## Quickstart

### Requirements

Rust (via [Nix](#nix-dev-shell) or `rustup`) and Node 20+. No PostgreSQL
required for v0.1 — SQLite is used automatically.

### Nix dev shell

```sh
nix develop   # or: nix-shell -p cargo rustc pkg-config openssl sqlite nodejs_22
```

### Try it

```sh
mkdir -p ~/org
cat > ~/org/tasks.org <<'EOF'
#+TODO: TODO NEXT | DONE

* TODO Build Orgion
DEADLINE: <2026-10-01 Thu>

* NEXT Implement parser

* DONE Create repository
EOF

cargo run -p orgion-server --bin orgion -- init ~/org
cargo run -p orgion-server --bin orgion -- serve
```

In another terminal:

```sh
cd apps/web
npm install
NEXT_PUBLIC_ORGION_API_URL=http://localhost:3030 npm run dev
```

Open http://localhost:3000 (or whatever port `next dev` picks). Click a
task's status badge — it cycles TODO → NEXT → DONE → (none) and rewrites
`~/org/tasks.org` in place. Edit that file directly (Emacs, vim, `sed`)
and the change appears in the browser without a reload.

### Docker Compose

```sh
mkdir -p org data
docker compose up --build
```

Drops your files in `./org`, the index in `./data`. See
[`docker-compose.yml`](docker-compose.yml) and
[`docs/security.md`](docs/security.md) §8 for what to put in front of it
(Cloudflare Tunnel, Tailscale, a reverse proxy — Orgion itself only binds
to the configured host/port).

## Repository layout

```
apps/
  web/      Next.js Notion-like workspace UI
  server/   Axum HTTP+WebSocket server, orgion CLI
crates/
  org-model/    Shared domain types (Node, Timestamp, TodoKeywords, ...)
  org-parser/   org text <-> AST, zero I/O, heavily unit tested
  org-storage/  Sandboxed FS I/O, optimistic locking, file watching
  org-index/    Indexing pipeline + query engine (agenda/search/backlinks)
docs/           Design docs — start with docs/architecture.md
docker/         Dockerfiles + entrypoint
```

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — system design, data flow
- [`docs/data-model.md`](docs/data-model.md) — DB schema and its relationship to file content
- [`docs/org-mapping.md`](docs/org-mapping.md) — org syntax ⇄ UI concept mapping, `#+ORGION_*` directives
- [`docs/api.md`](docs/api.md) — REST/WebSocket API reference
- [`docs/security.md`](docs/security.md) — threat model, path sandboxing, authn/authz
- [`docs/mvp.md`](docs/mvp.md) — v0.1 scope (MUST/SHOULD/NOT YET) and build order
- [`docs/licensing.md`](docs/licensing.md) — why AGPL-3.0

## Development

```sh
cargo test --workspace         # 64 tests across org-model/parser/storage/index
cd apps/web && npx tsc --noEmit && npm run build
```

## License

AGPL-3.0-or-later — see [`LICENSE`](LICENSE) and
[`docs/licensing.md`](docs/licensing.md) for why.
