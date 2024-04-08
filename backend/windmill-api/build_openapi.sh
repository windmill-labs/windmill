#!/bin/bash
set -e

npx @redocly/openapi-cli@latest bundle openapi.yaml > openapi-bundled.yaml


npx @redocly/openapi-cli@latest bundle openapi-bundled.yaml --ext yaml -d > openapi-deref.yaml
npx @redocly/openapi-cli@latest bundle openapi-bundled.yaml --ext json > openapi-deref.json


rm openapi-bundled.yaml
