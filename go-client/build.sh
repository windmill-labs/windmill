#!/bin/bash
set -e

cp  ../backend/openapi.yaml openapi.yaml

sed -z 's/                    extra_params:\n                      additionalProperties:\n                        type: string/                    extra_params: {}/' openapi.yaml > openapi1.yaml
sed -z 's/                          enum: \[script, failure, trigger, command\]//' openapi1.yaml > openapi2.yaml

npx @redocly/openapi-cli@latest bundle openapi2.yaml --ext json > openapi-bundled.json

rm -rf api/ || true
mkdir -p api
~/go/bin/oapi-codegen -old-config-style --package=windmill_api --generate=types,client  openapi-bundled.json > api/windmill_api.gen.go
rm openapi*
