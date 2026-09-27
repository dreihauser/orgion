# Security

## 1. Threat model summary

Orgion is typically self-hosted on a home server or small VPS, exposed
either only on a LAN/VPN (Tailscale) or to the public internet via a
reverse proxy or Cloudflare Tunnel. The operator is usually also the sole
or primary user. The two things that matter most, in order:

1. **Never let API input escape the workspace root on disk.** A path
   traversal bug here is a full read/write compromise of the host, not
   just of Orgion's data.
2. **Never let permission checks be bypassable via a different route to
   the same data** (e.g. `/api/nodes/:id` enforcing ACL but
   `/api/search` leaking a snippet of a private node).

## 2. File path handling

Rule: **API input is never concatenated into a filesystem path.**

- Every `File` known to Orgion has a DB-assigned `id` (uuid) and a `path`
  that was discovered by the indexer walking the workspace root, never a
  path supplied by a client.
- All filesystem operations in `org-storage` take a `File.id` (or an
  already-validated internal `RelPath` newtype) and resolve it against a
  fixed `Workspace.root_path`, which is itself only ever set by an
  operator running `orgion init <dir>` or editing `orgion.toml` — never
  by an API caller.
- `RelPath` construction is the single choke point: it rejects `..`
  components, absolute paths, symlink escapes (resolved and checked
  against the canonicalized root at open time, not just lexically), and
  null bytes. Every path that reaches the filesystem layer has gone
  through this constructor — there is no second, ad-hoc path-building
  code path.
- Creating a *new* file (`POST /api/files`, if/when exposed) generates
  the filename server-side from a slug of the title plus a uniqueness
  suffix, never from a raw client-supplied filename.

## 3. AuthN

- **Local accounts**: Argon2id password hashing (via `argon2` crate),
  per-user random salt, tunable work factor in `orgion.toml`.
- **OIDC**: standard authorization-code flow; Orgion validates the
  issuer/audience and does not trust claims beyond what the ID token
  signs.
- **Reverse-proxy auth**: only trusted when `trusted_header_proxy = true`
  is explicitly set *and* the configured header name is present *and* the
  connection's peer address is in a configured trusted-proxy CIDR list.
  Without both conditions, a client-supplied version of that header is
  ignored, so a misconfiguration doesn't silently turn into an auth
  bypass, and so this mechanism composes safely with Cloudflare Access
  without hardcoding Cloudflare-specific logic (any proxy that sets a
  verified identity header works the same way).
- **Bearer tokens** (CLI/automation): random 256-bit tokens, stored
  hashed (never plaintext) in the DB, workspace-scoped, revocable,
  optionally expiring.

## 4. AuthZ

- Every read/write endpoint resolves effective permission via the
  `Resource`/`Permission` model in data-model.md *before* touching
  content — including search and agenda, which filter at the SQL level
  (`WHERE resource permission check`) rather than filtering results after
  the fact, so a bug can't leak a snippet of a private node through a
  search result.
- Default is **Private** (creator/workspace-owner only) unless explicitly
  shared to Workspace, specific Users, or a Public Link.
- **Public Link** shares are read-only by construction (no `write`/`admin`
  level is ever attachable to the `public` principal) and are per-resource,
  revocable, and (recommended default) unguessable-token-based rather than
  sequential-ID-based, so revoking is instant and enumeration doesn't work.

## 5. Web-facing hardening

- **CSRF**: session-cookie-authenticated state-changing requests require
  a `SameSite=Lax` (or `Strict`) cookie plus a custom header
  (`X-Orgion-Request: 1`) that a cross-site form post cannot set,
  following the standard "custom header" CSRF mitigation used alongside
  SameSite cookies. Bearer-token requests are CSRF-exempt by nature (no
  ambient credential).
- **XSS**: the Web UI renders org body text through a sanitizing
  markdown/org-inline renderer (allowlist of elements/attributes) — raw
  HTML embedded in an org file is never passed to `dangerouslySetInnerHTML`
  or equivalent unescaped.
- **SSRF**: nothing in Orgion fetches a client-supplied URL server-side in
  v1 (no link-preview fetcher, no image proxy). If that's added later, it
  must go through an allowlist + private-IP-range block, not ad hoc.
- **Secure cookies**: `Secure`, `HttpOnly`, `SameSite=Lax` session cookies;
  session tokens are opaque, validated server-side against the `Session`
  table (not a signed-but-unencrypted JWT holding permissions, which would
  be hard to revoke).
- **Rate limiting**: login and token-issuance endpoints are rate-limited
  per-IP and per-account (`tower-governor` or equivalent middleware);
  general API traffic gets a generous default limit to protect against
  runaway automation, not against legitimate use.
- **Audit log**: `AuditLogEntry(actor, action, resource, at, ip)` for
  auth events, permission changes, and destructive operations
  (delete file/node, revoke share). Not for every read — that's a metrics
  concern, not a security one.

## 6. Dependency / supply chain

- MVP dependency surface is deliberately small (see architecture.md /
  mvp.md "Keep dependencies minimal"). Fewer dependencies = smaller attack
  surface for a self-hosted single-box service.
- `cargo audit` and `pnpm audit` (or `npm audit`) run in CI (see
  `.github/workflows/ci.yml`).

## 7. Optimistic locking is not a security control

Worth stating explicitly: `expected_version` conflict handling
(architecture.md §6) exists for correctness (don't clobber concurrent
edits), not for authorization. A caller without `write` permission is
rejected before version comparison even happens.

## 8. Docker / deployment

- The published container runs as a non-root user.
- The workspace volume is mounted read-write only where necessary; the
  container has no other reason to write outside its data volume and
  its own `/tmp`.
- Cloudflare Tunnel / reverse proxy / Tailscale are all operator-added —
  Orgion itself binds only to the configured `host`/`port` and does not
  assume any particular exposure mechanism.
