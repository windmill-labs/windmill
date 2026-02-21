#!/usr/bin/env bash

# Set script to exit on any error
set -e

# Generate client files
./gen_wm_client.sh

# Generate utils client files
./windmill-utils-internal/gen_wm_client.sh

# Install dependencies
bun install

# Build npm package with bun
echo "Building npm package..."
bun run build-npm.ts

echo "Build complete!"
