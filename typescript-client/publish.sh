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
npx tsdown --format esm --format cjs --dts

# Fix default export for TypeScript/Monaco compatibility
# tsdown generates "export { ..., client_d_exports as default, ... }" which doesn't work properly
# We need to replace it with a proper "export default" statement
for dts in "${script_dirpath}/dist/index.d.ts" "${script_dirpath}/dist/index.d.mts"; do
  # Remove "client_d_exports as default, " from the export statement
  sed -i 's/client_d_exports as default, //g' "$dts"
  # Add proper default export at the end
  echo "" >> "$dts"
  echo "export default client_d_exports;" >> "$dts"
done

cp "${script_dirpath}/src/client.ts" ${script_dirpath}
cp "${script_dirpath}/src/s3Types.ts" ${script_dirpath}
cp "${script_dirpath}/src/sqlUtils.ts" ${script_dirpath}
npm publish ${args}
