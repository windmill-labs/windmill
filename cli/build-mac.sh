#!/usr/bin/env bash

# Generate client files
./gen_wm_client-mac.sh

# Generate utils client files
./windmill-utils-internal/gen_wm_client-mac.sh

# Function to add .ts extensions to relative imports
add_ts_extensions() {
    find windmill-utils-internal/src -name "*.ts" -type f | while read -r file; do
        # Create backup of original
        cp "$file" "$file.orig"
        
        # Add .ts to relative imports that don't already have extensions
        gsed -E \
            -e 's/(from[[:space:]]+["'"'"'])(\.[^"'"'"']*[^./][^"'"'"']*)(["'"'"'])/\1\2.ts\3/g' \
            -e 's/(import[[:space:]]+["'"'"'])(\.[^"'"'"']*[^./][^"'"'"']*)(["'"'"'])/\1\2.ts\3/g' \
            "$file.orig" > "$file"
    done
}

# Function to revert to original files
revert_extensions() {
    find windmill-utils-internal/src -name "*.orig" -type f | while read -r backup_file; do
        original_file="${backup_file%.orig}"
        mv "$backup_file" "$original_file"
    done
}

# Set up trap to ensure cleanup happens on exit, error, or interruption
trap 'echo "Cleaning up..."; revert_extensions' EXIT ERR INT TERM

# Add .ts extensions for Deno
echo "Adding .ts extensions for Deno build..."
add_ts_extensions

# Run dnt
echo "Running dnt..."
deno run -A dnt.ts

echo "Build complete!"