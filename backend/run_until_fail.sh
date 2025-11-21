#!/bin/bash

# Run the command repeatedly until it fails (exits with non-zero code)
cd windmill-duckdb-ffi-internal && ./build_dev.sh && cd ..
while true; do
    DISABLE_EMBEDDING=true \
    RUST_LOG=info \
    DENO_PATH=$(which deno) \
    BUN_PATH=$(which bun) \
    GO_PATH=$(which go) \
    UV_PATH=$(which uv) \
    CARGO_PATH=$(which cargo) \
    cargo test --features enterprise,deno_core,license,python,duckdb,rust,scoped_cache \
        -- --nocapture --test-threads=8 | tee /tmp/test.log
    
    # Capture the exit code of the cargo test command (not tee)
    # We need to use PIPESTATUS to get the exit code of cargo test, not tee
    EXIT_CODE=${PIPESTATUS[0]}
    
    # Check if the command failed (non-zero exit code)
    if [ $EXIT_CODE -ne 0 ]; then
        echo "Command failed with exit code: $EXIT_CODE"
        echo "Test output saved to /tmp/test.log"
        exit $EXIT_CODE
    fi
    
    echo "Test passed, running again..."
    echo "----------------------------------------"
done
