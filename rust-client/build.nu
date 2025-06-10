#! /usr/bin/env nu

let v = git rev-parse --abbrev-ref HEAD
let dir = $"local/($v)/a/b/"

def main [] {
  mkdir $dir

  # http get $"https://raw.githubusercontent.com/windmill-labs/windmill/($v)/openflow.openapi.yaml"
  # | save -f $"local/($v)/openflow.openapi.yaml";

  # http get $"https://raw.githubusercontent.com/windmill-labs/windmill/($v)/backend/windmill-api/openapi.yaml"
  # # openapi-generator confused with these two, so we just drop them
  # | reject paths."/w/{workspace}/apps/create_raw"
  # | reject paths."/w/{workspace}/apps/update_raw/{path}"
  # | save -f $"local/($v)/a/b/openapi.yaml";

  openapi-generator-cli generate -i $"local/($v)/a/b/openapi.yaml" -g rust -o ./windmill_api --strict-spec true --additional-properties=packageName="windmill-api"

  patch
}

def 'main publish' [] {
  
}

def 'main test' [] {
  cargo check
  cargo check --features "async"
  cargo test
}


def patch [] {
  let version = http get $"https://raw.githubusercontent.com/windmill-labs/windmill/main/version.txt"

  # Patch Cargo.toml
  open Cargo.toml
  | update package.version $version
  | save -f Cargo.toml

  # Patch windmill_api/Cargo.toml
  open windmill_api/Cargo.toml
  | update dependencies.reqwest.features [json, multipart, rustls-tls]
  | update package.license "Apache-2.0"
  | insert package.homepage "https://windmill.dev"
  | insert package.repository "https://github.com/windmill-labs/windmill-rust-client"
  | save -f windmill_api/Cargo.toml
  
  # Recursively replace serde_json::from_str with our patched version
  ls ./windmill_api/src/**/*.rs
  | each { |file|
      let path = $file.name
      open $path
      | str replace --all "serde_json::from_str" "crate::from_str_patched/* Externally injected from /build.nu */"
      | save -f $path
  }

  # Inject patched from_str
  echo `  
// NOTE: Injected by /dev.nu
pub fn from_str_patched<'a, T>(s: &'a str) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned + 'static,
{
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<uuid::Uuid>() {
        // unsafe { std::mem::transmute::<&str, T>(s) }
        // Quote string
        let a = format!("\"{}\"", s.replace('"', r#"\""#));
        serde_json::from_str(&a)
    } else {
        serde_json::from_str(s)
    }
}
  ` | save --append ./windmill_api/src/lib.rs
}
# TODO:
# Build docs
