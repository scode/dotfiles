#!/usr/bin/env bash
set -eo pipefail

# Derive plugin root from script location (works even if CLAUDE_PLUGIN_ROOT not set)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"

cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "CODEX_REVIEW_BIN=${PLUGIN_ROOT}/bin"
  }
}
EOF
