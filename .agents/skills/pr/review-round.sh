#!/usr/bin/env bash
# Trigger a CI review round on a PR and wait for it to finish.
#
# Usage: bash .agents/skills/pr/review-round.sh [PR_NUMBER]
#   PR_NUMBER defaults to the current branch's PR.
#
# Comments `/review` on the PR (works on drafts), waits for the spawned
# "PR Review Commands" workflow run(s) to complete, then prints one verdict
# line per reviewer and saves each full review comment to a file. A round
# takes about 10 minutes, 20 at most: run this in the background and act on its output when
# it exits, per the pr skill ("Review rounds").
set -euo pipefail

REPO=${REPO:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}
PR=${1:-$(gh pr view --json number --jq .number)}
PR_TITLE=$(gh pr view "$PR" --repo "$REPO" --json title --jq .title)

# Timestamp of the trigger comment, straight from GitHub, so local clock skew
# can't make the run/comment filters below miss part of the round.
TRIGGER_TIME=$(gh api "repos/$REPO/issues/$PR/comments" -f body='/review' --jq .created_at)
echo "Review round triggered on $REPO#$PR at $TRIGGER_TIME"

# Retry wrapper for one-off gh/API hiccups: a 20-minute wait must not die on
# a single transient failure.
retry() {
  local attempt
  for attempt in 1 2 3; do
    if "$@"; then return 0; fi
    sleep 10
  done
  return 1
}

