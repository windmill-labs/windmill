#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Setting up python environment..."
python3 --version
python3 -m venv ${script_dirpath}/.venv/
${script_dirpath}/.venv/bin/pip install -r ${script_dirpath}/requirements.txt

echo "Running git sync integration tests"
cd ${script_dirpath} && ${script_dirpath}/.venv/bin/python -m unittest -v test.git_sync_test
