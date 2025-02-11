#!/usr/bin/env bash
set -e

npm install
rm -rf dist
npm run compile
npm run test:full-headless

rm -rf dist
npm run compile
newver=$(npm version patch -m "[ci skip] %s")
git push
git push --tags

vsce publish