# Org ⇄ Database UI Mapping

This is the contract between org-mode syntax and the Notion-like concepts
the Web UI presents. It exists so that neither side has to guess: a given
piece of org syntax always maps to exactly one UI concept, and every UI
action always has exactly one org-syntax representation it writes back.

## 1. Core mapping table

| Org construct | Web UI concept |
|---|---|
| Heading (`* Foo`) | Database row / page |
| `:PROPERTIES:` drawer entry | Database property (typed by convention, see below) |
| Tag (`:tag1:tag2:`) | Multi-select property |
| TODO keyword (`TODO`, `NEXT`, `DONE`, ...) | Status property |
| Planning timestamp (`SCHEDULED:`, `DEADLINE:`) | Date / DateTime property |
| Plain/inactive timestamp in body (`<...>`/`[...]`) | Date mention (agenda-eligible only for active `<...>`) |
| Link (`[[id:...]]`, `[[file:...]]`, `[[https://...]]`) | Relation (id-links), file reference, or external link |
| `:ID:` property | Stable object identifier (Node.id / Node.org_id) |
| Subtree (heading + its nested headings) | Nested page (parent/child rows) |
| `#+TITLE:`, file-level keywords | File metadata |
| File itself | A "database" when it contains a `#+ORGION_VIEW:` directive, otherwise a plain document/page |

## 2. Property typing convention

Org properties are untyped text. Orgion infers a UI type per property *key*
per file (not per value) using this precedence, so the same key renders
consistently across all rows of a "database":

1. **Reserved keys** get a fixed type: `ID` -> identifier, `STATUS` ->
   select (distinct values seen = options), `PROJECT`/`OWNER`/similar
   free-text keys -> text unless overridden.
2. **`#+ORGION_PROPTYPE: KEY select|date|number|text|url`** file-level
   directive lets the user override inference explicitly. Optional; omit
   it and inference just does its best.
3. **Inference from observed values**: all-ISO-date-like -> date; all
   numeric -> number; low cardinality relative to row count (e.g. <= 10
   distinct values across >= 20 rows) -> select; otherwise -> text.

Inference is a display hint only. The stored value is always the raw
string from the drawer; changing the inferred type never rewrites the
file.

## 3. TODO keywords and `todo_type`

Orgion reads the TODO keyword sequence the same way Emacs does:

```org
#+TODO: TODO(t) NEXT(n) | DONE(d)
#+TODO: | CANCELED(c)
```

Keywords left of `|` are `todo_type = todo`; keywords right of `|` are
`todo_type = done`. If a file has no `#+TODO:` line, Orgion falls back to
the standard default sequence `TODO | DONE`. This file-level (or
workspace-level, via a config default) sequence is what populates the
Status property's *options list* in the Web UI — Orgion does not invent a
separate status vocabulary.

## 4. `#+ORGION_*` directives

Orgion introduces a small, explicit set of file-level directives to carry
view configuration. They are ordinary org keyword lines
(`#+KEYWORD: value`), which org-mode already ignores gracefully if it
doesn't recognize the keyword — so a file using them still opens cleanly
in plain Emacs/vim with no plugin, and is still fully readable as a normal
outline.

```org
#+ORGION_VIEW: table
#+ORGION_FILTER: TYPE=Paper
#+ORGION_SORT: DEADLINE ASC
#+ORGION_PROPTYPE: STATUS select
```

Rules for this directive set (non-negotiable, from the project brief):

- **Never required** for a file to be useful in Orgion. A file with zero
  directives still gets a default Table view over its top-level headings.
- **Never breaks plain org-mode.** Only `#+KEYWORD:` lines are used —
  never custom drawers, never non-standard heading syntax, never anything
  that changes how Emacs parses the file.
- **Minimal vocabulary.** Only `VIEW`, `FILTER`, `SORT`, `PROPTYPE` exist
  today; new directives require an update to this document before they
  ship.
- A view built in the Web UI without corresponding directives in the file
  is stored as a DB-only `View` row (see data-model.md) and is visibly
  labeled "not saved to file" in the UI, so users aren't misled about
  portability.

## 4a. A parity footgun worth calling out

Real org-mode only recognizes a `:PROPERTIES:...:END:` drawer when it
immediately follows the heading line (or the planning line, if present) —
a blank line in between demotes it to ordinary body text, in both Emacs
*and* Orgion's parser. This matches Emacs exactly on purpose (see
Development Principle #1, "do not reinvent org syntax"), but it means a
hand-written file with a stray blank line before `:PROPERTIES:` will
silently lose its properties in both tools, not just one. `orgion doctor`
flags this pattern (a paragraph starting with `:PROPERTIES:` that isn't a
recognized drawer) so it's not a silent trap.

## 5. What is intentionally *not* mapped

- **Org-babel source blocks, LaTeX fragments, footnotes**: parsed as opaque
  body text for v1 (preserved verbatim on write-back) — not modeled as UI
  entities. Round-trip fidelity matters more than editing them from the
  Web UI in v1.
- **Drawers other than `:PROPERTIES:` and `:LOGBOOK:`**: preserved verbatim,
  not interpreted.
- **`:LOGBOOK:` entries**: parsed read-only for the future "Page History"
  feature (§16 of the brief) but not editable from the Web UI in v1 — state
  changes go through the normal TODO-state API instead, which appends
  `:LOGBOOK:` entries the same way Emacs does when `org-log-done` is set,
  so Emacs users see consistent logging either way.

## 6. Write-back rules (Web UI edit -> file mutation)

| Web UI action | File mutation |
|---|---|
| Change Status select | Replace the TODO keyword token in the heading line; append a `:LOGBOOK:` state-change entry if the workspace has logging enabled (mirrors `org-log-done`/`org-log-into-drawer`) |
| Edit a Date property mapped to SCHEDULED/DEADLINE | Rewrite the planning line under the heading |
| Add/remove a tag | Rewrite the `:tag1:tag2:` segment of the heading line, right-aligned per standard org conventions |
| Edit a property value | Rewrite that line inside `:PROPERTIES:...:END:`; unknown/new keys are appended before `:END:` |
| Rename the row/page title | Rewrite the heading text after stars/TODO/priority, before tags |
| Reorder rows in Table/Board view | Reorders headings within their parent in the file (byte-level move) — **only when the view is over a single file**; cross-file views do not support reordering in v1 |
| Move a row to a different Board column mapped to a non-Status property | Rewrites that property's value |

Every write goes through the optimistic-locking path described in
architecture.md §6 — a stale `version` produces a `409` before any of the
above mutations are applied.
