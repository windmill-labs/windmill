#!/bin/bash
# Lint a frontend file with oxlint after Claude edits it, and hand any findings back
# as context. Never blocks: the edit stands, Claude sees what to fix.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" =~ \.(ts|js|svelte)$ ]] || exit 0

# Same anchoring as format-frontend.sh: a "frontend" directory at a repo root.
REPO_ROOT="${FILE_PATH%%/frontend/*}"
[[ "$FILE_PATH" == *"/frontend/"* ]] && [[ -e "$REPO_ROOT/.git" ]] || exit 0
OXLINT="$REPO_ROOT/frontend/node_modules/.bin/oxlint"
[ -x "$OXLINT" ] || exit 0

cd "$REPO_ROOT/frontend" || exit 0
OUT=$("$OXLINT" --format=unix "$FILE_PATH" 2>&1) && exit 0

jq -n --arg out "$OUT" '{hookSpecificOutput: {hookEventName: "PostToolUse",
	additionalContext: ("oxlint found problems in the file you just edited; fix them:\n" + $out)}}'
exit 0
