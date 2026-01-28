#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

args=${1:-}

rm -rf "${script_dirpath}/dist"

${script_dirpath}/build.sh
rm "${script_dirpath}/client.ts"
rm "${script_dirpath}/s3Types.ts"
rm "${script_dirpath}/sqlUtils.ts"
npm install

# Build bundled CJS for CommonJS compatibility
npx tsdown --format cjs --no-dts

# Build unbundled ESM for tree-shaking support
npx tsdown --format esm --unbundle --no-dts --no-clean

# Generate .d.ts files with tsc (clean output, no bundling)
npx tsc

cp "${script_dirpath}/src/client.ts" ${script_dirpath}
cp "${script_dirpath}/src/s3Types.ts" ${script_dirpath}
cp "${script_dirpath}/src/sqlUtils.ts" ${script_dirpath}
npm publish ${args}
