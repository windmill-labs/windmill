#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

args=${1:-}

rm -rf "${script_dirpath}/dist"

npm install
npm run build
npm publish ${args}
