# This script outputs all features except private. Usage :
#  > cargo build --features $(./all_features_oss.sh)

#!/bin/bash

# Path to the Cargo.toml file
CARGO_TOML_PATH="./Cargo.toml"

# Extract features from Cargo.toml and output them separated by commas
if [[ -f "$CARGO_TOML_PATH" ]]; then
    grep -A 100 '\[features\]' "$CARGO_TOML_PATH" | \
    sed -n '/\[features\]/,/^\[/p' | \
    grep -E '^[a-zA-Z0-9_-]+' | \
    grep -v 'private' | \
    cut -d' ' -f1 | \
    paste -sd ',' -
else
    echo "Cargo.toml not found at $CARGO_TOML_PATH"
    exit 1
fi