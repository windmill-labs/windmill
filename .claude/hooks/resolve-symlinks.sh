#!/bin/bash
# Resolve _ee.rs symlinks to actual files so Claude can read them
# This script runs before each user prompt is processed

set -e

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-/home/farhad/windmill}"
MANIFEST_FILE="$PROJECT_DIR/.claude/hooks/.symlink-manifest"

# Find all _ee.rs symlinks and store their targets
find "$PROJECT_DIR" -name "*_ee.rs" -type l 2>/dev/null | while read -r symlink; do
    target=$(readlink -f "$symlink" 2>/dev/null) || continue

    # Only process if target file exists
    if [[ -f "$target" ]]; then
        # Store symlink path and target in manifest
        echo "$symlink|$target" >> "$MANIFEST_FILE.tmp"

        # Replace symlink with actual file content
        rm "$symlink"
        cp "$target" "$symlink"
    fi
done

# Atomically replace manifest
if [[ -f "$MANIFEST_FILE.tmp" ]]; then
    mv "$MANIFEST_FILE.tmp" "$MANIFEST_FILE"
fi

exit 0
