#!/bin/bash
set -e

cp  ../backend/openapi.yaml openapi.yaml

npx @redocly/openapi-cli@latest bundle openapi.yaml > openapi-bundled.yaml

sed -z 's/FlowModuleValue:/FlowModuleValue2:/' openapi-bundled.yaml  > openapi-decycled.yaml
echo "    FlowModuleValue: {}" >> openapi-decycled.yaml
npx @redocly/openapi-cli@latest bundle openapi-decycled.yaml --ext json -d > openapi-deref.json


rm -rf windmill-api/ || true
mkdir -p windmill_api
oapi-codegen -package windmill_api openapi-deref.json > windmill_api/windmill_api.gen.go
rm openapi*
