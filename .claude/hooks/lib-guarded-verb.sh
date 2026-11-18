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

# Canonical absolute path: `..` and existing symlinks resolved, missing trailing components
# allowed. Resolving symlinks is the load-bearing half — a lexical normalizer would collapse
# `/tmp/link/..` without seeing where `link` points, and let an operand out of its root.
# GNU `realpath -m` is exactly this; BSD realpath on macOS has no `-m` and exits on it, which
# would leave every operand unresolvable and every delete prompting, so fall back to python3's
# os.path.realpath, which has the same semantics. Trying rather than probing keeps the cost off
# the Bash calls that never reach a path check — most of them. With neither available this
# prints nothing, and every caller treats that as "cannot prove".
canon_path() {
  local out
  out=$(realpath -m -- "$1" 2>/dev/null) && [ -n "$out" ] && { printf '%s' "$out"; return; }
  python3 -c 'import os,sys;sys.stdout.write(os.path.realpath(sys.argv[1]))' "$1" 2>/dev/null
}

# The roots every class is anchored to, in the form a canonicalized operand comes back in. On
# macOS /tmp is a symlink to /private/tmp, so a resolved scratch path never starts with `/tmp`
# and matching the literal would put every scratch path outside every class. Both exist, so
# `cd -P` resolves them without the process canon_path would spawn on every sourcing.
TMP_ROOT=$(cd -P -- /tmp 2>/dev/null && pwd)
[ -n "$TMP_ROOT" ] || TMP_ROOT=/tmp
HOME_ROOT=""
[ -n "${HOME:-}" ] && HOME_ROOT=$(cd -P -- "$HOME" 2>/dev/null && pwd)

# Prints <token> ($1) with a leading `~/`, `$HOME/` or `${HOME}/` — and those three words on
# their own — replaced by the home directory, so the ordinary spelling of a path outside every
# checkout can still be proved. Only that prefix and only those spellings: `~user/` names another
# account, and any other `$` is an expansion nothing here can evaluate, so both stay in the token
# and fail the caller's charset check. A quoted token keeps its quotes and fails there too.
expand_home_prefix() {
  [ -n "$HOME_ROOT" ] || { printf '%s' "$1"; return; }
  case "$1" in
    '~' | '$HOME' | '${HOME}') printf '%s' "$HOME_ROOT" ;;
    '~/'*) printf '%s/%s' "$HOME_ROOT" "${1#'~/'}" ;;
    '$HOME/'*) printf '%s/%s' "$HOME_ROOT" "${1#'$HOME/'}" ;;
    '${HOME}/'*) printf '%s/%s' "$HOME_ROOT" "${1#'${HOME}/'}" ;;
    *) printf '%s' "$1" ;;
  esac
}

# 0 iff <text> ($1) starts with a command that only reads its input. An allowlist, because the
# opposite — naming the shells to avoid — would have to be complete: an unlisted one (`ash`,
# `rbash`, `busybox sh`) executes the body while the guard calls it data. Unrecognized here only
# costs a prompt. Text with no command word in it is not evidence of a reader either.
reads_only() {
  local w
  for w in $1; do
    w="${w//[\"\'\\]/}"
    w="${w%%<<*}"                                   # a redirect needs no space: `cat<<EOF`
    case "$w" in "" | -* | *=* | [0-9]* | '>'* | '<'*) continue ;; esac
    case "${w##*/}" in
      cat | tee | head | tail | grep | sed | awk | sort | uniq | wc | cut | diff | tr \
        | jq | yq | gh | git | base64 | column | envsubst | python | python3 | node \
        | psql | mysql | sqlite3 | wmill) return 0 ;;
    esac
    return 1
  done
  return 1
}

