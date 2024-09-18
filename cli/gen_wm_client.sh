set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/gen"

npx --yes @hey-api/openapi-ts@0.53.0  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/gen" --useOptions --client fetch  --schemas false 
cat <<EOF - gen/core/OpenAPI.ts > temp_file && mv temp_file gen/core/OpenAPI.ts
const getEnv = (key: string) => {
  if (typeof window === "undefined") {
    if (typeof process !== "undefined") {
      return process?.env?.[key];
    }
    // node
    return globalThis?.process?.env?.[key];
  }
  // browser
  return window?.process?.env?.[key];
};

const baseUrl = getEnv("BASE_INTERNAL_URL") ?? getEnv("BASE_URL") ?? "http://localhost:8000";
const baseUrlApi = (baseUrl ?? '') + "/api";

EOF
sed -i 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' gen/core/OpenAPI.ts
sed -i 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' gen/core/OpenAPI.ts
sed -i "s/BASE: '\/api'/BASE: baseUrlApi/g" gen/core/OpenAPI.ts