#!/bin/bash
set -eou pipefail

args=${1:-}

# bun and deno
cd "pkg-ts" && npm publish ${args}

cd "pkg-regex" && npm publish ${args}

cd "pkg-py" && npm publish ${args}

cd "pkg-go" && npm publish ${args}

cd "pkg-php" && npm publish ${args}
