#!/usr/bin/env nu
 
# Build in debug mode specified lang parser to wasm 
# and perform installation to frontend
# TODO: It builds in release mode for some reason
def "main" [ lang: string ] {
  let out_dir = $'pkg-($lang)'
  open build-pkgs.sh
    | split row  '#-'
    | find $out_dir  
    | str replace "--release" ""
    | bash -c $in.0
  (
    cd ../../../frontend;
    npm install ../backend/parsers/windmill-parser-wasm/($out_dir)
  )
}
