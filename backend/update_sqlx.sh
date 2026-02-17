#!/usr/bin/env bash

set -e

# Check if running on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Running on macOS - substituting samael..."
    # Comment out the version-based samael dependency
    sed -i '' 's/^samael = { version="0.0.14", features = \["xmlsec"\] }/#samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Uncomment the git-based samael dependency
    sed -i '' 's/^# \(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/\1/' Cargo.toml

    # Run cargo sqlx prepare with deno_core_mac
    echo "Running cargo sqlx prepare with deno_core_mac..."
    cargo sqlx prepare --workspace -- --workspace --all-targets --features all_sqlx_features,private,deno_core_mac,deno_core,enterprise,mcp
else
    # Run cargo sqlx prepare
    echo "Running cargo sqlx prepare..."
    cargo sqlx prepare --workspace -- --workspace --all-targets --features all_sqlx_features,ee,deno_core,private,enterprise,mcp
fi
