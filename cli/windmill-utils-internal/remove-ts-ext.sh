#!/usr/bin/env bash

# Set script to exit on any error
set -e

echo "Removing .ts extensions from imports..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Detected macOS - using BSD sed"
else
    echo "Detected Linux - using GNU sed"
fi
find . -name "*.ts" -type f | while read -r file; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS BSD sed
        if ! command -v gsed &> /dev/null; then
            echo "Error: gsed not found"
            echo "Run: brew install gnu-sed"
            exit 1
        fi
        gsed -E -i 's/(from|import)[[:space:]]+["'\'']([^"'\'']*?)\.ts(["'\''])/\1 "\2\3/g' "$file"
    else
        # Linux GNU sed
        sed -E -i 's/(from|import)[[:space:]]+["'\'']([^"'\'']*?)\.ts(["'\''])/\1 "\2\3/g' "$file"
    fi
done
echo "✓ All .ts extensions removed from import/export statements"