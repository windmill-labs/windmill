#!/usr/bin/env bash
./gen_wm_client-mac.sh
./windmill-utils-internal/gen_wm_client-mac.sh
deno run -A dnt.ts
