#!/bin/bash
set -eou pipefail

deno task wasmbuild --out ../../../cli/wasm/
