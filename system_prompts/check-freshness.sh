#!/usr/bin/env bash
# Check that auto-generated system prompts are up-to-date with their sources.
# Usage: bash system_prompts/check-freshness.sh
# Exit code 0 = fresh, 1 = stale (with diff printed)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."
GENERATED_DIR="$SCRIPT_DIR/auto-generated"
CLI_SKILLS="$ROOT_DIR/cli/src/guidance/skills.ts"

# Snapshot current state
BEFORE=$(git -C "$ROOT_DIR" diff -- "$GENERATED_DIR" "$CLI_SKILLS")
UNTRACKED_BEFORE=$(git -C "$ROOT_DIR" ls-files --others --exclude-standard -- "$GENERATED_DIR")

# Regenerate
echo "Running generate.py..."
python3 "$SCRIPT_DIR/generate.py"

# Compare
AFTER=$(git -C "$ROOT_DIR" diff -- "$GENERATED_DIR" "$CLI_SKILLS")
UNTRACKED_AFTER=$(git -C "$ROOT_DIR" ls-files --others --exclude-standard -- "$GENERATED_DIR")

if [ "$BEFORE" = "$AFTER" ] && [ "$UNTRACKED_BEFORE" = "$UNTRACKED_AFTER" ]; then
    echo "Auto-generated system prompts are up-to-date."
    exit 0
else
    echo "ERROR: Auto-generated system prompts are stale!"
    echo "Run 'python3 system_prompts/generate.py' and commit the result."
    echo ""
    echo "Diff:"
    git -C "$ROOT_DIR" diff -- "$GENERATED_DIR" "$CLI_SKILLS"
    if [ "$UNTRACKED_BEFORE" != "$UNTRACKED_AFTER" ]; then
        echo ""
        echo "New untracked files:"
        git -C "$ROOT_DIR" ls-files --others --exclude-standard -- "$GENERATED_DIR"
    fi
    exit 1
fi
