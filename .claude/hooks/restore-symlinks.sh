#!/bin/bash
# Restore _ee.rs symlinks after Claude finishes processing
# This script runs when Claude stops
# IMPORTANT: Copies any modifications back to the target before restoring symlinks

set -e

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-/home/farhad/windmill}"
MANIFEST_FILE="$PROJECT_DIR/.claude/hooks/.symlink-manifest"

# Check if manifest exists
if [[ ! -f "$MANIFEST_FILE" ]]; then
    exit 0
fi

# Read manifest and restore symlinks
while IFS='|' read -r symlink target; do
    if [[ -n "$symlink" && -n "$target" ]]; then
        # If the file exists (not a symlink) and target exists, copy changes back
        if [[ -f "$symlink" && ! -L "$symlink" && -e "$target" ]]; then
            # Copy the potentially modified file back to the target
            cp "$symlink" "$target"
        fi

        # Remove the regular file (which was a copy)
        rm -f "$symlink" 2>/dev/null || true

        # Recreate the symlink
        ln -s "$target" "$symlink" 2>/dev/null || true
    fi
done < "$MANIFEST_FILE"

# Clean up manifest
rm -f "$MANIFEST_FILE"

exit 0
