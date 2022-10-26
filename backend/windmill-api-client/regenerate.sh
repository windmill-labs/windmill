#!/bin/sh

swagger-cli bundle ../windmill-api/openapi.yaml | openapi-generator generate -g rust -o ./ -i /dev/stdin --global-property apis --global-property models --global-property supportingFiles="mod.rs:lib.rs" --global-property modelDocs=false