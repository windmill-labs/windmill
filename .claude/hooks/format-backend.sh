#!/bin/bash
# Format backend Rust files with rustfmt after Claude edits them

# Get the file path from the tool result (passed via stdin as JSON)
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Exit if no file path
if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Check if the file is in the backend directory and is a Rust file
if [[ "$FILE_PATH" == *"/backend/"* ]] && [[ "$FILE_PATH" =~ \.rs$ ]]; then
    cd "$CLAUDE_PROJECT_DIR/backend" || exit 0
    # Run rustfmt, surface errors as context but don't block Claude
    if rustfmt --config-path rustfmt.toml "$FILE_PATH" 2>&1; then
        echo "Formatted $(basename "$FILE_PATH")"
    fi
fi

exit 0
