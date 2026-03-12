#!/usr/bin/env bash
# Check that system_prompts/auto-generated/ is up-to-date with its sources.
# Usage: bash system_prompts/check-freshness.sh
# Exit code 0 = fresh, 1 = stale (with diff printed)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATED_DIR="$SCRIPT_DIR/auto-generated"

# Snapshot current state
BEFORE=$(git -C "$SCRIPT_DIR/.." diff --no-index /dev/null /dev/null 2>/dev/null; git -C "$SCRIPT_DIR/.." diff -- "$GENERATED_DIR")
UNTRACKED_BEFORE=$(git -C "$SCRIPT_DIR/.." ls-files --others --exclude-standard -- "$GENERATED_DIR")

# Regenerate
echo "Running generate.py..."
python3 "$SCRIPT_DIR/generate.py"

# Compare
AFTER=$(git -C "$SCRIPT_DIR/.." diff -- "$GENERATED_DIR")
UNTRACKED_AFTER=$(git -C "$SCRIPT_DIR/.." ls-files --others --exclude-standard -- "$GENERATED_DIR")

if [ "$BEFORE" = "$AFTER" ] && [ "$UNTRACKED_BEFORE" = "$UNTRACKED_AFTER" ]; then
    echo "system_prompts/auto-generated/ is up-to-date."
    exit 0
else
    echo "ERROR: system_prompts/auto-generated/ is stale!"
    echo "Run 'python3 system_prompts/generate.py' and commit the result."
    echo ""
    echo "Diff:"
    git -C "$SCRIPT_DIR/.." diff -- "$GENERATED_DIR"
    if [ "$UNTRACKED_BEFORE" != "$UNTRACKED_AFTER" ]; then
        echo ""
        echo "New untracked files:"
        git -C "$SCRIPT_DIR/.." ls-files --others --exclude-standard -- "$GENERATED_DIR"
    fi
    exit 1
fi
