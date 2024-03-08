#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

export WM_VERSION=$(cat ${root_dirpath}/version.txt)
# WM_VERSION="1.228.1"
export WM_VERSION_DEV="dev"
# WM_VERSION_DEV="1.229.0"
export WM_IMAGE="ghcr.io/windmill-labs/windmill-ee"
