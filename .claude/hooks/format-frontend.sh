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
        # Run prettier, surface errors as context but don't block Claude
        if ./node_modules/.bin/prettier --plugin prettier-plugin-svelte --write "$FILE_PATH" 2>&1; then
            echo "Formatted $(basename "$FILE_PATH")"
        fi
    fi
fi

exit 0