# A heredoc body is data rather than commands only when its delimiter is quoted and nothing
# executes it; a rule doesn't match a verb inside such a body, and a PR body would otherwise
# prompt for every `rm` in its text. Dropping one needs all of that, a delimiter that could
# really open a heredoc, and a terminator line — failing any part, nothing is dropped.
strip_heredoc_bodies() {
  local -a lines=()
  local line delim rest after trimmed piped quoted i j n
  while IFS= read -r line; do lines+=("$line"); done <<< "$1"
  n=${#lines[@]}
  i=0
  while [ "$i" -lt "$n" ]; do
    line="${lines[$i]}"
    printf '%s\n' "$line"
    i=$((i + 1))
    # A `#` opens a comment, and a comment opens no heredoc — including mid-line, as in
    # `echo hi # cat <<EOF`. Cutting there also discards a `#` that is really part of a word or
    # a string, which at worst leaves a real body to be scanned: an extra prompt, never a lost one.
    line="${line%%'#'*}"
    case "$line" in *'<<'*) ;; *) continue ;; esac
    rest="${line#*<<}"
    rest="${rest#-}"                                # <<- strips leading tabs from the body
    rest="${rest#"${rest%%[![:space:]]*}"}"
    delim="${rest%%[[:space:]]*}"
    # Whatever follows the delimiter word decides whether this line could open a heredoc at
    # all. Only a redirect or a pipe can (`cat <<EOF > f`); prose after it means the `<<` sits
    # inside a string (`echo "cat <<EOF and more"`), and dropping down to a line that happens
    # to match would discard the real commands in between. A quote anywhere in the remainder
    # says the same thing, since `echo "cat <<EOF > f"` ends its redirect-looking text with the
    # closing quote. That also refuses `cat <<EOF > "f"`, a real heredoc, which only over-prompts.
    after="${rest#"$delim"}"
    after="${after#"${after%%[![:space:]]*}"}"
    case "$after" in
      *[\"\'\\]*) continue ;;
      "" | '>'* | '<'* | '|'* | [0-9]'>'* | [0-9]'<'*) ;;
      *) continue ;;
    esac
    # A real delimiter is a bare word or one wholly quoted (`<<'EOF'`, `<<\EOF`); a stray quote
    # left in it means the `<<` was quoted prose.
    quoted=0
    case "$delim" in
      \'*\' | \"*\") delim="${delim:1:${#delim}-2}" quoted=1 ;;
      \\?*) delim="${delim#\\}" quoted=1 ;;
    esac
    case "$delim" in
      [A-Za-z_]*) ;;
      *) continue ;;
    esac
    case "$delim" in *[!A-Za-z0-9_]*) continue ;; esac
    # Only a quoted delimiter makes the body inert. Unquoted, the shell expands it before the
    # consumer ever sees it, so a `$(rm -rf ~)` written in the body runs whatever reads it.
    [ "$quoted" = 1 ] || continue
    # Two commands can see this body: the one the `<<` belongs to, and anything it is then piped
    # into. The first is whatever was started last before the `<<`, so splitting the text there
    # on separators and substitution openers and taking the final piece finds `cat` in
    # `--title "fix(agents): …" --body "$(cat <<`, without the title's parenthesis standing in
    # for it. A line continuation (`bash \` then `<<'EOF'`) leaves that piece empty, which is
    # not evidence of a reader and so keeps the body.
    reads_only "$(printf '%s' "${line%%<<*}" | tr ';&|()`' '\n' | grep -v '^[[:space:]]*$' | tail -1)" || continue
    piped="$after"
    while :; do
      case "$piped" in *'|'*) ;; *) break ;; esac
      piped="${piped#*|}"
      reads_only "${piped%%|*}" || continue 2
    done
    j="$i"
    while [ "$j" -lt "$n" ]; do
      trimmed="${lines[$j]#"${lines[$j]%%[![:space:]]*}"}"
      [ "$trimmed" = "$delim" ] && break
      j=$((j + 1))
    done
    [ "$j" -lt "$n" ] && i=$((j + 1))
  done
}

# 0 iff <verb> ($1) runs as a command word in <segment> ($2), which must already be one
# segment (no separator left in it). Wrapper, env-prefix and `/bin/<verb>` forms all count.
segment_runs_verb() {
  local verb="$1" w wrapped=0
  for w in $2; do
    # The shell strips quotes and backslashes before it looks up the command, so `'rm'` and
    # `r\m` run rm and have to compare equal to it.
    w="${w//[\"\'\\]/}"
    case "$w" in
      "$verb" | */"$verb") return 0 ;;
      *=*) ;;                                        # leading env assignment
      -* | *'>'* | *'<'*) ;;                         # a flag, or a leading redirect
      [0-9]*) [ "$wrapped" = 1 ] || break ;;         # a wrapper's duration, not `1:` in prose
      '!' | '{' | '}' | if | then | elif | else | while | until | do) ;;   # never the command
      timeout | time | nice | nohup | stdbuf | command | builtin | noglob | xargs | sudo | env)
        wrapped=1 ;;
      # A wrapper's option value is indistinguishable from a command name (`stdbuf -o L rm`),
      # so past a wrapper the scan runs to the end of the segment instead of stopping at the
      # first ordinary word. Before one, that word is the command and the verb cannot follow
      # it. Nothing bounds the scan: a wrapper takes unboundedly many operands
      # (`env -u A -u B ...`), and any cutoff — a word count, or stopping at the first quoted
      # word — drops the prompt for a real `sudo -u 'root' rm`. Prose after a wrapper is the
      # price, and it only over-prompts.
      *) [ "$wrapped" = 1 ] || break ;;
    esac
  done
  return 1
}

