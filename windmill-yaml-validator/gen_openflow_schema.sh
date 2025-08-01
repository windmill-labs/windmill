#!/usr/bin/env bash

script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
output_dirpath="${script_dirpath}/src/gen"

mkdir -p "${output_dirpath}"
npx @redocly/openapi-cli@latest bundle "${script_dirpath}/../openflow.openapi.yaml" --ext json > "${output_dirpath}/openflow.json"

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
    fs.writeFileSync(filePath, JSON.stringify(schema, null, 2));
    console.log('Removed discriminator mappings from openflow.json');
} catch (e) {
    console.error('Error removing discriminator mappings:', e);
}
"