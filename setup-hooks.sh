#!/bin/bash
set -euo pipefail
root_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ln -s -f ../../.githooks/pre-commit  ./.git/hooks
