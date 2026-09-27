# License Choice

Orgion is licensed under **AGPL-3.0-or-later** (see `LICENSE`).

## Why AGPL over Apache-2.0 or MIT

Orgion is a *service* as much as it is a library: the most likely way for
someone to derive value from it without contributing back is to run a
modified fork as a hosted product (exactly the "Notion-like workspace"
category Orgion competes in). Apache-2.0 and MIT only require attribution
and don't require a modified version to be shared at all — including a
modified version only ever offered as a network service, which never
triggers even the (weaker) copyleft of the plain GPL. AGPL closes that gap:
its §13 requires anyone who runs a modified version as a network service
to offer the corresponding source to that version's users. That's the
property the project brief calls for: if someone takes Orgion, changes it,
and hosts it for others, those users get the same freedom Orgion's own
users have.

## Comparison

| | MIT | Apache-2.0 | AGPL-3.0 |
|---|---|---|---|
| Attribution required | Yes | Yes | Yes |
| Patent grant | No | Yes (explicit) | Yes (via GPLv3 terms it incorporates) |
| Must share source of modifications | No | No | Yes, including for network use |
| Network/SaaS use triggers source-sharing | No | No | **Yes** (§13, the "AGPL clause") |
| Can be relicensed by a downstream fork | Yes (permissive) | Yes (permissive) | No (copyleft propagates) |
| Common concern raised against it | Offers no protection against closed-source SaaS forks | Same as MIT on this axis | Some companies avoid AGPL dependencies entirely, which can narrow adoption in enterprise contexts |

## What this means in practice

- Self-hosting Orgion, including modifying it for personal or internal
  organizational use without redistributing it, carries no obligation
  beyond keeping the license notice — AGPL only activates on
  distribution or network-service use of a *modified* version to parties
  outside the modifying organization.
- A company that takes Orgion, adds proprietary features, and offers it
  as a hosted product to customers must make that modified version's
  source available to those customers.
- Contributions to this repository are accepted under the same license;
  there is no CLA requiring copyright assignment.

If Orgion later ships an official plugin/extension API, plugins are
expected to be a separate concern from this license (e.g. distributed
separately, potentially under a different license, similar to how GPL'd
editors handle extensions) — not yet relevant since plugins are explicitly
out of scope for v0.1 (docs/mvp.md).
