#!/usr/bin/env bash
# PreToolUse allowance for scratch file ops: auto-allow a single, plain, single-line
# `mkdir` / `cp` / `mv` / `touch` / `chmod` / `tar` / `unzip` whose every path operand
# resolves under /tmp. Anything else makes no decision (exit 0) and falls back to the normal
# permission flow — where `Bash(mv:*)` and `Bash(chmod:*)` in the `ask` list prompt. A
# PreToolUse `allow` overrides those ask rules, which is why this is a hook and not an allow
# rule: permission rules match a command prefix, so they can only constrain the FIRST operand.
# `cp /tmp/x ~/.zshrc` matches a `cp /tmp/` prefix, and requiring every operand is the point.
#
# Requiring the sources under /tmp too (not just the destination) keeps this from becoming a
# read-exfiltration path around the `Read(**/.env)` / `Read(**/secrets/**)` deny rules: a copy
# out of the project into /tmp would land the content somewhere `Read(/tmp/**)` allows.
#
# Deny-by-default tokenizing, identical in kind to guard-rm-outside-tmp.sh: every path token
# must consist only of alphanumerics, `. _ / -` and glob chars `* ? [ ]`. That set contains
# none of the characters bash uses for quoting, expansion, or command separation ($ ` ~ { } ( )
# ' " \ ; & | < >), so those forms fail by construction. `realpath -m` then resolves `..` and
# existing symlinks (so /tmp/link -> /etc/passwd is caught), and a wildcard in a non-final path
# segment is refused because it can expand through a symlink realpath cannot see.
#
# `tar` and `unzip` get their own parser: their write destination arrives as a flag VALUE
# (`-C`, `-d`) rather than a positional, and a bundle like `-xzf` consumes the token after it.
# Flags are an allowlist, not a denylist, so `-P` / `--absolute-names` — which turn off tar's
# refusal to extract `..` and absolute member paths — defer rather than needing enumeration.
# Extraction additionally requires an explicit destination under /tmp, or a cwd already under
# /tmp, since otherwise members land in the project checkout.
#
# Residual risk accepted: an archive whose members include a symlink pointing out of /tmp
# followed by a write through it can still escape, because tar applies member symlinks as it
# extracts. The archive itself must be under /tmp to get here, so this is a hazard only for
# archives fetched from an untrusted source into the scratch dir.
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

# 0 iff the token is charset-safe and resolves to a path strictly inside /tmp.
under_tmp() {
  local t="$1" canon
  [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/*?[]-')" ] && return 1
  # No wildcard in a non-final path segment (`a/*/b`): it can expand through a symlink
  # realpath can't see. A slashless glob (`*.rs`) is a final-segment match — fine.
  case "$t" in */*) case "${t%/*}" in *[*?[]*) return 1 ;; esac ;; esac
  case "$t" in
    /*) canon=$(realpath -m -- "$t" 2>/dev/null) ;;
    *)  canon=$(realpath -m -- "${cwd:-$PWD}/$t" 2>/dev/null) ;;
  esac
  [ -n "$canon" ] || return 1
  # /tmp itself is never a target — only paths strictly inside it.
  case "$canon" in /tmp/?*) return 0 ;; esac
  return 1
}

allow() {
  jq -nc --arg r "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",permissionDecisionReason:$r}}'
  exit 0
}

# Bare command word only; wrappers (`timeout cp`), env prefixes, and `/bin/cp` defer.
case "${toks[0]:-}" in
  mkdir | cp | mv | touch) takes_mode=0 ;;
  chmod) takes_mode=1 ;;   # chmod's first operand is a mode, not a path
  tar)   ok_flags='xctzjJavfC'; val_flags='fC' ;;
  unzip) ok_flags='oqnljvd';    val_flags='d'  ;;
  *) exit 0 ;;
esac

# ---------------------------------------------------------------- tar / unzip
if [ -n "${ok_flags:-}" ]; then
  saw_archive=0 saw_dest=0 extracting=0 end_opts=0
  i=1
  while [ "$i" -lt "${#toks[@]}" ]; do
    t="${toks[$i]}"
    i=$((i + 1))
    if [ "$end_opts" = 0 ]; then
      [ "$t" = "--" ] && { end_opts=1; continue; }
      case "$t" in
        -?*)
          flags="${t#-}"
          # Allowlist: a long option, -P/--absolute-names, --transform, -I and friends all
          # leave a residue here and defer rather than being enumerated as denials.
          [ -n "$(printf '%s' "$flags" | tr -d "$ok_flags")" ] && exit 0
          case "$flags" in *x*) extracting=1 ;; esac
          # A flag consuming the next token must be alone in its bundle's final position
          # (`-xzf a.tar`), else the token it eats is ambiguous.
          case "${flags%?}" in *[$val_flags]*) exit 0 ;; esac
          case "${flags: -1}" in
            [$val_flags])
              val="${toks[$i]:-}"
              i=$((i + 1))
              [ -n "$val" ] || exit 0
              under_tmp "$val" || exit 0
              case "${flags: -1}" in
                f) saw_archive=1 ;;
                C | d) saw_dest=1 ;;
              esac
              ;;
          esac
          continue
          ;;
      esac
    fi
    # Positional. For tar these are sources (create) or member names (extract); for unzip the
    # first is the archive. Requiring every one under /tmp is conservative for member names,
    # which are not filesystem paths — those defer rather than being wrongly allowed.
    under_tmp "$t" || exit 0
    [ "${toks[0]}" = "unzip" ] && saw_archive=1
  done

  [ "$saw_archive" = 1 ] || exit 0   # tar without -f reads a tape/stdin; unzip needs an archive
  # Writes land relative to the working directory unless a destination was given.
  if [ "$extracting" = 1 ] || [ "${toks[0]}" = "unzip" ]; then
    [ "$saw_dest" = 1 ] || under_tmp "${cwd:-$PWD}" || exit 0
  fi
  allow "archive paths and extraction target are under /tmp"
fi

# ------------------------------------------- mkdir / cp / mv / touch / chmod
path_operand=0
seen_mode=0
end_opts=0
i=1
while [ "$i" -lt "${#toks[@]}" ]; do
  t="${toks[$i]}"
  i=$((i + 1))

  if [ "$end_opts" = 0 ]; then
    [ "$t" = "--" ] && { end_opts=1; continue; }
    # Skip real options only before the first operand: past that point GNU utils treat a
    # leading-dash token as a filename, so validate it rather than skipping it.
    if [ "$path_operand" = 0 ]; then
      case "$t" in
        -*[*?[]*) exit 0 ;;   # a glob in an option (`-[-]`) can expand to `--`
        -?*) [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._-')" ] && exit 0; continue ;;
      esac
    fi
  fi

  # chmod: consume the mode operand without a path check. Octal, or symbolic clauses.
  if [ "$takes_mode" = 1 ] && [ "$seen_mode" = 0 ]; then
    case "$t" in
      [0-7] | [0-7][0-7] | [0-7][0-7][0-7] | [0-7][0-7][0-7][0-7]) ;;
      *) printf '%s' "$t" | grep -Eq '^[ugoa]*[+=-][rwxXst]*(,[ugoa]*[+=-][rwxXst]*)*$' || exit 0 ;;
    esac
    seen_mode=1
    continue
  fi

  under_tmp "$t" || exit 0
  path_operand=1
done

[ "$path_operand" = 1 ] || exit 0
allow "every path operand is under /tmp"
