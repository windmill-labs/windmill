#!/usr/bin/env bash
set -e

mkdir openapi
cp  ../backend/windmill-api/openapi.yaml openapi/openapi.yaml

# sed -i '/                enum:/ { N; /                  \[/ { N; /"S3Storage",/ { N; /"AzureBlobStorage",/ { N; /"AzureWorkloadIdentity",/ { N; /"S3AwsOidc",/ { N; /                  \]/d; }}}}}}' openapi/openapi.yaml
# sed -z 's/                    extra_params:\n                      additionalProperties:\n                        type: string/                    extra_params: {}/' openapi/openapi.yaml > openapi/openapi3.yaml
# sed -z 's/                          enum: \[script, failure, trigger, command, approval\]//' openapi/openapi1.yaml > openapi/openapi2.yaml
# sed -z 's/                    enum: \[deno, python3, go, bash\]//' openapi/openapi2.yaml > openapi/openapi3.yaml

npx @redocly/openapi-cli@latest bundle openapi/openapi.yaml --ext json > openapi-bundled.json

cat openapi-bundled.json | jq 'del(.paths."/oauth/list_connects")' | jq 'del(.components.schemas.Job.discriminator)' > openapi-bundled-cleaned.json

rm -rf api/ || true
mkdir -p api
go install github.com/oapi-codegen/oapi-codegen/v2/cmd/oapi-codegen@v2.4.1
~/go/bin/oapi-codegen -old-config-style --package=windmill_api --generate=types,client  openapi-bundled-cleaned.json > api/windmill_api.gen.go
rm -rf openapi/
rm openapi*
