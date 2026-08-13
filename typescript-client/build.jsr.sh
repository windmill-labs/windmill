#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/dist"

npx --yes @hey-api/openapi-ts@0.43.0  --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" --output "${script_dirpath}/src" --useOptions --schemas false

# Add explicit type as it's missing in the client source code. This will be unnecessary in newer openapi-ts versions
sed -i 's/get \[Symbol\.toStringTag\]() {/get \[Symbol\.toStringTag\]() : string {/g' "${script_dirpath}/src/core/CancelablePromise.ts"

cp "${script_dirpath}/client.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/wacError.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/s3Types.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/sqlUtils.ts" "${script_dirpath}/src/"
# Two JSR-only rules, enforced by `jsr publish` (which publish.jsr.sh runs
# without --allow-slow-types) and so not reachable before a release tag:
# a type must be re-exported as `type X`, or deno fails with TS1205 under
# isolatedModules; and an exported function whose return type deno cannot
# trivially infer needs an explicit annotation, or it is a "slow type".
# `./build.jsr.sh && deno publish --dry-run` checks both.
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export type { DenoS3LightClientSettings } from "./s3Types";' >> "${script_dirpath}/src/index.ts"
echo "" >> "${script_dirpath}/src/index.ts"
echo 'export { type Base64, setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, setProgress, getProgress, getState, getIdToken, denoS3LightClientSettings, cancelJob, loadS3FileStream, loadS3File, writeS3File, deleteS3File, signS3Objects, signS3Object, getPresignedS3PublicUrls, getPresignedS3PublicUrl, task, runScript, runScriptAsync, runScriptByPath, runScriptByHash, runScriptByPathAsync, runScriptByHashAsync, runFlow, runFlowAsync, waitJob, getRootJobId, setFlowUserState, getFlowUserState, usernameToEmail, requestInteractiveSlackApproval, type Sql, requestInteractiveTeamsApproval, appendToResultStream, streamResult, datatable, ducklake, upsertPartition, appendPartition, type DucklakeMaterializeOptions, type SqlStatement, type DatatableSqlTemplateFunction, type SqlTemplateFunction, type S3Object, type S3ObjectRecord, type S3ObjectURI } from "./client";' >> "${script_dirpath}/src/index.ts"