# The /review comment spawns one "PR Review Commands" run holding the
# claude/codex/pi jobs. Runs aren't linked to a PR, but an issue_comment run's
# display title is the PR's title: match on it, since waiting on every run
# after the trigger means a stuck reviewer on any other PR holds this round
# to the deadline. Every comment on this PR (the reviewers' own
# included) spawns a fast-completing parse run, so the round's own run may
# briefly lag the listing while those already show completed: require the
# all-completed state to hold past a floor and across two consecutive polls.
#
# Reviews land 6-10 minutes in. A reviewer that never starts or hangs is common
# and only a human re-trigger fixes it, so stop early: as soon as all three
# verdicts are posted, else at 20 minutes, naming the jobs still unfinished.
VERDICT_RE='(Good to merge|Mergeable, but should ideally address nits|Should address issues before merging)'
# Claude edits a placeholder comment in place, hence updated_at.
all_verdicts_in() {
  gh api "repos/$REPO/issues/$PR/comments?per_page=100" --paginate 2>/dev/null \
    | jq -s -e --arg t "$TRIGGER_TIME" --arg re "$VERDICT_RE" '
        [.[][] | select(.updated_at > $t) | select(.body // "" | test($re))] as $c
        | ($c | any(.user.login == "github-actions[bot]" and (.body | startswith("## Codex Review"))))
          and ($c | any(.user.login == "github-actions[bot]" and (.body | startswith("## Pi Review"))))
          and ($c | any(.user.login == "claude[bot]"))' >/dev/null
}
DEADLINE=$(( $(date +%s) + 20 * 60 ))
NO_RUN_DEADLINE=$(( $(date +%s) + 5 * 60 ))
MIN_WAIT_UNTIL=$(( $(date +%s) + 3 * 60 ))
STABLE=0
SEEN=0
ALL_IN=0
ROUND_RUNS='[]'
FAILURES=0
while :; do
  if RUNS=$(gh run list --repo "$REPO" --workflow=pr-review-commands.yml \
      --created ">=$TRIGGER_TIME" --limit 500 --json databaseId,status,displayTitle \
      --jq "[.[] | select(.displayTitle == $(jq -Rn --arg t "$PR_TITLE" '$t'))]"); then
    FAILURES=0
  else
    FAILURES=$(( FAILURES + 1 ))
    if [ "$FAILURES" -ge 5 ]; then
      echo "ERROR: listing workflow runs failed $FAILURES times in a row; aborting the wait." >&2
      exit 1
    fi
    echo "WARNING: listing workflow runs failed (attempt $FAILURES/5); retrying in 60s." >&2
    sleep 60
    continue
  fi
  TOTAL=$(jq length <<<"$RUNS")
  # The limit applies before the title filter, so on a busy repo this PR's run
  # can age out of the listing: once seen, never report it as missing.
  if [ "$TOTAL" -gt 0 ]; then SEEN=1; ROUND_RUNS=$RUNS; fi
  if all_verdicts_in; then
    ALL_IN=1
    break
  fi
  PENDING=$(jq '[.[] | select(.status != "completed")] | length' <<<"$RUNS")
  NOW=$(date +%s)
  if [ "$TOTAL" -gt 0 ] && [ "$PENDING" -eq 0 ] && [ "$NOW" -gt "$MIN_WAIT_UNTIL" ]; then
    STABLE=$(( STABLE + 1 ))
    if [ "$STABLE" -ge 2 ]; then
      break
    fi
  else
    STABLE=0
  fi
  if [ "$SEEN" -eq 0 ] && [ "$NOW" -gt "$NO_RUN_DEADLINE" ]; then
    echo "ERROR: no 'PR Review Commands' run appeared within 5 minutes of the /review comment; check that the comment author has write access and the workflow is enabled." >&2
    exit 1
  fi
  if [ "$NOW" -gt "$DEADLINE" ]; then
    echo "WARNING: review round still pending after 20 minutes; reporting whatever has been posted so far." >&2
    break
  fi
  sleep 60
done

# Head SHA at trigger time. `/review` is idempotent per head: it skips an agent a
# running/successful review already covers, re-runs a cancelled/failed one in place on a
# separate head-tied run, and launches fresh only when nothing covers the head. Verdict
# reading below therefore keys off the head, not just the trigger timestamp.
HEAD_SHA=$(retry gh api "repos/$REPO/pulls/$PR" --jq .head.sha)
echo "Reviewing head $HEAD_SHA"

# Newest non-skipped run of <workflow> tied to the head ("status conclusion"), or empty
# when none exists. A re-run-in-place or an already-covering review resolves on such a
# head-tied run — separate from the pr-review-commands run waited on above (a fresh
# launch instead runs inside it, and posts after the trigger). A `skipped` run is the
# draft/fork gate and produced no review, so it is ignored.
head_run_state() {
  gh run list --repo "$REPO" --workflow "$1" --commit "$HEAD_SHA" --limit 20 \
    --json databaseId,status,conclusion \
    --jq '[.[] | select(.conclusion != "skipped")] | sort_by(.databaseId) | last | if . then "\(.status) \(.conclusion // "-")" else empty end' 2>/dev/null || true
}

# A re-run-in-place review lands on a head-tied run that finishes after the fast
# pr-review-commands run, so let those settle before reading verdicts.
for wf in codex-pr-review.yml pi-pr-review.yml pr-ready-review.yml; do
  while [ "$ALL_IN" -eq 0 ]; do
    case "$(head_run_state "$wf")" in
      ""|"completed "*) break ;;
      *) if [ "$(date +%s)" -gt "$DEADLINE" ]; then break; fi; sleep 30 ;;
    esac
  done
done

OUT_DIR=$(mktemp -d -t review-round-XXXXXX)
COMMENTS_RAW=$(retry gh api "repos/$REPO/issues/$PR/comments?per_page=100" --paginate)
# Two views: comments from THIS round (after the trigger) and the full history. A fresh
# launch posts after the trigger; an idempotent skip leaves the covering verdict in the
# earlier run's comment, so fall back to history when that agent's head run is green.
jq -s --arg t "$TRIGGER_TIME" '[.[][] | select(.created_at > $t)]' \
  <<<"$COMMENTS_RAW" > "$OUT_DIR/comments.json"
