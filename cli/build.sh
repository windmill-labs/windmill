#!/usr/bin/env bash
# Note for mac OS users: you need to install gnu-sed with `brew install gnu-sed` and use `gsed` instead of `sed`.
./gen_wm_client.sh
deno run -A dnt.ts
