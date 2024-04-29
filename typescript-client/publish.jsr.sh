#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
${script_dirpath}/build.jsr.sh

args=${1:-}

npx jsr publish ${args}
