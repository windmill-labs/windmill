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
# Deny-by-default tokenizing, in the same spirit as guard-rm-outside-tmp.sh: every path token
# must consist only of alphanumerics and `. _ / -`. That set contains none of the characters
# bash uses for quoting, expansion, or command separation ($ ` ~ { } ( ) ' " \ ; & | < >), nor
# any glob character, so all of those forms fail by construction. `realpath -m` then resolves
# `..` and existing symlinks, so `/tmp/link` pointing at /etc/passwd is caught.
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
  # Globs never auto-allow. Bash expands them only after this hook has decided, so realpath
  # sees the unexpanded pattern: `/tmp/link*` canonicalizes to itself and passes, then
  # expands onto a symlink whose target is outside /tmp. chmod and cp follow command-line
  # symlinks, so that is a write to the target. guard-rm-outside-tmp.sh can allow globs
  # because `rm` unlinks the symlink itself rather than following it.
  case "$t" in *[*?[]*) return 1 ;; esac
  [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/-')" ] && return 1
  # Absolute only. Resolving a relative operand against the cwd makes any bare word look like
  # a safe path whenever the cwd is under /tmp, while the tool itself reads it as an option:
  # `tar P -xf ...` is --absolute-names, not ./P, and `cp /tmp/t -RL /tmp/o` is a
  # dereferencing recursive copy, not a file named -RL.
  case "$t" in /*) ;; *) return 1 ;; esac
  canon=$(realpath -m -- "$t" 2>/dev/null)
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
# Options are an allowlist per command, so anything that changes how symlinks are followed
# defers instead of needing enumeration. `cp -L` / `-H` matter most: they dereference while
# recursing, which copies the CONTENT of a symlink target from outside /tmp into a scratch
# dir that `Read(/tmp/**)` then exposes. Plain `-r` and `-a` (which implies `-d`) recreate
# such a symlink as a symlink instead, so no outside content is materialized.
case "${toks[0]:-}" in
  mkdir) takes_mode=0; ok_opts='pv' ;;
  cp)    takes_mode=0; ok_opts='rRvfnpa' ;;
  mv)    takes_mode=0; ok_opts='vfn' ;;
  touch) takes_mode=0; ok_opts='acmv' ;;
  chmod) takes_mode=1; ok_opts='Rvfc' ;;   # chmod's first operand is a mode, not a path
  tar)   ok_flags='xctzjJavfC'; val_flags='fC' ;;
  unzip) ok_flags='oqnljvd';    val_flags='d'  ;;
  *) exit 0 ;;
esac

# ---------------------------------------------------------------- tar / unzip
if [ -n "${ok_flags:-}" ]; then
  saw_archive=0 saw_dest=0 extracting=0 listing=0 end_opts=0
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
          case "${toks[0]}$flags" in unzip*[lv]*) listing=1 ;; esac
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
  # Writes land relative to the working directory unless a destination was given. `unzip -l`
  # and `-v` only list, so they need no destination.
  if [ "$extracting" = 1 ] || { [ "${toks[0]}" = "unzip" ] && [ "$listing" = 0 ]; }; then
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
    # Checked at any position, not just before the first operand: GNU utils permute, so
    # `cp /tmp/tree -RL /tmp/out` still enables dereferencing recursion.
    case "$t" in
      -?*)
        # Allowlist: long options and the dereferencing flags leave a residue and defer.
        [ -n "$(printf '%s' "${t#-}" | tr -d "$ok_opts")" ] && exit 0
        continue
        ;;
    esac
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
