#!/bin/bash
set -e

npm ci --ignore-scripts
npx --yes openapi-typescript-codegen --input ../backend/openapi.yaml \
 --output ./src --useOptions \
 && sed -i '213 i \\    request.referrerPolicy = \"no-referrer\"\n' src/core/request.ts
npx --yes denoify
rm -rf windmill-api
mv deno_dist windmill-api
rm -rf src/
