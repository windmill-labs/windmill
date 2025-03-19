#!/usr/bin/env bash
set -eou pipefail

# Flags are needed to build correctly with C#
# NOTE: Currently does not work with nix nor nixos
CFLAGS_wasm32_unknown_unknown="-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin" RUSTFLAGS="-Zwasm-c-abi=spec" deno task wasmbuild --inline --out ../../../cli/wasm/  -p windmill-parser-wasm --all-features


