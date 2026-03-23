#!/usr/bin/env bash
# FixOnce — pre-tool-use hook
#
# Called before a tool is used by the agent.
# Checks the proposed tool input against anti-memory patterns and warns
# when a match score exceeds 0.7.
#
# IMPORTANT: This hook is ALWAYS warn-only. It never blocks tool execution.
# The exit code is always 0 regardless of what fixonce returns.
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

timeout 3 fixonce hook pre-tool-use 2>/dev/null || true

# Always exit 0 — never block the agent.
exit 0
