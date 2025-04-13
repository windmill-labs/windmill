#!/usr/bin/env bash
set -eou pipefail

#-# bun and deno
OUT_DIR="../../../cli/wasm/ts" 
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "ts-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-ts"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# sql languages, graphql and bash/powershell, since they all use regex
OUT_DIR="../../../cli/wasm/regex"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR \
	--features "sql-parser,graphql-parser,bash-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-regex"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# python
OUT_DIR="../../../cli/wasm/python"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "py-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-py"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# go
OUT_DIR="../../../cli/wasm/go"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "go-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-go"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# php
OUT_DIR="../../../cli/wasm/php"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "php-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-php"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# rust
OUT_DIR="../../../cli/wasm/rust"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "rust-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-rust"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# ansible
OUT_DIR="../../../cli/wasm/yaml"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "ansible-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-yaml"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# C# (needs some more stuff to compile C tree sitter into wasm)
OUT_DIR="../../../cli/wasm/csharp"
mkdir -p $OUT_DIR
CFLAGS_wasm32_unknown_unknown="-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin" RUSTFLAGS="-Zwasm-c-abi=spec" wasm-pack build --release --target deno --out-dir $OUT_DIR --features "csharp-parser"
# sed -i 's/"windmill-parser-wasm"/"windmill-parser-wasm-csharp"/' $OUT_DIR/package.json
rm $OUT_DIR/.gitignore

#-# Nu
OUT_DIR="../../../cli/wasm/nu"
mkdir -p $OUT_DIR
wasm-pack build --release --target deno --out-dir $OUT_DIR --features "nu-parser" \
	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
rm $OUT_DIR/.gitignore

#-# Java
OUT_DIR="../../../cli/wasm/java"
mkdir -p $OUT_DIR
CFLAGS_wasm32_unknown_unknown="-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin" RUSTFLAGS="-Zwasm-c-abi=spec" wasm-pack build --release --target deno --out-dir $OUT_DIR --features "java-parser"
rm $OUT_DIR/.gitignore
