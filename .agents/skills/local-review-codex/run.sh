#!/usr/bin/env bash
# Local Codex review — mirrors the .github/workflows/codex-pr-review.yml CI job,
# but scoped to this branch's unpushed work (committed + uncommitted) so you can
# review before pushing. Same policy (REVIEW.md), same model (gpt-5.6-sol) and
# reasoning effort (xhigh) as CI. Runs read-only: Codex cannot modify your tree.
#
# Usage: run.sh [BASE_REF]   (BASE_REF defaults to "main")
set -euo pipefail

BASE_REF="${1:-main}"
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

if ! command -v codex >/dev/null 2>&1; then
  echo "codex CLI not found. Install with: npm install --global @openai/codex@0.144.1" >&2
  exit 1
fi

# Resolve the base to a concrete commit, preferring a local ref but falling back to
# the remote-tracking ref — checkouts (CI, single-branch clones) often have only
# origin/main, not a local main.
if git rev-parse --verify --quiet "${BASE_REF}^{commit}" >/dev/null; then
  BASE_COMMITISH="$BASE_REF"
elif git rev-parse --verify --quiet "origin/${BASE_REF}^{commit}" >/dev/null; then
  BASE_COMMITISH="origin/${BASE_REF}"
else
  echo "Base ref '$BASE_REF' not found as '$BASE_REF' or 'origin/$BASE_REF'. Try: git fetch origin $BASE_REF" >&2
  exit 1
fi

# Diff from the merge-base so only this branch's changes are reviewed. Using the
# base SHA with a single-ref `git diff` also folds in uncommitted working-tree edits,
# but `git diff` never sees untracked files — those are gathered separately below so
# brand-new files (a whole new module, a new skill dir) are not silently skipped.
BASE_SHA="$(git merge-base HEAD "$BASE_COMMITISH")"
HEAD_SHA="$(git rev-parse HEAD)"
UNTRACKED="$(git ls-files --others --exclude-standard)"

if [ "$BASE_SHA" = "$HEAD_SHA" ] && git diff --quiet "$BASE_SHA" && [ -z "$UNTRACKED" ]; then
  echo "No changes vs $BASE_REF — nothing to review." >&2
  exit 0
fi

PROMPT="$(mktemp)"
OUT="$(mktemp)"
trap 'rm -f "$PROMPT" "$OUT"' EXIT

# REVIEW.md is the shared policy CI feeds Codex. Append the local output-format
# and diff context inline (CI reads these from a generated context file; inlining
# keeps the working tree clean — no scratch files land in the repo).
cat REVIEW.md > "$PROMPT"
cat >> "$PROMPT" <<EOF

# Codex output format

- This is a pre-push LOCAL review of unpushed work; there is no PR yet.
- Inspect the changes by running the diff commands in the review context below.
- Untracked files do NOT appear in \`git diff\`. Review every untracked path listed below by reading it directly (\`cat\`) — treat its entire contents as newly added.
- Return markdown starting with \`## Codex Review\`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.

# Review context

Local review (pre-push): current branch vs $BASE_REF
Base SHA: $BASE_SHA
Head SHA: $HEAD_SHA (plus any uncommitted working-tree changes)

Changed commits command:
git log --oneline $BASE_SHA..HEAD

Changed files command:
git diff --stat $BASE_SHA

Full review diff command (tracked changes, includes uncommitted edits):
git diff --unified=0 $BASE_SHA

Untracked files (NOT in the diff above — read each one directly, it is entirely new):
$(if [ -n "$UNTRACKED" ]; then printf '%s\n' "$UNTRACKED"; else echo "(none)"; fi)
EOF

codex exec \
  -C "$REPO_ROOT" \
  -m gpt-5.6-sol \
  -c 'model_reasoning_effort="xhigh"' \
  -s read-only \
  -o "$OUT" \
  - < "$PROMPT"

echo
echo "===== Codex review ====="
cat "$OUT"
