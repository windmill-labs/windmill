#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/src"

npx --yes @hey-api/openapi-ts  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/src" --useOptions

cp "${script_dirpath}/client.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/s3Types.ts" "${script_dirpath}/src/"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export type { S3Object, DenoS3LightClientSettings } from "./s3Types";' >> "${script_dirpath}/src/index.ts"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export { type Base64, setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, getState, getIdToken, denoS3LightClientSettings, loadS3FileStream, loadS3File, writeS3File, task, runScript, runScriptAsync, waitJob, getRootJobId, setFlowUserState, getFlowUserState } from "./client";' >> "${script_dirpath}/src/index.ts"
