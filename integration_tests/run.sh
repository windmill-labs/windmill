#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

source ${script_dirpath}/common.sh

echo "Setting up python environment..."
python --version
python -m venv ${script_dirpath}/.venv/
${script_dirpath}/.venv/bin/pip install -r ${script_dirpath}/requirements.txt

echo "Running initial round of test for version ${WM_VERSION}"
WM_IMAGE=${WM_IMAGE} WM_VERSION=${WM_VERSION} docker compose up -d

mkdir -p ./logs
echo "" > ./logs/docker-compose.log
docker compose logs --no-color --follow &> ./logs/docker-compose.log &

cd ${script_dirpath} && ${script_dirpath}/.venv/bin/python -m unittest -v test

echo "Running second round of test for version ${WM_VERSION_DEV}"
WM_IMAGE=${WM_IMAGE} WM_VERSION=${WM_VERSION_DEV} docker compose up  -d
WM_VERSION_DEV=${WM_VERSION}-dev WMILL_RUNNING_DEV=1 ${script_dirpath}/.venv/bin/python -m unittest -v test
