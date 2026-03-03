#!/usr/bin/env bash
set -e

./build.sh

cd windmill-api && poetry publish --username __token__ --password $PYPI_PASSWORD -n || true
cd ../wmill && poetry publish --username __token__ --password $PYPI_PASSWORD -n || true

rm -rf windmill-api/
