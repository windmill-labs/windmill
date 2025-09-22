#! /usr/bin/env nu

let version = open ../version.txt | str trim;

def main [ --publish(-p) --check(-c) --test(-t) ] {
  mkdir api/a/

  open ../backend/windmill-api/openapi.yaml
    # openapi-generator confused with these two, so we just drop them
    | reject paths."/w/{workspace}/apps/create_raw"
    | reject paths."/w/{workspace}/apps/update_raw/{path}"
    | save -f api/openapi.yaml;

  openapi-generator-cli generate -i api/openapi.yaml -g rust -o ./windmill_api --strict-spec true --additional-properties=packageName="windmill-api"

  # Patch Cargo.toml
  open Cargo.proto.toml
  | update package.version $version
  | update dependencies.windmill-api.version $version
  | save -f Cargo.toml

  # Patch windmill_api/Cargo.toml
  open windmill_api/Cargo.toml
  # Use rustls - otherwise compilation will fail due to missing libssl
  | update dependencies.reqwest.features [json, multipart, rustls-tls]
  | insert dependencies.reqwest.default-features false
  | update package.license "Apache-2.0"
  | insert package.homepage "https://windmill.dev"
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
// NOTE: Injected by rust-client/dev.nu
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

  if $check {
    print "Checking..."
    cargo check --no-default-features
    cargo check --features "async"
  }

  if $test {
    print "Testing..."
    cargo test --features async simple
    cargo test --no-default-features
  }

  if $publish {
    print "Publishing..."

    print Publishing windmill-api
    cd windmill_api
    cargo publish --token $env.CRATES_IO_TOKEN --allow-dirty

    print Publishing wmill
    cd ../
    cargo publish --token $env.CRATES_IO_TOKEN  --allow-dirty
  }
}
