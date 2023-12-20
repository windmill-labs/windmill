#!/bin/bash
set -eou pipefail

./build.sh
rm client.ts
tsc
cp src/client.ts .
npm publish
