#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow ONLY a single, plain, single-line `rm` whose every
# operand is a whitelisted target — under /tmp, or strictly inside a git working tree located
# in $HOME (git-recoverable). Anything else makes no decision (exit 0) and falls back to the
# normal permission flow, where the `Bash(rm:*)` ask rule prompts (classifier as a backstop).
#
# Deny-by-default: every token must consist only of a safe character set (alphanumerics,
# `. _ / -` and glob chars `* ? [ ]`). That set contains none of the characters bash uses for
# quoting, expansion, or command separation ($ ` ~ { } ( ) ' " \ ; & | < >), so those forms
# fail by construction rather than needing to be enumerated. `realpath -m` then resolves `..`
# and existing symlinks (so a symlink out of the allowed roots is caught), and a wildcard in a
# non-final path segment is refused because it can expand through a symlink realpath can't see.
#
# The git-repo allowance covers targets inside a git working tree under $HOME, and the tree's
# own root folder only when it is a linked worktree (`.git` is a pointer file, so history in
# the main repo survives); a primary checkout's root (`.git` is a history dir) and any `.git`
# path are never auto-allowed. Relative operands resolve against the command's cwd (from the
# hook input). A PreToolUse `allow` overrides the ask rule.
#
# Assumes GNU `realpath` (-m) and `jq`, both present in this repo's Linux dev env.
set -uo pipefail

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0
cwd=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null)

# A newline separates commands, and the tokenizer below only reads the first line — defer.
case "$cmd" in *$'\n'*) exit 0 ;; esac

read -r -a toks <<< "$cmd"
# Bare leading `rm` only; wrappers (`timeout rm`), env prefixes, and `/bin/rm` defer.
[ "${toks[0]:-}" = "rm" ] || exit 0

# 0 (allow) iff the canonical path is an auto-allowable rm target: under /tmp, or strictly
# inside a git working tree located under $HOME (git-recoverable). The walk stops at $HOME, so
# a dotfiles repo at ~ can't make all of $HOME deletable, and top-level ~ files stay protected.
allowed_target() {
  local canon="$1" d root=""
  case "$canon" in /tmp/?*) return 0 ;; esac
  [ -n "${HOME:-}" ] || return 1
  case "$canon" in "$HOME"/?*) ;; *) return 1 ;; esac
  case "$canon" in *"/.git" | *"/.git/"*) return 1 ;; esac   # protect history, not recoverable
  d="$canon"
  while [ "$d" != "/" ] && [ "$d" != "$HOME" ]; do
    [ -e "$d/.git" ] && { root="$d"; break; }
    d=$(dirname "$d")
  done
  [ -n "$root" ] || return 1              # not inside a git working tree under $HOME
  if [ "$canon" = "$root" ]; then
    # Deleting the repo root folder itself: allow only for a linked worktree, whose `.git` is
    # a file/pointer so the history lives in the main repo and survives. A primary checkout's
    # `.git` is a directory holding the history, so deleting it is unrecoverable — defer.
    [ -f "$root/.git" ] && return 0
    return 1
  fi
  return 0
}

had_operand=0
end_opts=0
i=1
while [ "$i" -lt "${#toks[@]}" ]; do
  t="${toks[$i]}"
  i=$((i + 1))
  # Whitelist every token (flags included, so an operator hidden in a flag like `-rf;rm`
  # can't slip past): any character outside the safe set makes it unsafe to reason about.
  [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/*?[]-')" ] && exit 0
  # A glob in an option-looking token (`-[-]`) can expand to `--` and turn a later `-name`
  # into an operand — never a real option, so defer.
  case "$t" in -*[*?[]*) exit 0 ;; esac
  if [ "$end_opts" = 0 ]; then
    [ "$t" = "--" ] && { end_opts=1; continue; }
    # A bare `-` is a filename to rm, not an option — only skip real `-x`/`--long` options.
    case "$t" in -?*) continue ;; esac
  fi
  had_operand=1
  # No wildcard in a non-final path segment (`a/*/b`): it can expand through a symlink
  # realpath can't see. A slashless glob (`*.rs`) is a final-segment match — fine.
  case "$t" in */*) case "${t%/*}" in *[*?[]*) exit 0 ;; esac ;; esac
  case "$t" in
    /*) canon=$(realpath -m -- "$t" 2>/dev/null) ;;
    *)  canon=$(realpath -m -- "${cwd:-$PWD}/$t" 2>/dev/null) ;;
  esac
  [ -n "$canon" ] || exit 0
  allowed_target "$canon" || exit 0
done

[ "$had_operand" = 1 ] || exit 0
jq -nc '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",permissionDecisionReason:"rm operands are under /tmp or inside a windmill git checkout in $HOME"}}'
