#!/usr/bin/env bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/src"

npx --yes @hey-api/openapi-ts@0.43.0  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/src" --useOptions  --schemas false
cat <<EOF - src/core/OpenAPI.ts > temp_file && mv temp_file src/core/OpenAPI.ts
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
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' src/core/OpenAPI.ts
  sed -i '' 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' src/core/OpenAPI.ts
  sed -i '' "s/BASE: '\/api'/BASE: baseUrlApi/g" src/core/OpenAPI.ts
else
  sed -i 's/WITH_CREDENTIALS: false/WITH_CREDENTIALS: true/g' src/core/OpenAPI.ts
  sed -i 's/TOKEN: undefined/TOKEN: getEnv("WM_TOKEN")/g' src/core/OpenAPI.ts
  sed -i "s/BASE: '\/api'/BASE: baseUrlApi/g" src/core/OpenAPI.ts
fi



cp "${script_dirpath}/client.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/s3Types.ts" "${script_dirpath}/src/"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export type { S3Object, DenoS3LightClientSettings } from "./s3Types";' >> "${script_dirpath}/src/index.ts"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export { type Base64, setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, setProgress, getProgress, getState, getIdToken, denoS3LightClientSettings, loadS3FileStream, loadS3File, writeS3File, signS3Objects, signS3Object, task, runScript, runScriptAsync, runScriptByPath, runScriptByHash, runScriptByPathAsync, runScriptByHashAsync, runFlow, runFlowAsync, waitJob, getRootJobId, setFlowUserState, getFlowUserState, usernameToEmail, requestInteractiveSlackApproval, Sql, requestInteractiveTeamsApproval, appendToResultStream, streamResult } from "./client";' >> "${script_dirpath}/src/index.ts"
