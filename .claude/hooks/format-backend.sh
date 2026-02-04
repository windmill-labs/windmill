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
    # Run rustfmt with config from rustfmt.toml (edition=2021)
    rustfmt --config-path rustfmt.toml "$FILE_PATH" 2>/dev/null || true
fi

exit 0
