#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow only when every path operand canonicalizes
# under /tmp, and force a prompt for any other rm. Glob allow-rules can't express this
# because Bash `*` spans spaces and `..`, so `Bash(rm /tmp/*)` would also auto-approve
# `rm /tmp/a backend/x` and `rm /tmp/../etc/x`. Canonicalization has to happen here.
# Non-rm commands are left untouched (exit 0, no decision).
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

# Anything with control operators, redirection, or substitution can't be reasoned about
# operand-by-operand — prompt rather than risk auto-allowing a hidden deletion.
printf '%s' "$cmd" | grep -Eq '[;&|<>`]|\$\(|\$\{|\bxargs\b|\bfind\b' \
  && emit ask "compound or substituted rm command; confirm manually"

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
