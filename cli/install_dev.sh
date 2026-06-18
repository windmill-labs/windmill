#!/usr/bin/env bash

set -e

# Parse options
USE_NODE=false
name=""
for arg in "$@"; do
    case "$arg" in
        --node|-node|---node) USE_NODE=true ;;
        -*) echo "Unknown option: $arg"; echo "Usage: $0 [name] [--node]"; exit 1 ;;
        *) [ -z "$name" ] && name="$arg" ;;
    esac
done

if [ -z "$name" ]; then
    name="wmill-dev"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

./gen_wm_client.sh
./windmill-utils-internal/gen_wm_client.sh

bun install

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

if [ "$USE_NODE" = true ]; then
    echo "Building npm bundle..."
    bun run build-npm.ts

    NPM_DIR="$SCRIPT_DIR/npm"
    cd "$NPM_DIR" && npm install
    cd "$SCRIPT_DIR"

    cat > "$INSTALL_DIR/$name" <<EOF
#!/bin/sh
exec node "$NPM_DIR/esm/main.js" "\$@"
EOF
else
    cat > "$INSTALL_DIR/$name" <<EOF
#!/bin/sh
exec bun run "$SCRIPT_DIR/src/main.ts" "\$@"
EOF
fi

chmod +x "$INSTALL_DIR/$name"

echo "Installed dev cli as '$name' at $INSTALL_DIR/$name"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo "Warning: $INSTALL_DIR is not in your PATH. Add it with:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi
