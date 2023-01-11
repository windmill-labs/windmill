#!/bin/bash
set -e

npx @redocly/openapi-cli@latest bundle openapi.yaml > openapi-bundled.yaml

sed -z 's/FlowModuleValue:/FlowModuleValue2:/' openapi-bundled.yaml  > openapi-decycled.yaml
echo "    FlowModuleValue: {}" >> openapi-decycled.yaml
npx @redocly/openapi-cli@latest bundle openapi-decycled.yaml --ext json -d > openapi-deref.json

rm openapi-bundled.yaml
rm openapi-decycled.yaml