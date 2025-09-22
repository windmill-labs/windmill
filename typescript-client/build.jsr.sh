#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/dist"

npx --yes @hey-api/openapi-ts@0.43.0  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/src" --useOptions --schemas false

# Add explicit type as it's missing in the client source code. This will be unnecessary in newer openapi-ts versions
sed -i 's/get \[Symbol\.toStringTag\]() {/get \[Symbol\.toStringTag\]() : string {/g' "${script_dirpath}/src/core/CancelablePromise.ts"

cp "${script_dirpath}/client.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/s3Types.ts" "${script_dirpath}/src/"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export type { S3Object, DenoS3LightClientSettings } from "./s3Types";' >> "${script_dirpath}/src/index.ts"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export { type Base64, setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, setProgress, getProgress, getState, getIdToken, denoS3LightClientSettings, loadS3FileStream, loadS3File, writeS3File, signS3Objects, signS3Object, task, runScript, runScriptAsync, runScriptByPath, runScriptByHash, runScriptByPathAsync, runScriptByHashAsync, runFlow, runFlowAsync, waitJob, getRootJobId, setFlowUserState, getFlowUserState, usernameToEmail, requestInteractiveSlackApproval, Sql, requestInteractiveTeamsApproval, appendToResultStream, streamResult } from "./client";' >> "${script_dirpath}/src/index.ts"


