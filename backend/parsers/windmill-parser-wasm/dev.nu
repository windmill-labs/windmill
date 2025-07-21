#!/usr/bin/env nu
 
# Build in debug mode specified lang parser to wasm 
# and perform installation to frontend
def "main" [
  lang: string # Example: nu
] {
  ./build.nu $lang --no-opt
  (
    cd ../../../frontend; npm install ../backend/parsers/windmill-parser-wasm/pkg-($lang)
  )
}
