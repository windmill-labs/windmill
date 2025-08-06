#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Default to regular sed
REVERT_MODE=false

# Function to show usage
show_usage() {
    echo "Usage: $0 [-r] [-h]"
    echo "  -r    Revert changes (restore from .orig files)"
    echo "  -h    Show this help message"
    exit 1
}

# Parse command line arguments
while getopts "rh" opt; do
    case ${opt} in
        r )
            REVERT_MODE=true
            ;;
        h )
            show_usage
            ;;
        \? )
            echo "Invalid option: -$OPTARG" >&2
            show_usage
            ;;
    esac
done

# Function to add .ts extensions to relative imports
add_ts_extensions() {
    echo "Adding .ts extensions to imports..."
    find windmill-utils-internal/src -name "*.ts" -type f | while read -r file; do
        # Create backup of original
        cp "$file" "$file.orig"
        
        # Add .ts to relative imports that don't already have extensions
        sed -E \
            -e 's/(from[[:space:]]+["'"'"'])(\.[^"'"'"']*[^./][^"'"'"']*)(["'"'"'])/\1\2.ts\3/g' \
            -e 's/(import[[:space:]]+["'"'"'])(\.[^"'"'"']*[^./][^"'"'"']*)(["'"'"'])/\1\2.ts\3/g' \
            "$file.orig" > "$file"
    done
    echo "✓ Added .ts extensions"
}

# Function to revert to original files
revert_extensions() {
    echo "Removing .ts extensions..."
    find windmill-utils-internal/src -name "*.orig" -type f | while read -r backup_file; do
        original_file="${backup_file%.orig}"
        mv "$backup_file" "$original_file"
    done
    echo "✓ All files reverted"
}

# Main execution
if [ "$REVERT_MODE" = true ]; then
    revert_extensions
else
    add_ts_extensions
fi
