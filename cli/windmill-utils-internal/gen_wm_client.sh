#!/usr/bin/env bash

# Set script to exit on any error
set -e

script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
output_dirpath="${script_dirpath}/src/gen"

rm -rf "${output_dirpath}"

npx --yes @hey-api/openapi-ts@0.53.1  --input "${script_dirpath}/../../backend/windmill-api/openapi.yaml" --output "${output_dirpath}" --useOptions --client legacy/fetch  --schemas false 
cat <<EOF - "${output_dirpath}/core/OpenAPI.ts" > temp_file && mv temp_file "${output_dirpath}/core/OpenAPI.ts"
const getEnv = (key: string) => {
  return process.env[key]
};

const baseUrl = getEnv("BASE_INTERNAL_URL") ?? getEnv("BASE_URL") ?? "http://localhost:8000";
const baseUrlApi = (baseUrl ?? '') + "/api";

EOF

echo "Applying sed transformations..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Detected macOS - using GNU sed (gsed)"
    if ! command -v gsed &> /dev/null; then
        echo "Error: gsed not found"
        echo "Run: brew install gnu-sed"
        exit 1
    fi
    gsed -i 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' "${output_dirpath}/core/OpenAPI.ts"
    gsed -i 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' "${output_dirpath}/core/OpenAPI.ts"
    gsed -i "s/BASE: '\\/api'/BASE: baseUrlApi/g" "${output_dirpath}/core/OpenAPI.ts"
else
    echo "Detected Linux - using GNU sed"
    sed -i 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' "${output_dirpath}/core/OpenAPI.ts"
    sed -i 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' "${output_dirpath}/core/OpenAPI.ts"
    sed -i "s/BASE: '\\/api'/BASE: baseUrlApi/g" "${output_dirpath}/core/OpenAPI.ts"
fi

echo "✓ Client generation completed"