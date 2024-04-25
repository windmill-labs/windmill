#!/bin/bash
set -eou pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

args=${1:-}

npx jsr publish ${args}
