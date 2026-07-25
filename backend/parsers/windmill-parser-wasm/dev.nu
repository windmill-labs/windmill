#!/usr/bin/env nu

# Build in debug mode specified lang parser to wasm
# and perform installation to frontend
def "main" [
  lang: string # Example: nu
] {
  ./build.nu $lang --no-opt
  # do-closures scope the cd; a bare (subexpression) leaks it to the caller,
  # which would make the second relative cd resolve from inside frontend/
  do {
    cd ../../../frontend; npm install ../backend/parsers/windmill-parser-wasm/pkg-($lang)
  }
  do {
    cd ../../../cli; bun install ../backend/parsers/windmill-parser-wasm/pkg-($lang)
  }
}
