#!/usr/bin/env bash

set -e

# Default directory
EE_DIR="../windmill-ee-private"

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --dir) EE_DIR="$2"; shift ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

./substitute_ee_code.sh --dir "$EE_DIR"

# Check if running on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Running on macOS - substituting samael..."
    # Comment out the version-based samael dependency
    sed -i '' 's/^samael = { version="0.0.14", features = \["xmlsec"\] }/#samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Uncomment the git-based samael dependency
    sed -i '' 's/^# \(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/\1/' Cargo.toml

    # Run cargo sqlx prepare with deno_core_mac 
    echo "Running cargo sqlx prepare with deno_core_mac..."
    cargo sqlx prepare --workspace -- --all-targets --features all_sqlx_features,private,deno_core_mac
else
    # Run cargo sqlx prepare
    echo "Running cargo sqlx prepare..."
    cargo sqlx prepare --workspace -- --all-targets --features all_sqlx_features,private
fi



# Undo the samael changes on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Reverting samael changes..."
    # Uncomment the version-based samael dependency
    sed -i '' 's/^#samael = { version="0.0.14", features = \["xmlsec"\] }/samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Comment out the git-based samael dependency
    sed -i '' 's/^\(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/# \1/' Cargo.toml
fi
