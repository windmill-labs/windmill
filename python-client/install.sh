#!/bin/bash
set -e

rm  windmill-api/dist/* || true
rm  wmill/dist/* || true
rm  wmill_pg/dist/* || true

./build.sh

pip3 install windmill-api/dist/*.whl --force-reinstall --user --break-system-packages
pip3 install wmill/dist/*.whl --force-reinstall  --user --break-system-packages
pip3 install wmill_pg/dist/*.whl --force-reinstall --user --break-system-packages

rm -rf windmill-api/
rm -rf wmill/dist/*
rm -rf install wmill_pg/dist/*
