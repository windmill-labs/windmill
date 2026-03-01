#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

VERSION=$1
echo "Updating versions to: $VERSION"

sed -i '' -e "/^version =/s/= .*/= \"$VERSION\"/" ${root_dirpath}/backend/Cargo.toml
sed -i '' -e "/^export const VERSION =/s/= .*/= \"v$VERSION\";/"  ${root_dirpath}/cli/src/main.ts
sed -i '' -e "/^export const VERSION =/s/= .*/= \"v$VERSION\";/" ${root_dirpath}/benchmarks/lib.ts
sed -i '' -e "/version: /s/: .*/: $VERSION/" ${root_dirpath}/backend/windmill-api/openapi.yaml
sed -i '' -e "/version: /s/: .*/: $VERSION/" ${root_dirpath}/openflow.openapi.yaml
sed -i '' -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/typescript-client/package.json
sed -i '' -e "/\"version\": /s/: .*,/: \"$VERSION\",/" ${root_dirpath}/frontend/package.json
sed -i '' -e "/^version =/s/= .*/= \"$VERSION\"/" ${root_dirpath}/python-client/wmill/pyproject.toml
sed -i '' -e "/^windmill-api =/s/= .*/= \"\\^$VERSION\"/" ${root_dirpath}/python-client/wmill/pyproject.toml
sed -i '' -e "/^[[:space:]]*ModuleVersion[[:space:]]*=/s/= .*/= '$VERSION'/" ${root_dirpath}/powershell-client/WindmillClient/WindmillClient.psd1
sed -i '' -e "/^wmill =/s/= .*/= \">=$VERSION\"/" ${root_dirpath}/lsp/Pipfile

sed -i '' -E "s/name = \"windmill\"\nversion = \"[^\"]*\"\\n(.*)/name = \"windmill\"\nversion = \"$VERSION\"\\n\\1/" ${root_dirpath}/backend/Cargo.lock

cd ${root_dirpath}/frontend && npm i --package-lock-only
