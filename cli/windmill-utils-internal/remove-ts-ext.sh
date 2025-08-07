#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Parse command line arguments
USE_GSED=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -m)
            USE_GSED=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-m]"
            echo "  -m    Use gsed instead of sed"
            exit 1
            ;;
    esac
done

echo "Removing .ts extensions from imports..."
find . -name "*.ts" -type f | while read -r file; do
    if [ "$USE_GSED" = true ]; then
        if ! command -v gsed &> /dev/null; then
            echo "Error: gsed not found"
            echo "Run: brew install gnu-sed"
            exit 1
        fi
        gsed -E -i 's/(from|import)[[:space:]]+["'\'']([^"'\'']*?)\.ts(["'\''])/\1 "\2\3/g' "$file"
    else
        if ! sed -E -i '' 's/(from|import)[[:space:]]+["'\'']([^"'\'']*?)\.ts(["'\''])/\1 "\2\3/g' "$file" 2>/dev/null; then
            echo "Error: sed command failed"
            echo "This might be due to sed compatibility issues."
            echo "Try running with the -m flag to use gsed instead:"
            echo "  $0 -m"
            exit 1
        fi
    fi
done
echo "✓ All .ts extensions removed from import/export statements"