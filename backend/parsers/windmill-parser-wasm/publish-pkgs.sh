#!/bin/bash
set -eou pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

args=${1:-}

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

pushd "pkg-rust" && npm publish ${args}
popd

pushd "pkg-yaml" && npm publish ${args}
popd

pushd "pkg-csharp" && npm publish ${args}
popd

pushd "pkg-nu" && npm publish ${args}
popd
