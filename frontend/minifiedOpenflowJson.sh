#!/usr/bin/env bash

set -e

# Script to generate Zod schemas from OpenFlow OpenAPI schema
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_file="${script_dirpath}/../windmill-yaml-validator/src/gen/openflow.json"
output_file="${script_dirpath}/src/lib/components/copilot/chat/flow/openFlowZod.ts"

echo "Generating OpenFlow Zod schemas..."

if [ ! -f "${source_file}" ]; then
    echo "Error: Source file not found: ${source_file}"
    echo "Please run windmill-yaml-validator/gen_openflow_schema.sh first"
    exit 1
fi

node -e "
const { jsonSchemaToZod } = require('json-schema-to-zod');
const fs = require('fs');

const openApiSchema = JSON.parse(fs.readFileSync('${source_file}', 'utf8'));
const definitions = openApiSchema.components?.schemas || {};

// Inline \$refs, treating circular references as z.object({}).passthrough()
function inlineRefs(obj, seenRefs = new Set()) {
    if (typeof obj !== 'object' || obj === null) return obj;
    if (Array.isArray(obj)) return obj.map(item => inlineRefs(item, seenRefs));

    if (obj['\$ref']) {
        const match = obj['\$ref'].match(/#\\/components\\/schemas\\/(.+)\$/);
        if (match) {
            const refName = match[1];
            if (seenRefs.has(refName)) return { type: 'object' }; // Circular ref
            if (definitions[refName]) {
                return inlineRefs(definitions[refName], new Set([...seenRefs, refName]));
            }
        }
        return { type: 'object' };
    }

    const result = {};
    for (const [key, value] of Object.entries(obj)) {
        result[key] = inlineRefs(value, seenRefs);
    }
    return result;
}

const inlinedSchema = inlineRefs(definitions.FlowModule, new Set(['FlowModule']));

let zodCode = jsonSchemaToZod(inlinedSchema, { name: 'flowModuleSchema', module: 'esm' });
zodCode = zodCode.replace('from \"zod\"', 'from \"zod/v3\"');
zodCode += '\n\nexport const flowModulesSchema = z.array(flowModuleSchema)\n';

fs.writeFileSync('${output_file}', zodCode);
console.log('✓ Generated: ${output_file}');
"

echo "Done!"
