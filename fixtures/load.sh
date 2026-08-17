#!/usr/bin/env bash
# Load fixtures/cli-sync/ into a fresh workspace on a local Windmill instance.
#
# Requires: bun on PATH. No `wmill` install needed — we invoke
# cli/src/main.ts directly via `bun run`.
#
# Assumes a Windmill backend running at http://localhost:8000 with the
# default super-admin (admin@windmill.dev / changeme). Override via flags.
set -euo pipefail

BASE_URL="http://localhost:8000"
EMAIL="admin@windmill.dev"
# Prefer WMILL_PASSWORD env var over --password flag — flags leak into
# /proc/<pid>/cmdline and shell history.
PASSWORD="${WMILL_PASSWORD:-changeme}"
WORKSPACE=""
DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) BASE_URL="$2"; shift 2 ;;
    --email) EMAIL="$2"; shift 2 ;;
    --password) PASSWORD="$2"; shift 2 ;;
    --workspace) WORKSPACE="$2"; shift 2 ;;
    --dir) DIR="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,8p' "$0"; exit 0 ;;
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

if [[ -z "$WORKSPACE" ]]; then
  # Use $RANDOM rather than piping /dev/urandom through head -c, which
  # SIGPIPEs `tr` and aborts the script under `set -o pipefail`.
  printf -v WORKSPACE 'fixture-%04x%04x' $RANDOM $RANDOM
fi

# JSON body builder — interpolation via printf '%s' is unsafe for arbitrary
# emails / passwords / workspace ids. Python is universal enough for a dev
# script and produces correctly escaped JSON.
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

echo "→ Creating workspace '$WORKSPACE'"
CREATE_OUT="$(mktemp)"
trap 'rm -f "$CREATE_OUT"' EXIT
HTTP_CODE="$(curl -sS -o "$CREATE_OUT" -w '%{http_code}' \
  -X POST "$BASE_URL/api/workspaces/create" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(json_object id "$WORKSPACE" name "$WORKSPACE")")"
if [[ "$HTTP_CODE" != "200" && "$HTTP_CODE" != "201" ]]; then
  echo "✗ Workspace creation failed (HTTP $HTTP_CODE):" >&2
  cat "$CREATE_OUT" >&2
  echo >&2
  exit 1
fi

echo "→ Pushing $DIR to workspace '$WORKSPACE'"
(
  cd "$DIR"
  bun run "$CLI_ENTRY" sync push --yes \
    --base-url "$BASE_URL" \
    --workspace "$WORKSPACE" \
    --token "$TOKEN"
)

echo
echo "✓ Fixture loaded into workspace '$WORKSPACE'"
echo "  Open: ${BASE_URL%/}/?workspace=$WORKSPACE"
