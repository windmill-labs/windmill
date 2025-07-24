#!/usr/bin/env bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/gen"

npx --yes @hey-api/openapi-ts@0.53.1  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/gen" --useOptions --client legacy/fetch  --schemas false 
cat <<EOF - gen/core/OpenAPI.ts > temp_file && mv temp_file gen/core/OpenAPI.ts
const getEnv = (key: string) => {
  return Deno.env.get(key)
};

const baseUrl = getEnv("BASE_INTERNAL_URL") ?? getEnv("BASE_URL") ?? "http://localhost:8000";
const baseUrlApi = (baseUrl ?? '') + "/api";

EOF
gsed -i 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' gen/core/OpenAPI.ts
gsed -i 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' gen/core/OpenAPI.ts
gsed -i "s/BASE: '\/api'/BASE: baseUrlApi/g" gen/core/OpenAPI.ts

find gen/ -name "*.ts" -exec gsed -i -E "s/(import.*from[[:space:]]*['\"][^'\"]+)(['\"])/\1.ts\2/g" {} \;

find gen/ -name "*.ts" -exec gsed -i -E "s/(export.*from[[:space:]]*['\"][^'\"]+)(['\"])/\1.ts\2/g" {} \;
