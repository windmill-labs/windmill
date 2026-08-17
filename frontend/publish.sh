#!/usr/bin/env bash
# Abort if the package build fails — otherwise npm publish ships the stale
# package/ directory left over from the last successful build.
set -euo pipefail
cd "$(dirname "$0")"

npm run package
npm publish
