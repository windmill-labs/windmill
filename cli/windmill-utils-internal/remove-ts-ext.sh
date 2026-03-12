#!/usr/bin/env bash

# Set script to exit on any error
set -e
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse command line arguments
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
            echo "  -r    Restore .ts extensions to imports/exports"
            exit 1
            ;;
    esac
done

if [[ "$RESTORE_MODE" == true ]]; then
    echo "Adding .ts extensions to imports..."
    # Only add .ts if the path doesn't already end with .ts or /
    # Also skip node: built-in module imports
    REGEX='/\.ts["'\'']/! { /["'\''"]node:/! s/(from|import)[[:space:]]+["'\'']([^"'\'']*[^/])(["'\''])/\1 "\2.ts\3/g; }'
    SUCCESS_MSG="✓ All .ts extensions added to import/export statements"
else
    echo "Removing .ts extensions from imports..."
    REGEX='s/(from|import)[[:space:]]+["'\'']([^"'\'']*?)\.ts(["'\''])/\1 "\2\3/g'
    SUCCESS_MSG="✓ All .ts extensions removed from import/export statements"
fi

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Detected macOS - using GNU sed (gsed)"
    if ! command -v gsed &> /dev/null; then
        echo "Error: gsed not found"
        echo "Run: brew install gnu-sed"
        exit 1
    fi
else
    echo "Detected Linux - using GNU sed"
fi

find "$script_dirpath"/src -name "*.ts" -type f | while read -r file; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        gsed -E -i "$REGEX" "$file"
    else
        sed -E -i "$REGEX" "$file"
    fi
done

echo "$SUCCESS_MSG"