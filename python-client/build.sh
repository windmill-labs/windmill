#!/bin/bash
set -e

cd ../wmill && poetry build
cd ../wmill_pg && poetry build
cd .. && echo "windmill-api/" >> .gitignore 
