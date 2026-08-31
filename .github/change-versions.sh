#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

VERSION=$1
echo "Updating versions to: $VERSION"

sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" ${root_dirpath}/backend/Cargo.toml
sed -i -e "/^export const VERSION =/s/= .*/= \"$VERSION\";/" ${root_dirpath}/cli/src/core/constants.ts
sed -i -e "/^export const VERSION =/s/= .*/= \"v$VERSION\";/" ${root_dirpath}/benchmarks/lib.ts
sed -i -e "/version: /s/: .*/: $VERSION/" ${root_dirpath}/backend/windmill-api/openapi.yaml
sed -i -e "/version: /s/: .*/: $VERSION/" ${root_dirpath}/openflow.openapi.yaml
sed -i -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/typescript-client/package.json
sed -i -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/typescript-client/jsr.json
sed -i -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/frontend/package.json
sed -i -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/windmill-yaml-validator/package.json
sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" ${root_dirpath}/python-client/wmill/pyproject.toml
sed -i -e "/^windmill-api =/s/= .*/= \"\\^$VERSION\"/" ${root_dirpath}/python-client/wmill/pyproject.toml
sed -i -e "/^[[:space:]]*ModuleVersion[[:space:]]*=/s/= .*/= '$VERSION'/" ${root_dirpath}/powershell-client/WindmillClient/WindmillClient.psd1
sed -i -e "/^wmill =/s/= .*/= \">=$VERSION\"/" ${root_dirpath}/lsp/Pipfile

# Every workspace member is named windmill*, so rewriting their entries in place
# keeps the lockfile consistent with Cargo.toml. Never regenerate it here: a
# release must not re-resolve third-party dependencies, or a `^0` requirement
# silently pulls a breaking 0.x bump into the tag that CI then fails to build.
sed -i -zE "s/(name = \"windmill[^\"]*\"\nversion = )\"[^\"]*\"/\\1\"$VERSION\"/g" ${root_dirpath}/backend/Cargo.lock

# windmill-parser-wasm is its own workspace (excluded from the backend workspace
# because of nightly-only cargo-features), so its version lives in
# [workspace.package].
sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" ${root_dirpath}/backend/parsers/windmill-parser-wasm/Cargo.toml
sed -i -zE "s/(name = \"windmill[^\"]*\"\nversion = )\"[^\"]*\"/\\1\"$VERSION\"/g" ${root_dirpath}/backend/parsers/windmill-parser-wasm/Cargo.lock

cd ${root_dirpath}/frontend && npm i --package-lock-only --ignore-scripts

# The CLI installs this package on every `bun install`, which would otherwise rewrite the
# lockfile's version and leave a dirty tree.
cd ${root_dirpath}/windmill-yaml-validator && npm i --package-lock-only --ignore-scripts
