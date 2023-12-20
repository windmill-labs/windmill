#!/bin/bash
set -eou pipefail

npx --yes openapi-typescript-codegen --input ../backend/windmill-api/openapi.yaml \
 --output ./src --useOptions \
 && sed -i '213 i \\    request.referrerPolicy = \"no-referrer\"\n' src/core/request.ts 

cp client.ts src/
cp s3Types.ts src/
echo 'export { type S3Object } from "./s3Types";' >> src/index.ts
echo 'export { setClient, getVariable, setVariable, getResource, setResource, getResumeUrls, setState, getState, denoS3LightClientSettings } from "./client";' >> src/index.ts
