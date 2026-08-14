#!/usr/bin/env bash
# Sourced by the PreToolUse guards; not a hook itself.
#
# A permission rule beats a hook: an `ask` rule prompts whatever a PreToolUse hook returns, which
# makes the hook's `allow` dead weight. So settings.json carries no `ask` rule for `rm`, `mv` or
# `chmod`, and the guards own both halves — `allow` what they can prove safe, `ask` for the rest.
# Removing a guard's `ask` path therefore removes that verb's prompt entirely.
#
# `set -f` is global to the sourcing script so that the unquoted word split in runs_verb cannot
# expand a glob operand against the filesystem. Neither guard relies on pathname expansion.
set -f

# 0 iff <verb> ($1) runs as a command word anywhere in <command> ($2). Mirrors how a Bash
# permission rule matches, so that owning the prompt here doesn't narrow what used to prompt:
# the command splits on `; & |` and newlines, and a leading env assignment or process wrapper
# (`timeout 5 rm`, `xargs rm`) is skipped before the command word is read.
#
# The split set also carries the characters that open a nested command — `$(`, backticks, `( )`
# and `{ }` — because a rule matches the verb inside one (`echo $(rm -rf ~)` prompts), and a
# separator that only ends statements would read that as an `echo`.
#
# The scan is textual, so a heredoc body that merely contains the verb (`cat > s.sh <<EOF` …)
# reads as a command and prompts. Left that way on purpose: parsing heredocs to suppress it
# would risk dropping a real trailing command, and an extra prompt is the safe failure.
runs_verb() {
  local verb="$1" seg w wrapped
  while IFS= read -r seg; do
    wrapped=0
    for w in $seg; do
      # The shell strips quotes and backslashes before it looks up the command, so `'rm'` and
      # `r\m` run rm and have to compare equal to it.
      w="${w//[\"\'\\]/}"
      case "$w" in
        "$verb" | */"$verb") return 0 ;;
        *=*) ;;                                        # leading env assignment
        -* | [0-9]*) ;;                                # a wrapper's own flag, or its duration
        *'>'* | *'<'*) ;;                              # leading redirect, `>/dev/null rm ...`
        '!' | if | then | elif | else | while | until | do) ;;   # keywords, never the command
        timeout | time | nice | nohup | stdbuf | command | builtin | noglob | xargs | sudo | env) wrapped=1 ;;
        # A wrapper's option value is indistinguishable from a command name (`stdbuf -o L rm`),
        # so past a wrapper the whole segment is scanned instead of stopping at the first
        # ordinary word. Before one, that word is the command and the verb cannot follow it.
        *) [ "$wrapped" = 1 ] || break ;;
      esac
    done
    # `tr` and not `${2//[...]}`: a `}` inside the bracket expression closes the expansion
    # itself, which silently leaves the command unsplit and every separator unseen.
  done <<< "$(printf '%s' "$2" | tr ';&|(){}`' '\n')"
  return 1
}

# Emit a PreToolUse decision and exit. `ask` is the ordinary permission prompt.
decide() {
  jq -nc --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:$d,permissionDecisionReason:$r}}'
  exit 0
}
