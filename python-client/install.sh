#!/bin/bash
set -e

rm  windmill-api/dist/* || true
rm  wmill/dist/* || true

./build.sh

pip3 install windmill-api/dist/*.whl --force-reinstall --user --break-system-packages
pip3 install wmill/dist/*.whl --force-reinstall  --user --break-system-packages

rm -rf windmill-api/
rm -rf wmill/dist/*
