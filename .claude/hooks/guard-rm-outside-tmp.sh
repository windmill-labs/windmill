#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow ONLY a single, plain, single-line `rm` whose every
# operand is a whitelisted /tmp path. Anything else makes no decision (exit 0) and falls
# back to the normal permission flow, where the `Bash(rm:*)` ask rule prompts (with the
# auto-mode classifier as a further backstop).
#
# Deny-by-default: every token must consist only of a safe character set — alphanumerics,
# `. _ / -` and glob chars `* ? [ ]`. That set contains none of the characters bash uses for
# quoting, expansion, or command separation ($ ` ~ { } ( ) ' " \ ; & | < >), so those forms
# fail by construction rather than needing to be enumerated. `realpath -m` then resolves `..`
# and existing symlinks, and a wildcard in a non-final path segment is refused because it can
# expand through a symlink realpath can't see on the unexpanded token.
#
# A PreToolUse `allow` overrides the ask rule, so the safe /tmp case runs without a prompt.
# Assumes GNU `realpath` (-m) and `jq`, both present in this repo's Linux dev env.
set -uo pipefail

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0

# A newline separates commands, and the tokenizer below only reads the first line — defer.
case "$cmd" in *$'\n'*) exit 0 ;; esac

read -r -a toks <<< "$cmd"
# Bare leading `rm` only; wrappers (`timeout rm`), env prefixes, and `/bin/rm` defer.
[ "${toks[0]:-}" = "rm" ] || exit 0

had_operand=0
end_opts=0
i=1
while [ "$i" -lt "${#toks[@]}" ]; do
  t="${toks[$i]}"
  i=$((i + 1))
  # Whitelist every token (flags included, so an operator hidden in a flag like `-rf;rm`
  # can't slip past): any character outside the safe set makes it unsafe to reason about.
  [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/*?[]-')" ] && exit 0
  # Only skip `-`-prefixed tokens as flags BEFORE `--`; after it every token (even `-rf`,
  # which then names the cwd file ./-rf) is an operand and must pass the /tmp checks.
  if [ "$end_opts" = 0 ]; then
    [ "$t" = "--" ] && { end_opts=1; continue; }
    case "$t" in -*) continue ;; esac
  fi
  had_operand=1
  case "$t" in /tmp/*) ;; *) exit 0 ;; esac      # must be an absolute /tmp path
  case "${t%/*}" in *[*?[]*) exit 0 ;; esac       # no wildcard in a non-final segment
  canon=$(realpath -m -- "$t" 2>/dev/null) || exit 0
  case "$canon" in /tmp/?*) ;; *) exit 0 ;; esac  # and it must still resolve under /tmp
done

[ "$had_operand" = 1 ] || exit 0
jq -nc '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",permissionDecisionReason:"all rm operands are whitelisted /tmp paths"}}'
