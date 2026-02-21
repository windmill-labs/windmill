#!/usr/bin/env bash

set -e

if [ -z "$1" ]; then
    name="wmill-dev"
else
    name="$1"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

./gen_wm_client.sh
./windmill-utils-internal/gen_wm_client.sh

bun install

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

cat > "$INSTALL_DIR/$name" <<EOF
#!/bin/sh
exec bun run "$SCRIPT_DIR/src/main.ts" "\$@"
EOF
chmod +x "$INSTALL_DIR/$name"

echo "Installed dev cli as '$name' at $INSTALL_DIR/$name"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo "Warning: $INSTALL_DIR is not in your PATH. Add it with:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi