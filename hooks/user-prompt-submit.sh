#!/usr/bin/env bash
# FixOnce — user-prompt-submit hook
#
# Called when the user submits a prompt.
# Injects relevant memories as context for the agent.
#
# Claude Code passes the prompt text via stdin or the CLAUDE_PROMPT env var.
# We forward it to the fixonce binary which reads it from stdin.
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

timeout 3 fixonce hook user-prompt-submit 2>/dev/null || true
