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
# The split set also carries the characters that open a nested command — `$(`, backticks and
# `( )` — because a rule matches the verb inside one (`echo $(rm -rf ~)` prompts), and a
# separator that only ends statements would read that as an `echo`. Braces are handled as
# words rather than separators, since splitting on them cuts `xargs -I {} … rm` in half and
# strands the `rm` in a segment that no longer knows a wrapper preceded it.

# A heredoc body is data rather than commands only when its delimiter is quoted and nothing
# executes it; a rule doesn't match a verb inside such a body, and a PR body would otherwise
# prompt for every `rm` in its text. Dropping one needs all of that, a delimiter that could
# really open a heredoc, and a terminator line — failing any part, nothing is dropped.
strip_heredoc_bodies() {
  local -a lines=()
  local line delim rest after trimmed word quoted i j n
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
    # A body is only data when nothing executes it. Fed to a shell — `bash <<EOF`,
    # `cat <<EOF | bash`, `ssh host <<EOF` — every line in it is a command, so it has to be
    # scanned like one.
    # Separators are split off first: none of `cat <<'EOF'|bash`, `$(bash <<'EOF'` or
    # `(bash <<'EOF')` puts whitespace around the shell that runs the body.
    for word in $(printf '%s' "$line" | tr ';&|()`' ' '); do
      word="${word//[\"\'\\]/}"
      word="${word%%<<*}"                           # a redirect needs no space: `bash<<EOF`
      case "${word##*/}" in
        bash | sh | zsh | dash | ksh | fish | csh | tcsh | ssh | eval | source) continue 2 ;;
      esac
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
        -* | *'>'* | *'<'*) ;;                         # a flag, or a leading redirect
        [0-9]*) [ "$wrapped" = 1 ] || break ;;         # a wrapper's duration, not `1:` in prose
        '!' | '{' | '}' | if | then | elif | else | while | until | do) ;;   # never the command
        timeout | time | nice | nohup | stdbuf | command | builtin | noglob | xargs | sudo | env)
          wrapped=1 ;;
        # A wrapper's option value is indistinguishable from a command name (`stdbuf -o L rm`),
        # so past a wrapper the scan runs to the end of the segment instead of stopping at the
        # first ordinary word. Before one, that word is the command and the verb cannot follow
        # it. Nothing bounds the scan: a wrapper takes unboundedly many operands
        # (`env -u A -u B …`), and any cutoff — a word count, or stopping at the first quoted
        # word — drops the prompt for a real `sudo -u 'root' rm`. Prose after a wrapper is the
        # price, and it only over-prompts.
        *) [ "$wrapped" = 1 ] || break ;;
      esac
    done
    # `tr` and not `${2//[...]}`: a `}` inside the bracket expression closes the expansion
    # itself, which silently leaves the command unsplit and every separator unseen.
  done <<< "$(strip_heredoc_bodies "$2" | tr ';&|()`' '\n')"
  return 1
}

# Emit a PreToolUse decision and exit. `ask` is the ordinary permission prompt.
decide() {
  jq -nc --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:$d,permissionDecisionReason:$r}}'
  exit 0
}
