#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Generate client files
./gen_wm_client-mac.sh

# Generate utils client files
./windmill-utils-internal/gen_wm_client-mac.sh

# Run dnt
echo "Running dnt..."
deno run -A dnt.ts

echo "Build complete!"