# Splits <command> ($1) into its command segments, into the global array SEGMENTS. Every guard
# reasons one segment at a time, so `a && b` is two commands here rather than one unparsable
# blob, and a newline is a separator like any other.
#
# The split set carries more than `; & |` and newlines: `$(`, backticks and `( )` open a nested
# command, and a separator that only ended statements would read `echo $(rm -rf ~)` as an
# `echo`. Braces are handled as words rather than separators, since splitting on them cuts
# `xargs -I {} … rm` in half and strands the `rm` in a segment that no longer knows a wrapper
# preceded it.
#
# `tr` and not `${1//[...]}`: a `}` inside the bracket expression closes the expansion itself,
# which silently leaves the command unsplit and every separator unseen.
split_segments() {
  local seg
  SEGMENTS=()
  while IFS= read -r seg; do SEGMENTS+=("$seg"); done <<< "$(strip_heredoc_bodies "$1" | tr ';&|()`' '\n')"
}

# 0 iff <command> ($1) carries a command substitution outside a heredoc body. A substitution is
# concatenated into the word it sits in, and splitting on its opener cuts that word in half:
# `/tmp/a/`printf ../../etc`` would be proved as `/tmp/a/`, with the traversal validated as an
# unrelated segment. Nothing here can evaluate it, so a guard proves nothing about such a
# command. Heredoc bodies are excepted — those are data the split has already dropped.
has_substitution() {
  case "$(strip_heredoc_bodies "$1")" in
    *'$('* | *'`'*) return 0 ;;
  esac
  return 1
}

# Reads <segment> ($1) into the global array SEG_TOKS, dropping the shell keywords that can
# precede a command word so that `then rm -rf x` is analyzed as the `rm` it runs. Word
# splitting only: quotes are left in the token and fail the guards' charset check downstream,
# which is what keeps `rm -rf "$HOME/x"` unprovable.
segment_tokens() {
  SEG_TOKS=()
  read -r -a SEG_TOKS <<< "$1"
  while [ "${#SEG_TOKS[@]}" -gt 0 ]; do
    case "${SEG_TOKS[0]}" in
      '!' | '{' | '}' | if | then | elif | else | while | until | do) SEG_TOKS=("${SEG_TOKS[@]:1}") ;;
      *) break ;;
    esac
  done
}

