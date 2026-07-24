#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow ONLY a single, plain, single-line `rm` whose every
# operand canonicalizes under /tmp. Anything else makes no decision (exit 0) and falls back
# to the normal permission flow — the `Bash(rm:*)` ask rule prompts, with the auto-mode
# classifier as a further backstop.
#
# Deliberately allow-only: a PreToolUse `allow` overrides the ask rule, so the safe /tmp
# case runs without a prompt, while the hook never has to enumerate the unbounded set of
# unsafe forms (wrappers like `timeout rm` / `command rm` / env prefixes, brace/glob/command
# expansion, multi-line). It recognizes just the narrow safe case and lets the rest prompt.
#
# Assumes GNU `realpath` (-m) and `jq`, both present in this repo's Linux dev env.
set -uo pipefail

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0

# Defer on anything bash would transform before `rm` sees the operand, so the raw token we
# canonicalize is exactly what gets deleted: newlines/operators/redirection (new commands),
# expansion metacharacters ($ ` ~ { } ( )), and quotes/backslash (quote removal can hide a
# `..` from realpath, e.g. rm /tmp/"../etc/x" or rm /tmp/\../etc/x).
case "$cmd" in
  *$'\n'* | *\'* | *\"* | *\\* | *\`* | *\$* | *\~* \
  | *\;* | *\&* | *\|* | *\<* | *\>* | *\(* | *\)* | *\{* | *\}* )
    exit 0 ;;
esac

# Must be a bare, leading `rm` — no sudo/env/timeout/command wrappers, no `/bin/rm` path.
read -r -a toks <<< "$cmd"
[ "${toks[0]:-}" = "rm" ] || exit 0

had_operand=0
i=1
while [ "$i" -lt "${#toks[@]}" ]; do
  t="${toks[$i]}"
  i=$((i + 1))
  [ "$t" = "--" ] && continue
  case "$t" in -*) continue ;; esac
  had_operand=1
  # A wildcard in a non-final segment (`/tmp/*/x`) can expand through a symlink that
  # realpath can't see on the unexpanded token — defer those.
  case "${t%/*}" in *[*?[]*) exit 0 ;; esac
  canon=$(realpath -m -- "$t" 2>/dev/null) || exit 0
  case "$canon" in
    /tmp/?*) ;;
    *) exit 0 ;;
  esac
done

[ "$had_operand" = 1 ] || exit 0

jq -nc '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",permissionDecisionReason:"all rm operands are under /tmp"}}'
