./substitute_ee_code.sh --dir ../windmill-ee-private

# Check if running on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Running on macOS - substituting samael..."
    # Comment out the version-based samael dependency
    sed -i '' 's/^samael = { version="0.0.14", features = \["xmlsec"\] }/#samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Uncomment the git-based samael dependency
    sed -i '' 's/^# \(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/\1/' Cargo.toml
fi

cargo sqlx prepare --workspace -- --all-targets --all-features
./substitute_ee_code.sh -r --dir ../windmill-ee-private

# Undo the samael changes on macOS
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Reverting samael changes..."
    # Uncomment the version-based samael dependency
    sed -i '' 's/^#samael = { version="0.0.14", features = \["xmlsec"\] }/samael = { version="0.0.14", features = ["xmlsec"] }/' Cargo.toml
    # Comment out the git-based samael dependency
    sed -i '' 's/^\(samael = { git="https:\/\/github.com\/njaremko\/samael", rev="464d015e3ae393e4b5dd00b4d6baa1b617de0dd6", features = \["xmlsec"\] }\)/# \1/' Cargo.toml
fi
