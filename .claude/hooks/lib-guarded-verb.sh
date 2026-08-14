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
# Past a wrapper the scan is bounded, because the tokens it walks are no longer known to be
# commands: without a bound, one `timeout` in front of a long quoted argument turns every `rm`
# written in that prose into a prompt.
# Heredoc bodies are data, not commands, and a rule doesn't match a verb written inside one —
# a PR body or a generated script would otherwise prompt for every `rm` in its text. A body is
# dropped only when its terminator is actually there, so a `<<` appearing in prose or in an
# arithmetic shift can't swallow the commands that follow it.
strip_heredoc_bodies() {
  local -a lines=()
  local line delim trimmed i j n
  while IFS= read -r line; do lines+=("$line"); done <<< "$1"
  n=${#lines[@]}
  i=0
  while [ "$i" -lt "$n" ]; do
    line="${lines[$i]}"
    printf '%s\n' "$line"
    i=$((i + 1))
    case "$line" in *'<<'*) ;; *) continue ;; esac
    delim="${line#*<<}"
    delim="${delim#-}"                              # <<- strips leading tabs from the body
    delim="${delim#"${delim%%[![:space:]]*}"}"
    delim="${delim%%[[:space:]]*}"                  # the word after <<, ignoring any redirect
    delim="${delim//[\"\'\\]/}"
    [ -n "$delim" ] || continue
    j="$i"
    while [ "$j" -lt "$n" ]; do
      trimmed="${lines[$j]#"${lines[$j]%%[![:space:]]*}"}"
      [ "$trimmed" = "$delim" ] && break
      j=$((j + 1))
    done
    [ "$j" -lt "$n" ] && i=$((j + 1))
  done
}

runs_verb() {
  local verb="$1" seg w wrapped budget
  while IFS= read -r seg; do
    wrapped=0 budget=0
    for w in $seg; do
      if [ "$wrapped" = 1 ]; then
        [ "$budget" -gt 0 ] || break
        budget=$((budget - 1))
      fi
      # The shell strips quotes and backslashes before it looks up the command, so `'rm'` and
      # `r\m` run rm and have to compare equal to it.
      w="${w//[\"\'\\]/}"
      case "$w" in
        "$verb" | */"$verb") return 0 ;;
        *=*) ;;                                        # leading env assignment
        -* | *'>'* | *'<'*) ;;                         # a flag, or a leading redirect
        [0-9]*) [ "$wrapped" = 1 ] || break ;;         # a wrapper's duration, not `1:` in prose
        '!' | if | then | elif | else | while | until | do) ;;   # keywords, never the command
        timeout | time | nice | nohup | stdbuf | command | builtin | noglob | xargs | sudo | env)
          wrapped=1 budget=6 ;;
        # A wrapper's option value is indistinguishable from a command name (`stdbuf -o L rm`),
        # so past a wrapper the next few words are scanned instead of stopping at the first
        # ordinary one. Before one, that word is the command and the verb cannot follow it.
        *) [ "$wrapped" = 1 ] || break ;;
      esac
    done
    # `tr` and not `${2//[...]}`: a `}` inside the bracket expression closes the expansion
    # itself, which silently leaves the command unsplit and every separator unseen.
  done <<< "$(strip_heredoc_bodies "$2" | tr ';&|(){}`' '\n')"
  return 1
}

# Emit a PreToolUse decision and exit. `ask` is the ordinary permission prompt.
decide() {
  jq -nc --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:$d,permissionDecisionReason:$r}}'
  exit 0
}
