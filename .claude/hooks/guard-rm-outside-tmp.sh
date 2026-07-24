#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow only when the command is a single plain `rm`
# whose every operand canonicalizes under /tmp; force a prompt for anything else.
#
# Why a hook and not glob allow-rules: Bash `*` in a permission pattern spans spaces and
# `..`, so `Bash(rm /tmp/*)` would also auto-approve `rm /tmp/a backend/x`. And because we
# inspect the *unexpanded* command string, we must refuse any shell syntax that could
# retarget the deletion after this check runs — newlines (command separators, which a
# single-line tokenizer misses) and expansion metacharacters `$ ` ~ { } ( )` (e.g. brace
# expansion `/tmp/{a,../etc/b}`), plus control/redirect operators `; & | < >`. Only literal
# paths and in-directory globs (`* ? [ ]`) are ever auto-allowed.
#
# Assumes GNU `realpath` (-m) and `jq`, both present in this repo's Linux dev env. If jq is
# absent the hook makes no decision and rm falls back to the normal permission flow.
set -uo pipefail

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0

emit() {
  jq -nc --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:$d,permissionDecisionReason:$r}}'
  exit 0
}

# Only weigh in when an `rm` invocation is present (line start or after a shell operator).
printf '%s' "$cmd" | grep -Eq '(^|[;&|(]|&&|\|\|)[[:space:]]*rm([[:space:]]|$)' || exit 0

# Multi-line: a newline separates commands like `;`, so refuse rather than validate one line.
case "$cmd" in *$'\n'*) emit ask "multi-line rm command; confirm manually" ;; esac

# Any operator, redirection, or expansion metacharacter: refuse — we can't prove the
# post-expansion targets stay under /tmp.
printf '%s' "$cmd" | grep -Eq '[;&|<>`(){}$~]|\bxargs\b|\bfind\b' \
  && emit ask "shell operators or expansion in rm command; confirm manually"

# rm must be the sole leading command (no sudo/env/cd prefixes).
read -r -a toks <<< "$cmd"
[ "${toks[0]:-}" = "rm" ] || emit ask "rm is not the leading command; confirm manually"

had_operand=0
i=1
while [ "$i" -lt "${#toks[@]}" ]; do
  t="${toks[$i]}"
  i=$((i + 1))
  [ "$t" = "--" ] && continue
  case "$t" in -*) continue ;; esac
  had_operand=1
  canon=$(realpath -m -- "$t" 2>/dev/null) || emit ask "cannot resolve operand '$t'; confirm manually"
  case "$canon" in
    /tmp/?*) ;;
    *) emit ask "operand '$t' is not under /tmp; confirm manually" ;;
  esac
done

[ "$had_operand" = 1 ] || emit ask "rm without a path operand; confirm manually"
emit allow "all rm operands are under /tmp"
