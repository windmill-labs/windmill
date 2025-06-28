#!/bin/bash
set -eou pipefail

#-# full pkg
OUT_DIR="pkg"
wasm-pack build --release --target web --out-dir $OUT_DIR --all-features \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort

#-# bun and deno
OUT_DIR="pkg-ts"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "ts-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-ts"/' $OUT_DIR/package.json

#-# sql languages, graphql and bash/powershell, since they all use regex
OUT_DIR="pkg-regex"
wasm-pack build --release --target web --out-dir $OUT_DIR \
	--features "sql-parser,graphql-parser,bash-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-regex"/' $OUT_DIR/package.json

#-# python
OUT_DIR="pkg-py"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "py-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-py"/' $OUT_DIR/package.json

#-# go
OUT_DIR="pkg-go"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "go-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-go"/' $OUT_DIR/package.json

#-# php
OUT_DIR="pkg-php"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "php-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-php"/' $OUT_DIR/package.json

#-# rust
OUT_DIR="pkg-rust"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "rust-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-rust"/' $OUT_DIR/package.json

#-# ansible
OUT_DIR="pkg-yaml"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "ansible-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-yaml"/' $OUT_DIR/package.json

#-# C# (needs some more stuff to compile C tree sitter into wasm)
# TODO: hasn't been tested on mac, might need fixing
OUT_DIR="pkg-csharp"
CFLAGS_wasm32_unknown_unknown="-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin" RUSTFLAGS="-Zwasm-c-abi=spec" wasm-pack build --release --target web --out-dir $OUT_DIR --features "csharp-parser"
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-csharp"/' $OUT_DIR/package.json

#-# nu
# TODO: hasn't been tested on mac, might need fixing
OUT_DIR="pkg-nu"
wasm-pack build --release --target web --out-dir $OUT_DIR --features "nu-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-nu"/' $OUT_DIR/package.json

#-# Java (needs some more stuff to compile C tree sitter into wasm)
OUT_DIR="pkg-java"
CFLAGS_wasm32_unknown_unknown="-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin" RUSTFLAGS="-Zwasm-c-abi=spec" wasm-pack build --release --target web --out-dir $OUT_DIR --features "java-parser"
sed -i '' 's/"windmill-parser-wasm"/"windmill-parser-wasm-java"/' $OUT_DIR/package.json
