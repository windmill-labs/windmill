#!/bin/bash
# Format frontend files with prettier after Claude edits them

# Get the file path from the tool result (passed via stdin as JSON)
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Exit if no file path
if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Check if the file is in the frontend directory
if [[ "$FILE_PATH" == *"/frontend/"* ]]; then
    # Check if it's a formattable file type
    if [[ "$FILE_PATH" =~ \.(ts|js|svelte|json|css|html|md)$ ]]; then
        cd "$CLAUDE_PROJECT_DIR/frontend" || exit 0
        # Run prettier silently, don't fail the hook if prettier fails
        npx prettier --write "$FILE_PATH" 2>/dev/null || true
    fi
fi

exit 0
