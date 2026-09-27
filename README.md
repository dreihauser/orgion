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
  basic repeater expansion), `LIKE`-based search, backlinks, and local
  account/session/workspace management (Argon2id password hashing,
  hashed session tokens). Always rebuildable from disk (17 tests,
  including an end-to-end test of the project's "Initial Success
  Condition": parse → index → agenda → edit via the API rewrites the
  file → an external edit is picked up on reindex).
- **`apps/server`** — Axum REST API + WebSocket + `orgion` CLI
  (`init`/`serve`/`index`/`check`/`doctor`). Cookie-based session auth
  guards every route except `/api/auth/*` (toggleable — see Quickstart).
  Manually verified end-to-end, including the live browser test below.
- **`apps/web`** — a Next.js workspace UI: login page, a Dashboard
  landing page (agenda summary, stats, recent files), a directory-tree
  file sidebar (scrollable, independent of the fixed nav above it), a
  table view over one file's headings with a click-to-cycle Status
  badge, and a Settings page with a Light/Dark/System theme picker. It
  builds, type-checks, and was exercised in a real headless-Chromium
  session end to end: unauthenticated redirect → login → dashboard →
  tree navigation → an external `sed`-style file edit propagating into
  the UI over WebSocket without a reload → theme persistence across a
  reload → logout.

Not yet built: ACL/sharing beyond a single admin workspace, Kanban/
Calendar views beyond the table, backlinks UI, Quick Capture, PostgreSQL
(the index schema is written to be portable to it, but only SQLite is
wired up), webhooks delivery, OIDC/reverse-proxy auth, and the richer
per-field `409` diff shape docs/api.md sketches. See
[`docs/mvp.md`](docs/mvp.md) for the full MUST/SHOULD/NOT YET breakdown.

## Quickstart

Nothing here is tied to a particular OS or package manager — just a
standard Rust + Node toolchain, on Linux, macOS, or Windows.

### Requirements

- **Rust**, via [rustup](https://rustup.rs). This repo ships a
  [`rust-toolchain.toml`](rust-toolchain.toml), so once rustup is
  installed, running `cargo` anywhere in the project auto-installs and
  uses the pinned toolchain — no separate setup step.
- **Node 20+** ([`.nvmrc`](.nvmrc) pins the version for nvm/fnm/Volta
  users; any Node 20+ install works).

No PostgreSQL required for v0.1 — SQLite is used automatically.

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
```

This prints a generated admin username/password once — save it. Then:

```sh
cargo run -p orgion-server --bin orgion -- serve
```

In another terminal:

```sh
cd apps/web
npm install
npm run dev
```

Open http://localhost:3000, log in with the credentials `init` printed,
and you'll land on the Dashboard. Click a task's status badge in the
Workspace view — it cycles TODO → NEXT → DONE → (none) and rewrites
`~/org/tasks.org` in place. Edit that file directly (Emacs, vim, `sed`)
and the change appears in the browser without a reload.

`next dev` proxies `/api/*` to `orgion serve` on `localhost:3030` by
default (see `apps/web/next.config.mjs`); set `ORGION_API_ORIGIN` if
it's running elsewhere. Set `[auth] enabled = false` in `orgion.toml` to
skip the login screen for a quick local trial.

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
rust-toolchain.toml   Pins the Rust toolchain for `rustup` (any OS)
.nvmrc                Pins the Node version for nvm/fnm/Volta (any OS)
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
cargo test --workspace         # 69 tests across org-model/parser/storage/index
cd apps/web && npx tsc --noEmit && npm run build
```

## License

AGPL-3.0-or-later — see [`LICENSE`](LICENSE) and
[`docs/licensing.md`](docs/licensing.md) for why.