# Prints the directory a `cd` lands in, given the current one ($1) and the tokens after the
# `cd` ($2...). Fails, printing nothing, when the destination cannot be resolved — a variable,
# `-`, an option, a relative path, no operand at all (`cd` alone is $HOME), or more than one.
#
# Resolving says nothing about whether the `cd` will SUCCEED: the destination may not exist, and
# `;` runs the next command anyway, leaving it in the directory it started in. So a caller may
# never treat this as the working directory outright — it is one of two candidates, and a
# relative operand has to be provable against the one the command started in as well. That also
# makes a `cd` word splitting invented out of quoted text harmless: it can only add a candidate,
# never drop one. Past the first `cd` the branching outruns two candidates, so a caller that
# sees a second gives up on relative operands entirely.
apply_cd() {
  local cwd="$1" t
  shift
  [ "$#" -eq 1 ] || return 1
  t=$(expand_home_prefix "$1")
  [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/-')" ] && return 1
  # Absolute only. A relative destination is not `$cwd/$t`: the shell searches $CDPATH first,
  # so `cd ssh` may land in /etc/ssh, and this cannot see the caller's $CDPATH to rule it out.
  case "$t" in /*) ;; *) return 1 ;; esac
  canon_path "$t"
}

# Prints the class of a canonical path and returns 0: `tmp` for one strictly under /tmp,
# `mcp-cache` for one in a browser-automation cache the MCP servers rebuild on demand, or
# `repo:<root>` for one strictly inside the git working tree at <root>, itself under $HOME.
# Fails, printing nothing, for anything else — those are the only roots the guards are willing
# to touch unprompted. The root is part of the class so that a caller pairing two operands can
# tell one checkout from another: sibling repos are separate permission boundaries, not one.
#
# The `repo` class trades on "this is a project under version control" being lower-stakes than
# the same act elsewhere — NOT on full recoverability: committed content is restorable via git,
# but untracked / .gitignore'd / uncommitted content, and an independent nested repo's history
# under a recursively-deleted parent, are NOT. Accepted as a deliberate convenience tradeoff.
#
# The walk stops at $HOME, so a dotfiles repo at ~ can't put all of $HOME in a class, and
# top-level ~ files stay out of one. A working tree's own root folder counts only when it is a
# linked worktree, whose `.git` is a pointer file so the history lives in the main repo and
# survives; a primary checkout's `.git` is a directory holding the history itself, so losing it
# is unrecoverable.
#
# Some paths are in no class in any root, /tmp included. Git history, and the agent's own guards
# and settings, because removing those is what removes the prompt on everything else. And every
# path `.claude/settings.json` refuses to read — `.env`, `secrets/`, `*.pem`, `*.key`,
# `credentials.json`, `.secret*` — because a `cp` or `mv` that is auto-allowed on both ends
# would rename one out of those globs and hand back through `Read` exactly what they deny.
path_class() {
  local canon="$1" d root="" folded
  # Matched against a lowercased copy: APFS is case-insensitive by default, so `.GIT` and `.git`
  # are one directory, and a case-sensitive list would leave the history — and these guards' own
  # settings — one keystroke from an auto-allowed delete. On a case-sensitive volume a genuinely
  # distinct `.GIT/` over-matches, which costs a prompt and nothing else. `tr` and not `${x,,}`:
  # macOS ships bash 3.2, which has no case-folding expansion.
  folded=$(printf '%s' "$canon" | tr 'A-Z' 'a-z')
  case "$folded" in
    *"/.git" | *"/.git/"* | *"/.claude" | *"/.claude/"*) return 1 ;;
    *"/.env" | *"/.env."*) return 1 ;;
    *"/secrets" | *"/secrets/"*) return 1 ;;
    *.pem | *.key | *"/credentials.json") return 1 ;;
    *"/.secret"* | *.secret | *.secrets) return 1 ;;
  esac
  case "$canon" in "$TMP_ROOT"/?*) printf 'tmp'; return 0 ;; esac
  [ -n "$HOME_ROOT" ] || return 1
  # The Playwright MCP servers download browsers into `ms-playwright` and open a throwaway
  # profile per session under `ms-playwright-mcp`; nothing prunes either, so they grow without
  # bound (10G here) and clearing one costs a re-download and nothing else. They sit outside
  # every checkout, where no other class reaches them. Matched including the root itself,
  # unlike the repo class, because wiping the whole directory is the point.
  # Each root is named exactly and then again with `/*`, rather than one trailing `*`: a case
  # pattern's `*` spans the `-` as well, which would put a sibling somebody created themselves —
  # `ms-playwright-mcp-backup` — in a class that auto-allows deleting it.
  case "$canon" in
    "$HOME_ROOT"/Library/Caches/ms-playwright | "$HOME_ROOT"/Library/Caches/ms-playwright/* \
      | "$HOME_ROOT"/Library/Caches/ms-playwright-mcp | "$HOME_ROOT"/Library/Caches/ms-playwright-mcp/* \
      | "$HOME_ROOT"/.cache/ms-playwright | "$HOME_ROOT"/.cache/ms-playwright/* \
      | "$HOME_ROOT"/.cache/ms-playwright-mcp | "$HOME_ROOT"/.cache/ms-playwright-mcp/*)
      printf 'mcp-cache'
      return 0
      ;;
  esac
  case "$canon" in "$HOME_ROOT"/?*) ;; *) return 1 ;; esac
  d="$canon"
  while [ "$d" != "/" ] && [ "$d" != "$HOME_ROOT" ]; do
    [ -e "$d/.git" ] && { root="$d"; break; }
    d=$(dirname "$d")
  done
  [ -n "$root" ] || return 1              # not inside a git working tree under $HOME
  if [ "$canon" = "$root" ]; then
    [ -f "$root/.git" ] || return 1
  fi
  printf 'repo:%s' "$root"
}

# 0 iff <verb> ($1) runs as a command word anywhere in <command> ($2). Mirrors how a Bash
# permission rule matches, so that owning the prompt here doesn't narrow what used to prompt:
# a guard consults this before it starts proving segments, and every bail-out it then takes
# is a prompt for exactly the commands a rule would have caught.
runs_verb() {
  local verb="$1" seg
  split_segments "$2"
  for seg in "${SEGMENTS[@]}"; do
    segment_runs_verb "$verb" "$seg" && return 0
  done
  return 1
}

# Emit a PreToolUse decision and exit. `ask` is the ordinary permission prompt.
decide() {
  jq -nc --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:$d,permissionDecisionReason:$r}}'
  exit 0
}
