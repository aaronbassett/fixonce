#!/usr/bin/env bash
# FixOnce — session-start hook
#
# Called when a Claude Code session begins.
# Surfaces critical memories for the current project environment.
#
# Edge cases:
#   EC-41: 3-second hard timeout via `timeout(1)`
#   EC-42: Skip gracefully when `fixonce` is not on PATH
#   EC-43: `fixonce` exits 0 silently when unauthenticated

# Never block Claude Code — always exit 0.
set -euo pipefail

if ! command -v fixonce >/dev/null 2>&1; then
    # EC-42: binary not found; skip silently.
    exit 0
fi

timeout 3 fixonce hook session-start 2>/dev/null || true
