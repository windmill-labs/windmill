#!/usr/bin/env bash
# Trigger a CI review round on a PR and wait for it to finish.
#
# Usage: bash .agents/skills/pr/review-round.sh [PR_NUMBER]
#   PR_NUMBER defaults to the current branch's PR.
#
# Comments `/review` on the PR (works on drafts), waits for the spawned
# "PR Review Commands" workflow run(s) to complete, then prints one verdict
# line per reviewer and saves each full review comment to a file. A round
# takes 10-30 minutes: run this in the background and act on its output when
# it exits, per the pr skill ("Review rounds").
set -euo pipefail

REPO=${REPO:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}
PR=${1:-$(gh pr view --json number --jq .number)}

# Timestamp of the trigger comment, straight from GitHub, so local clock skew
# can't make the run/comment filters below miss part of the round.
TRIGGER_TIME=$(gh api "repos/$REPO/issues/$PR/comments" -f body='/review' --jq .created_at)
echo "Review round triggered on $REPO#$PR at $TRIGGER_TIME"

# The /review comment spawns one "PR Review Commands" run holding the
# claude/codex/pi jobs. Runs aren't linked to a PR, so wait on every run of
# that workflow created after the trigger: a concurrent round on another PR
# can only delay the answer, never truncate it.
DEADLINE=$(( $(date +%s) + 45 * 60 ))
NO_RUN_DEADLINE=$(( $(date +%s) + 5 * 60 ))
while :; do
  RUNS=$(gh run list --repo "$REPO" --workflow=pr-review-commands.yml \
    --created ">=$TRIGGER_TIME" --json status)
  TOTAL=$(jq length <<<"$RUNS")
  PENDING=$(jq '[.[] | select(.status != "completed")] | length' <<<"$RUNS")
  if [ "$TOTAL" -gt 0 ] && [ "$PENDING" -eq 0 ]; then
    break
  fi
  NOW=$(date +%s)
  if [ "$TOTAL" -eq 0 ] && [ "$NOW" -gt "$NO_RUN_DEADLINE" ]; then
    echo "ERROR: no 'PR Review Commands' run appeared within 5 minutes of the /review comment; check that the comment author has write access and the workflow is enabled." >&2
    exit 1
  fi
  if [ "$NOW" -gt "$DEADLINE" ]; then
    echo "WARNING: review round still pending after 45 minutes; reporting whatever has been posted so far." >&2
    break
  fi
  sleep 60
done

OUT_DIR=$(mktemp -d -t review-round-XXXXXX)
gh api "repos/$REPO/issues/$PR/comments?per_page=100" --paginate \
  | jq -s --arg t "$TRIGGER_TIME" '[.[][] | select(.created_at > $t)]' \
  > "$OUT_DIR/comments.json"
# cubic posts through the PR reviews API, not issue comments.
gh api "repos/$REPO/pulls/$PR/reviews?per_page=100" --paginate \
  | jq -s --arg t "$TRIGGER_TIME" '[.[][] | select((.submitted_at // "") > $t)]' \
  > "$OUT_DIR/pr-reviews.json"

VERDICT_RE='(Good to merge|Mergeable, but should ideally address nits|Should address issues before merging)'

body_by_header() {
  jq -r --arg h "$1" '[.[] | select(.body // "" | contains($h))] | last | .body // empty' \
    "$OUT_DIR/comments.json"
}
body_by_login() {
  jq -r --arg l "$1" '[.[] | select(.user.login == $l)] | last | .body // empty' \
    "$OUT_DIR/comments.json"
}
report() { # <reviewer-name> <comment-body>
  local name=$1 body=$2 verdict
  if [ -z "$body" ]; then
    echo "$name: (no review posted this round)"
    return
  fi
  printf '%s\n' "$body" > "$OUT_DIR/$name.md"
  verdict=$(printf '%s\n' "$body" | grep -m1 -oE "${VERDICT_RE}.*" | sed 's/\*\*//g' || true)
  echo "$name: ${verdict:-(review posted but no verdict line; read $OUT_DIR/$name.md)}"
}

echo
echo "=== Review round verdicts for $REPO#$PR (posted after $TRIGGER_TIME) ==="
CODEX_BODY=$(body_by_header '## Codex Review')
report codex "$CODEX_BODY"
report claude "$(body_by_login 'claude[bot]')"
report pi "$(body_by_header '## Pi Review')"
CUBIC_BODY=$(jq -r '[.[] | select(.user.login | test("cubic"; "i"))] | last | .body // empty' \
  "$OUT_DIR/pr-reviews.json")
if [ -z "$CUBIC_BODY" ]; then
  CUBIC_BODY=$(jq -r '[.[] | select(.user.login | test("cubic"; "i"))] | last | .body // empty' \
    "$OUT_DIR/comments.json")
fi
report cubic "$CUBIC_BODY"
echo
echo "Full round output: $OUT_DIR (comments.json, pr-reviews.json, one .md per reviewer)"
if [ -z "$CODEX_BODY" ]; then
  echo "WARNING: Codex verdict missing - the round is incomplete. Re-trigger with a '/codex' PR comment and wait again." >&2
fi
