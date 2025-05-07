#!/usr/bin/env nu
 
# Build in debug mode specified lang parser to wasm 
# and perform installation to frontend
def "main" [
  lang: string # Example: nu
  --release(-r)
] {
  let out_dir = $'pkg-($lang)'
  if $release {
    open build-pkgs.sh
      | split row  '#-'
      | find $out_dir  
      | bash -c $"RUST_LOG=trace ($in.0)"
  } else {
    open build-pkgs.sh
      | split row  '#-'
      | find $out_dir  
      | str replace "--release" "--no-opt"
      | bash -c $"WASM_OPT=-Oz ($in.0)"
  }
  (
    cd ../../../frontend; npm install ../backend/parsers/windmill-parser-wasm/($out_dir)
  )
}
