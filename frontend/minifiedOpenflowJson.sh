#!/usr/bin/env bash

set -e

# Script to generate minified OpenFlow JSON for frontend AI system prompt
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_file="${script_dirpath}/../windmill-yaml-validator/src/gen/openflow.json"
output_dirpath="${script_dirpath}/src/lib/components/copilot/chat/flow"
output_file="${output_dirpath}/openFlow.json"

echo "Generating minified OpenFlow JSON..."

# Validate source file exists
if [ ! -f "${source_file}" ]; then
    echo "Error: Source file not found: ${source_file}"
    echo "Please run windmill-yaml-validator/gen_openflow_schema.sh first"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "${output_dirpath}"

# Minify JSON by removing all whitespace
node -e "
const fs = require('fs');
const sourceFile = '${source_file}';
const outputFile = '${output_file}';

try {
    const schema = JSON.parse(fs.readFileSync(sourceFile, 'utf8'));

    // Minify: stringify without spaces
    const minified = JSON.stringify(schema);

    fs.writeFileSync(outputFile, minified);

    const originalSize = fs.statSync(sourceFile).size;
    const minifiedSize = fs.statSync(outputFile).size;
    const savings = ((originalSize - minifiedSize) / originalSize * 100).toFixed(1);

    console.log(' Minified OpenFlow JSON generated successfully');
    console.log('  Original size: ' + (originalSize / 1024).toFixed(1) + ' KB');
    console.log('  Minified size: ' + (minifiedSize / 1024).toFixed(1) + ' KB');
    console.log('  Savings: ' + savings + '%');
    console.log('  Output: ' + outputFile);
} catch (e) {
    console.error('Error minifying JSON:', e.message);
    process.exit(1);
}
"

echo "Done!"
