#!/usr/bin/env bash
# PreToolUse hook: block destructive git operations when on the main branch.
# Non-git tool calls and read-only git commands pass through silently.

set -euo pipefail

input="$(cat)"
tool_name="$(echo "$input" | jq -r '.tool_name // empty')"

# Only care about Bash tool calls
[[ "$tool_name" == "Bash" ]] || exit 0

command="$(echo "$input" | jq -r '.tool_input.command // empty')"

# Only care about git write commands
if [[ "$command" =~ ^git\ (push|reset|revert|checkout|merge|rebase|commit|add) ]]; then
  branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
  if [[ "$branch" == "main" ]]; then
    echo "BLOCK: You are on the main branch. Create or switch to a feature branch first." >&2
    exit 2
  fi
fi

# Block force-push targeting main from any branch.
if [[ "$command" =~ ^git[[:space:]]+push([[:space:]]|$) ]]; then
  has_force=false
  if [[ "$command" =~ (--force([[:space:]]|=|$)|--force-with-lease|[[:space:]]-f([[:space:]]|$)) ]]; then
    has_force=true
  fi
  # `+ref` refspec syntax is also a force push.
  if [[ "$command" =~ [[:space:]]\+[A-Za-z] ]]; then
    has_force=true
  fi
  if $has_force && [[ "$command" =~ (^|[[:space:]:])\+?main([[:space:]]|$) ]]; then
    echo "BLOCK: Force-push to main is not allowed via Claude. Run it yourself if you really mean to." >&2
    exit 2
  fi
fi
