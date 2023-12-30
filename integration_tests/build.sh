#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

source ${script_dirpath}/common.sh

docker pull ${WM_IMAGE}:${WM_VERSION}
docker pull ${WM_IMAGE}:${WM_VERSION_DEV}

previous_version=${WM_VERSION}
custom_version="${WM_VERSION}-dev"

echo "Previous version was: ${previous_version}"
${root_dirpath}/.github/change-versions.sh ${custom_version}

# Publishing Windmill SDK package to private NPM registry
npm install -g npm-cli-login
docker compose up npm_registry -d
sleep 2
curl -vvv -XPUT -H "Content-type: application/json" -d '{ "name": "windmill", "password": "changeme" }' 'http://localhost:4873/-/user/org.couchdb.user:windmill' | jq -r .token
npm-cli-login -u windmill -p changeme -e admin@windmill.dev -r http://localhost:4873
cd ${root_dirpath}/typescript-client
${root_dirpath}/typescript-client/publish.sh '--registry http://localhost:4873'
cd ${script_dirpath}
docker compose down

# TODO: publish Python SDK to private PyPI registry
  
${root_dirpath}/.github/change-versions.sh ${previous_version}
