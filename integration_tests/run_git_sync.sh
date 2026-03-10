#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source ${script_dirpath}/common.sh

echo "Setting up python environment..."
python --version
python -m venv ${script_dirpath}/.venv/
${script_dirpath}/.venv/bin/pip install -r ${script_dirpath}/requirements.txt

echo "Running git sync integration tests against ${WM_IMAGE}:${WM_VERSION}"
WM_IMAGE=${WM_IMAGE} WM_VERSION=${WM_VERSION} docker compose -f docker-compose.git-sync.yml up -d

mkdir -p ./logs
echo "" > ./logs/docker-compose-git-sync.log
docker compose -f docker-compose.git-sync.yml logs --no-color --follow &> ./logs/docker-compose-git-sync.log &

cd ${script_dirpath} && ${script_dirpath}/.venv/bin/python -m unittest -v test.git_sync_test

docker compose -f docker-compose.git-sync.yml down
