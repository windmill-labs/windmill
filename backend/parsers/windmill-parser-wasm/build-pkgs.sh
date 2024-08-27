#!/bin/bash
set -eou pipefail

# full pkg
OUT_DIR="pkg"
wasm-pack build --release --target web --out-dir $OUT_DIR --all-features

# bun and deno
OUT_DIR="pkg-ts"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "ts-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-ts"/' $OUT_DIR/package.json

# sql languages and graphql
OUT_DIR="pkg-regex"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "sql-parser,graphql-parser,bash-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-regex"/' $OUT_DIR/package.json

# python, bash, php
OUT_DIR="pkg-py"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "py-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-py"/' $OUT_DIR/package.json

# go and soon rust
OUT_DIR="pkg-other"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "go-parser,php-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-other"/' $OUT_DIR/package.json
