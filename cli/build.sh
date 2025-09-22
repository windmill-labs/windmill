#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Generate client files
./gen_wm_client.sh

# Generate utils client files
./windmill-utils-internal/gen_wm_client.sh

# Add .ts extensions to windmill-utils-internal
./windmill-utils-internal/remove-ts-ext.sh -r

# Run dnt
echo "Running dnt..."
deno run -A dnt.ts

echo "Build complete!"