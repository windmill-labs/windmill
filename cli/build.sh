#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Generate client files
./gen_wm_client.sh

# Generate utils client files
./windmill-utils-internal/gen_wm_client.sh

# Set up trap to ensure cleanup happens on exit, error, or interruption
trap 'echo "Cleaning up..."; ./add-ts-ext.sh -r' EXIT ERR INT TERM

# Add .ts extensions for Deno
./add-ts-ext.sh

# Run dnt
echo "Running dnt..."
deno run -A dnt.ts

echo "Build complete!"