#!/usr/bin/env bash

set -e

if [ -z "$1" ]; then
    name="wmill"
else
    name="$1"
fi

./gen_wm_client.sh 
./windmill-utils-internal/gen_wm_client.sh

echo "Installing dev cli as $name (pass arg to override)"
deno install -f -A -g src/main.ts --name $name --unstable