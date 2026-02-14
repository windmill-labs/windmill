#!/usr/bin/env bash

# Makes windmill-yaml-validator source files Deno-compatible by:
# 1. Adding .ts extensions to relative imports
# 2. Adding `with { type: "json" }` to JSON imports
# Use -r to restore (undo changes).

set -e
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

RESTORE_MODE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -r)
            RESTORE_MODE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-r]"
            echo "  -r    Restore original imports"
            exit 1
            ;;
    esac
done

if [[ "$OSTYPE" == "darwin"* ]]; then
    SED=gsed
    if ! command -v gsed &> /dev/null; then
        echo "Error: gsed not found. Run: brew install gnu-sed"
        exit 1
    fi
else
    SED=sed
fi

if [[ "$RESTORE_MODE" == true ]]; then
    echo "Restoring original imports..."
    find "$script_dirpath"/src -name "*.ts" -type f ! -path '*__tests__*' | while read -r file; do
        # Remove .ts from relative imports: from "./foo.ts" -> from "./foo"
        $SED -E -i 's|(from "\.\.?/[^"]*)\.ts(")|\1\2|g' "$file"
        # Remove ` with { type: "json" }` from JSON imports
        $SED -E -i 's/ with \{ type: "json" \}//' "$file"
    done
    echo "✓ Restored original imports"
else
    echo "Making sources Deno-compatible..."
    find "$script_dirpath"/src -name "*.ts" -type f ! -path '*__tests__*' | while read -r file; do
        # Add .ts to relative imports that don't already end in .ts or .json
        $SED -E -i '/\.json"/! { /\.ts"/! s|(from "(\.\.?/[^"]*[^/]))"(;?)$|\1.ts"\3|; }' "$file"
        # Add `with { type: "json" }` to .json imports that don't already have it
        $SED -E -i '/with \{ type: "json" \}/! s/(from "[^"]*\.json")(;?)$/\1 with { type: "json" }\2/' "$file"
    done
    echo "✓ Sources are now Deno-compatible"
fi
