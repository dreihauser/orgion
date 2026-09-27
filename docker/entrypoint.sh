#!/bin/sh
# Makes `docker compose up` work out of the box against a fresh volume:
# writes orgion.toml + builds the initial index on first run, then always
# just serves. Safe to re-run — `orgion init` refuses to overwrite an
# existing config, so subsequent container restarts just serve.
set -e

CONFIG="${ORGION_CONFIG:-/data/orgion.toml}"
WORKSPACE_ROOT="${ORGION_WORKSPACE_ROOT:-/data/org}"

if [ ! -f "$CONFIG" ]; then
    orgion init "$WORKSPACE_ROOT" --config "$CONFIG"
fi

exec orgion serve --config "$CONFIG"
