#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/dist"

npx --yes @hey-api/openapi-ts  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/dist" --useOptions --schemas false

# Add explicit type as it's missing in the client source code. This will be unnecessary in newer openapi-ts versions
sed -i 's/get \[Symbol\.toStringTag\]() {/get \[Symbol\.toStringTag\]() : string {/g' "${script_dirpath}/dist/core/CancelablePromise.ts"

