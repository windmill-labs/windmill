#!/bin/bash
set -e

mkdir openapi
cp  ../backend/windmill-api/openapi.yaml openapi/openapi.yaml

sed -z 's/                    extra_params:\n                      additionalProperties:\n                        type: string/                    extra_params: {}/' openapi/openapi.yaml > openapi/openapi1.yaml
sed -z 's/                          enum: \[script, failure, trigger, command\]//' openapi/openapi1.yaml > openapi/openapi2.yaml

npx @redocly/openapi-cli@latest bundle openapi/openapi2.yaml --ext json > openapi-bundled.json

rm -rf api/ || true
mkdir -p api
~/go/bin/oapi-codegen -old-config-style --package=windmill_api --generate=types,client  openapi-bundled.json > api/windmill_api.gen.go
rm -rf openapi/
rm openapi*
