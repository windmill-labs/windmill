#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

args=${1:-}

# bun and deno
pushd "pkg-ts" && npm publish ${args}
popd

pushd "pkg-regex" && npm publish ${args}
popd

pushd "pkg-py" && npm publish ${args}
popd

pushd "pkg-go" && npm publish ${args}
popd

pushd "pkg-php" && npm publish ${args}
popd