jq -s '[.[][]]' <<<"$COMMENTS_RAW" > "$OUT_DIR/comments-all.json"
# cubic posts through the PR reviews API, not issue comments.
REVIEWS_RAW=$(retry gh api "repos/$REPO/pulls/$PR/reviews?per_page=100" --paginate)
jq -s --arg t "$TRIGGER_TIME" '[.[][] | select((.submitted_at // "") > $t)]' \
  <<<"$REVIEWS_RAW" > "$OUT_DIR/pr-reviews.json"

body_by_header() { # <file> <header-substring>
  jq -r --arg h "$2" '[.[] | select(.body // "" | contains($h))] | last | .body // empty' "$1"
}
body_by_login() { # <file> <login>
  jq -r --arg l "$2" '[.[] | select(.user.login == $l)] | last | .body // empty' "$1"
}
head_ok() { [ "$(head_run_state "$1")" = "completed success" ]; }
# Latest verdict for a reviewer: prefer this round's comment; if none and the reviewer's
# head run succeeded (an idempotent /review skipped re-reviewing an already-green head),
# fall back to the covering comment from the full history.
verdict_body() { # <header|login> <value> <workflow>
  local body
  body=$("body_by_$1" "$OUT_DIR/comments.json" "$2")
  if [ -z "$body" ] && head_ok "$3"; then
    body=$("body_by_$1" "$OUT_DIR/comments-all.json" "$2")
  fi
  printf '%s' "$body"
}
report() { # <reviewer-name> <comment-body>
  local name=$1 body=$2 verdict
  if [ -z "$body" ]; then
    if [ "$name" = cubic ]; then
      echo "$name: (no review posted for this head)"
    else
      echo "$name: (no review posted for this head; re-trigger now with a '/$name' PR comment)"
    fi
    return
  fi
  printf '%s\n' "$body" > "$OUT_DIR/$name.md"
  verdict=$(printf '%s\n' "$body" | grep -m1 -oE "${VERDICT_RE}.*" | sed 's/\*\*//g' || true)
  echo "$name: ${verdict:-(review posted but no verdict line; read $OUT_DIR/$name.md)}"
}

echo
echo "=== Review round verdicts for $REPO#$PR (head $HEAD_SHA) ==="
CODEX_BODY=$(verdict_body header '## Codex Review' codex-pr-review.yml)
report codex "$CODEX_BODY"
report claude "$(verdict_body login 'claude[bot]' pr-ready-review.yml)"
report pi "$(verdict_body header '## Pi Review' pi-pr-review.yml)"
CUBIC_BODY=$(jq -r '[.[] | select(.user.login | test("^cubic(-dev-ai)?(\\[bot\\])?$"; "i"))] | last | .body // empty' \
  "$OUT_DIR/pr-reviews.json")
if [ -z "$CUBIC_BODY" ]; then
  CUBIC_BODY=$(jq -r '[.[] | select(.user.login | test("^cubic(-dev-ai)?(\\[bot\\])?$"; "i"))] | last | .body // empty' \
    "$OUT_DIR/comments.json")
fi
report cubic "$CUBIC_BODY"
echo
echo "Full round output: $OUT_DIR (comments.json, pr-reviews.json, one .md per reviewer)"
if [ "$ALL_IN" -eq 0 ]; then
  UNFINISHED=$(for id in $(jq -r '.[].databaseId' <<<"$ROUND_RUNS"); do
    gh run view "$id" --repo "$REPO" --json jobs --jq '.jobs[]
      | select(.status != "completed" or (.conclusion | IN("success", "skipped") | not))
      | "  \(.name): \(.status) \(.conclusion // "") since \(.startedAt // "-")"' 2>/dev/null || true
  done)
  if [ -n "$UNFINISHED" ]; then
    echo "Review jobs that did not finish cleanly:"
    printf '%s\n' "$UNFINISHED"
  fi
fi
if [ -z "$CODEX_BODY" ]; then
  echo "WARNING: no Codex verdict for $HEAD_SHA - its head run is not green (cancelled/failed/absent, not merely skipped-because-already-reviewed). Re-trigger with a '/codex' PR comment (re-runs the interrupted run in place, or launches one) and wait again." >&2
fi
