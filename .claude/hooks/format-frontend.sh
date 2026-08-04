#!/bin/bash
# Format frontend files with prettier after Claude edits them

# Get the file path from the tool result (passed via stdin as JSON)
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Exit if no file path
if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Only the frontend app itself, i.e. a "frontend" directory sitting at a repo root.
# A bare */frontend/* substring also matches ai_evals/adapters/frontend and the
# ai_evals app fixtures, which no prettier config governs — prettier then falls back
# to its defaults and rewrites the whole file. Anchoring to $CLAUDE_PROJECT_DIR
# instead would skip worktrees edited from a session rooted elsewhere.
if [[ "$FILE_PATH" == *"/frontend/"* ]] && [[ -e "${FILE_PATH%%/frontend/*}/.git" ]]; then
    # Check if it's a formattable file type
    if [[ "$FILE_PATH" =~ \.(ts|js|svelte|json|css|html|md)$ ]]; then
        cd "$CLAUDE_PROJECT_DIR/frontend" || exit 0
        # Run prettier, surface errors as context but don't block Claude
        if ./node_modules/.bin/prettier --plugin prettier-plugin-svelte --write "$FILE_PATH" 2>&1; then
            echo "Formatted $(basename "$FILE_PATH")"
        fi
    fi
fi

exit 0
