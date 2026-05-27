#!/usr/bin/env bash
# Fail if fixtures/cli-sync/ contains anything beyond the fixture scaffold
# (wmill.yaml, .gitkeep). Used by CI to guard `main` against accidentally
# merging PRs with a test workspace snapshot still committed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DIR="${1:-$SCRIPT_DIR/cli-sync}"

ALLOWED_RE='^fixtures/cli-sync/(\.gitkeep|wmill\.yaml)$'

# Use `git ls-files` so we only check tracked files. Untracked local snapshots
# are fine — devs may keep them locally between sessions.
EXTRA=$(cd "$REPO_ROOT" && git ls-files fixtures/cli-sync \
  | grep -vE "$ALLOWED_RE" || true)

if [[ -n "$EXTRA" ]]; then
  echo "✗ fixtures/cli-sync/ contains committed snapshot files:" >&2
  echo "$EXTRA" | sed 's/^/    /' >&2
  echo >&2
  echo "  Run fixtures/snapshot.sh against an empty workspace or" >&2
  echo "  remove the files before merging." >&2
  exit 1
fi

echo "✓ fixtures/cli-sync/ is clean"
