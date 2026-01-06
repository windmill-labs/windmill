#!/usr/bin/env bash

set -e

# Script to generate minified OpenFlow JSON and Zod schemas
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_file="${script_dirpath}/../windmill-yaml-validator/src/gen/openflow.json"
output_dirpath="${script_dirpath}/src/lib/components/copilot/chat/flow"
json_output_file="${output_dirpath}/openFlow.json"
zod_output_file="${output_dirpath}/openFlowZod.ts"

echo "Generating OpenFlow JSON and Zod schemas..."

if [ ! -f "${source_file}" ]; then
    echo "Error: Source file not found: ${source_file}"
    echo "Please run windmill-yaml-validator/gen_openflow_schema.sh first"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "${output_dirpath}"

node -e "
const { jsonSchemaToZod } = require('json-schema-to-zod');
const fs = require('fs');

const sourceFile = '${source_file}';
const jsonOutputFile = '${json_output_file}';
const zodOutputFile = '${zod_output_file}';

const openApiSchema = JSON.parse(fs.readFileSync(sourceFile, 'utf8'));
const definitions = openApiSchema.components?.schemas || {};

// === 1. Generate minified OpenFlow JSON ===
const minified = JSON.stringify(openApiSchema);
fs.writeFileSync(jsonOutputFile, minified);

const originalSize = fs.statSync(sourceFile).size;
const minifiedSize = fs.statSync(jsonOutputFile).size;
const savings = ((originalSize - minifiedSize) / originalSize * 100).toFixed(1);

console.log('✓ Minified OpenFlow JSON generated');
console.log('  Original: ' + (originalSize / 1024).toFixed(1) + ' KB → Minified: ' + (minifiedSize / 1024).toFixed(1) + ' KB (saved ' + savings + '%)');

// === 2. Generate Zod schema ===
// Inline \$refs, treating circular references with distinct placeholders
function inlineRefs(obj, seenRefs = new Set()) {
    if (typeof obj !== 'object' || obj === null) return obj;
    if (Array.isArray(obj)) return obj.map(item => inlineRefs(item, seenRefs));

    if (obj['\$ref']) {
        const match = obj['\$ref'].match(/#\\/components\\/schemas\\/(.+)\$/);
        if (match) {
            const refName = match[1];
            if (seenRefs.has(refName)) {
                // Mark circular ref with placeholder based on which schema is being referenced
                // FlowModule is for loops (modules array), FlowModuleValue is for AI agent tools
                return { type: 'string', const: '__CIRCULAR_REF_' + refName.toUpperCase() + '__' };
            }
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

// Generate FlowModuleValue schema first (needed for AI agent tools)
const inlinedValueSchema = inlineRefs(definitions.FlowModuleValue, new Set(['FlowModuleValue']));
let valueZodCode = jsonSchemaToZod(inlinedValueSchema, { name: 'flowModuleValueSchema', module: 'esm' });

// Generate FlowModule schema (the full module with id wrapper)
const inlinedSchema = inlineRefs(definitions.FlowModule, new Set(['FlowModule']));
let zodCode = jsonSchemaToZod(inlinedSchema, { name: 'flowModuleSchema', module: 'esm' });

// Remove the import line from valueZodCode to get just the schema definition
// The import looks like: import { z } from \"zod\"\\n\\n
let valueSchemaExport = valueZodCode.replace(/^import[^\\n]*\\n\\n?/, '');

// Replace circular reference placeholders with appropriate z.lazy() calls in valueZodCode
valueSchemaExport = valueSchemaExport
    .replace(/z\\.literal\\(\"__CIRCULAR_REF_FLOWMODULE__\"\\)/g, 'z.lazy(() => flowModuleSchema)')
    .replace(/z\\.literal\\(\"__CIRCULAR_REF_FLOWMODULEVALUE__\"\\)/g, 'z.lazy(() => flowModuleValueSchema)');

// Replace circular reference placeholders in zodCode
// FlowModule is used for loops (modules array)
zodCode = zodCode.replace(/z\\.literal\\(\"__CIRCULAR_REF_FLOWMODULE__\"\\)/g, 'z.lazy(() => flowModuleSchema)');
// FlowModuleValue is used for AI agent tool values
zodCode = zodCode.replace(/z\\.literal\\(\"__CIRCULAR_REF_FLOWMODULEVALUE__\"\\)/g, 'z.lazy(() => flowModuleValueSchema)');

zodCode = zodCode.replace('from \"zod\"', 'from \"zod/v3\"');

// Insert the valueSchemaExport before flowModuleSchema definition
zodCode = zodCode.replace(
    'export const flowModuleSchema',
    valueSchemaExport + '\\n\\nexport const flowModuleSchema'
);

zodCode += '\\n\\nexport const flowModulesSchema = z.array(flowModuleSchema)\\n';

fs.writeFileSync(zodOutputFile, zodCode);
console.log('✓ Generated Zod schema: ' + zodOutputFile);
"

echo "Done!"
