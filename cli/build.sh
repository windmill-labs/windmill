#!/usr/bin/env bash
./gen_wm_client.sh
./windmill-utils-internal/gen_wm_client.sh
deno run -A dnt.ts
