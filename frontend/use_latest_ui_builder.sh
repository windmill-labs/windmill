#!/bin/bash
set -euo pipefail

cd ~/windmill-code-ui-builder
HASH=$(git rev-parse --short HEAD)
HASH=${HASH::-1}
ARTIFACT_URL="https://pub-06154ed168a24e73a86ab84db6bf15d8.r2.dev/ui_builder-${HASH}.tar.gz"

TMP_FILE=$(mktemp)
trap 'rm -f "$TMP_FILE"' EXIT

echo "Using UI Builder hash: ${HASH}"
curl -fsSL "$ARTIFACT_URL" -o "$TMP_FILE"

if command -v sha256sum >/dev/null 2>&1; then
  SHA256=$(sha256sum "$TMP_FILE" | awk '{print $1}')
else
  SHA256=$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')
fi
echo "Using UI Builder sha256: ${SHA256}"

node -e '
const fs = require("fs")
const [version, sha256] = process.argv.slice(1)
const artifactPath = "../windmill/frontend/scripts/ui_builder_artifact.json"
const artifact = JSON.parse(fs.readFileSync(artifactPath, "utf8"))
artifact.version = version
artifact.sha256 = sha256
fs.writeFileSync(artifactPath, JSON.stringify(artifact, null, "\t") + "\n")
' "$HASH" "$SHA256"
