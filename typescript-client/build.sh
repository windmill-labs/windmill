#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "${script_dirpath}/src"

npx --yes openapi-typescript-codegen --input "${script_dirpath}/../backend/windmill-api/openapi.yaml" \
 --output "${script_dirpath}/src" --useOptions \
 && sed -i '213 i \\    request.referrerPolicy = \"no-referrer\"\n' src/core/request.ts

cp "${script_dirpath}/client.ts" "${script_dirpath}/src/"
cp "${script_dirpath}/s3Types.ts" "${script_dirpath}/src/"
echo 'export type { S3Object, DenoS3LightClientSettings } from "./s3Types";' >> "${script_dirpath}/src/index.ts"
echo 'export { setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, getState, denoS3LightClientSettings } from "./client";' >> "${script_dirpath}/src/index.ts"
