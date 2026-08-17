#!/usr/bin/env bash
# Snapshot a workspace into fixtures/cli-sync/ so it can be committed
# alongside a PR.
#
# Requires: bun on PATH. No `wmill` install needed.
#
# Assumes a Windmill backend running at http://localhost:8000 with the
# default super-admin (admin@windmill.dev / changeme). Override via flags.
set -euo pipefail

BASE_URL="http://localhost:8000"
EMAIL="admin@windmill.dev"
# Prefer WMILL_PASSWORD env var over --password flag — flags leak into
# /proc/<pid>/cmdline and shell history.
PASSWORD="${WMILL_PASSWORD:-changeme}"
DIR=""
WORKSPACE=""

if [[ $# -lt 1 || "$1" == "-h" || "$1" == "--help" ]]; then
  sed -n '2,8p' "$0"
  echo
  echo "Usage: $(basename "$0") <workspace-id> [--base-url URL] [--email E] [--password P] [--dir PATH]"
  exit 0
fi

WORKSPACE="$1"; shift

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) BASE_URL="$2"; shift 2 ;;
    --email) EMAIL="$2"; shift 2 ;;
    --password) PASSWORD="$2"; shift 2 ;;
    --dir) DIR="$2"; shift 2 ;;
    *) echo "Unknown flag: $1" >&2; exit 2 ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DIR="${DIR:-$SCRIPT_DIR/cli-sync}"
DIR="$(cd "$DIR" && pwd)"
CLI_ENTRY="$REPO_ROOT/cli/src/main.ts"

if ! command -v bun >/dev/null 2>&1; then
  echo "✗ bun is required but not found on PATH" >&2
  exit 1
fi
if [[ ! -f "$CLI_ENTRY" ]]; then
  echo "✗ Cannot find CLI entry at $CLI_ENTRY" >&2
  exit 1
fi
if [[ ! -f "$DIR/wmill.yaml" ]]; then
  echo "✗ No wmill.yaml in $DIR — is the fixture folder set up?" >&2
  exit 1
fi

# JSON body builder — interpolation via printf '%s' is unsafe for arbitrary
# emails / passwords. Python is universal enough for a dev script and
# produces correctly escaped JSON.
json_object() {
  python3 -c '
import json, sys
print(json.dumps(dict(zip(sys.argv[1::2], sys.argv[2::2]))))
' "$@"
}

echo "→ Logging in as $EMAIL on $BASE_URL"
TOKEN="$(curl -sS -f -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "$(json_object email "$EMAIL" password "$PASSWORD")")"
if [[ -z "$TOKEN" ]]; then
  echo "✗ Login failed (empty token)" >&2
  exit 1
fi

# Clear previous snapshot content while preserving the fixture scaffold
# (wmill.yaml, .gitkeep). Anything else is removed so the snapshot reflects
# exactly what is in the workspace.
echo "→ Clearing previous snapshot in $DIR"
(
  cd "$DIR"
  find . -mindepth 1 -maxdepth 1 \
    ! -name 'wmill.yaml' \
    ! -name '.gitkeep' \
    -exec rm -rf {} +
)

echo "→ Pulling workspace '$WORKSPACE' into $DIR"
(
  cd "$DIR"
  bun run "$CLI_ENTRY" sync pull --yes \
    --base-url "$BASE_URL" \
    --workspace "$WORKSPACE" \
    --token "$TOKEN"
)

echo
echo "✓ Snapshot of '$WORKSPACE' written to $DIR"
echo "  Commit the changes to share with reviewers. Run fixtures/check-empty.sh"
echo "  to verify the dir is empty again before merging."
