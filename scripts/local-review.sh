#!/usr/bin/env bash
# Universal local-review entry point. Runs the shared review policy
# (.github/review-prompt-shared.md) against the current branch/PR using
# the requested CLI agent.
#
# Usage:
#   scripts/local-review.sh [claude|codex|pi]
#
# The agent will follow the local-review skill at
# .agents/skills/local-review/SKILL.md, which reads the shared policy
# and figures out the PR/diff itself via gh / git.
set -euo pipefail

TOOL="${1:-}"

usage() {
  cat <<EOF >&2
Usage: $0 [claude|codex|pi]

  claude  Use the Claude Code /local-review skill (interactive only —
          this script just prints the invocation hint).
  codex   Run the review via 'codex exec' (gpt-5.5, danger-full-access).
  pi      Run the review via 'pi -p' (deepseek-v4-pro).

The same policy and skill file are used for all three —
.agents/skills/local-review/SKILL.md and .github/review-prompt-shared.md.
EOF
  exit 1
}

[ -z "$TOOL" ] && usage

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
SKILL="$REPO_ROOT/.agents/skills/local-review/SKILL.md"
SHARED="$REPO_ROOT/.github/review-prompt-shared.md"

case "$TOOL" in
  claude)
    cat <<EOF
For Claude Code, run /local-review in your active session.

The skill is at .claude/skills/local-review/SKILL.md (symlinked to
.agents/skills/local-review/SKILL.md). Both Claude and Pi auto-discover
it; this script can't drive an interactive Claude Code session
externally.
EOF
    ;;
  codex)
    cat "$SKILL" "$SHARED" | codex exec \
      -C "$REPO_ROOT" \
      -m gpt-5.5 \
      -c 'model_reasoning_effort="xhigh"' \
      -s danger-full-access \
      -
    ;;
  pi)
    cat "$SKILL" "$SHARED" | pi -p \
      --provider deepseek \
      --model deepseek-v4-pro \
      --tools read,grep,find,ls,bash
    ;;
  *)
    echo "Unknown tool: $TOOL" >&2
    usage
    ;;
esac
