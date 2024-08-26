#!/bin/bash
set -eou pipefail

# full pkg
OUT_DIR="pkg"
wasm-pack build --release --target web --out-dir $OUT_DIR --all-features

# bun and deno
OUT_DIR="pkg-ts"
wasm-pack build --release --target web --out-dir $OUT_DIR --features ts-parser
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-ts"/' $OUT_DIR/package.json

# sql languages and graphql
OUT_DIR="pkg-sql"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "sql-parser,graphql-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-sql"/' $OUT_DIR/package.json

# python, bash, php
OUT_DIR="pkg-script"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "py-parser,php-parser,bash-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-script-lang"/' $OUT_DIR/package.json

# go and soon rust
OUT_DIR="pkg-compiled"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "go-parser"
sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-compiled-lang"/' $OUT_DIR/package.json
