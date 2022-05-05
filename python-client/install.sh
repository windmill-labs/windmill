#!/bin/bash
set -e

./build.sh

pip3 install windmill-api/dist/*.whl --force-reinstall
pip3 install wmill/dist/*.whl 
pip3 install wmill_pg/dist/*.whl 

rm -rf windmill-api/
rm -rf wmill/dist/*
rm -rf install wmill_pg/dist/*
