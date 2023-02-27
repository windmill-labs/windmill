#!/bin/bash
set -e

npm ci --ignore-scripts
npx --yes openapi-typescript-codegen --input ../backend/windmill-api/openapi.yaml \
 --output ./src --useOptions \
 && sed -i '213 i \\    request.referrerPolicy = \"no-referrer\"\n' src/core/request.ts 
 npx --yes denoify
rm -rf windmill-api
mv deno_dist windmill-api
sed -i 's/buffer DENOIFY: UNKNOWN NODE BUILTIN/node:buffer/' ./windmill-api/core/request.ts 
rm -rf src/
