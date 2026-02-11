#!/usr/bin/env bash

set -euo pipefail

script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
output_dirpath="${script_dirpath}/src/gen"
tmp_dirpath="$(mktemp -d)"

cleanup() {
    rm -rf "${tmp_dirpath}"
}
trap cleanup EXIT

mkdir -p "${output_dirpath}"
mkdir -p "${output_dirpath}/triggers"

node -e "
const fs = require('fs');
const yaml = require('js-yaml');

const openflowPath = '${script_dirpath}/../openflow.openapi.yaml';
const backendPath = '${script_dirpath}/../backend/windmill-api/openapi.yaml';
const openflowOutputPath = '${output_dirpath}/openflow.json';
const backendOutputPath = '${tmp_dirpath}/backend-openapi.json';

const openflowData = yaml.load(fs.readFileSync(openflowPath, 'utf8'));
const backendData = yaml.load(fs.readFileSync(backendPath, 'utf8'));

fs.writeFileSync(openflowOutputPath, JSON.stringify(openflowData, null, 2) + '\\n');
fs.writeFileSync(backendOutputPath, JSON.stringify(backendData, null, 2) + '\\n');
"

# Remove discriminator mapping from openflow.json as it's not supported by ajv
node -e "
const fs = require('fs');
const filePath = '${output_dirpath}/openflow.json';

try {
    const schema = JSON.parse(fs.readFileSync(filePath, 'utf8'));

    function removeMapping(obj) {
    if (obj && typeof obj === 'object') {
        if (obj.discriminator?.mapping) delete obj.discriminator.mapping;
        for (const v of Object.values(obj)) removeMapping(v);
    }
    }

    removeMapping(schema);

    // Remove discriminator entirely from ToolValue as it doesn't work with allOf in FlowModuleTool
    if (schema.components?.schemas?.ToolValue?.discriminator) {
        delete schema.components.schemas.ToolValue.discriminator;
        console.log('Removed discriminator from ToolValue schema');
    }

    fs.writeFileSync(filePath, JSON.stringify(schema, null, 2));
    console.log('Removed discriminator mappings from openflow.json');
} catch (e) {
    console.error('Error removing discriminator mappings:', e);
}
"

node "${script_dirpath}/scripts/generate-resource-schemas.js" \
    "${tmp_dirpath}/backend-openapi.json" \
    "${output_dirpath}/openflow.json" \
    "${output_dirpath}"
