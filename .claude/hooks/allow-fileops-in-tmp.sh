#!/usr/bin/env bash
# PreToolUse allowance for scratch file ops: auto-allow `mkdir` / `cp` / `mv` / `touch` /
# `chmod` whose every path operand resolves inside one of the roots `path_class` recognizes —
# under /tmp, inside a git working tree under $HOME, or in an MCP browser cache — and
# `tar` / `unzip` confined to /tmp.
# Anything else makes no decision (exit 0) and falls back to the normal permission flow, except
# for `mv` and `chmod`: those get an explicit `ask`, the only prompt they get (see
# lib-guarded-verb.sh).
#
# The command is read one segment at a time, so chaining and line breaks carry no weight of
# their own: `cd /tmp/scratch && mv /tmp/a /tmp/b` is proved on the operands of the `mv`. A
# decision covers the whole command line, so `allow` is emitted only when every segment is one
# of these verbs proved here or a `cd` that resolved, AND exactly one of them writes (see the
# gate at the foot of this file — an earlier write can change what a later operand means). A
# line that mixes a proven op with some other command makes no decision instead and leaves that
# line to the normal permission flow, rather than waving an unexamined command through with it.
#
# This is a hook rather than an allow rule because permission rules match a command prefix, so
# they can only constrain the FIRST operand. `cp /tmp/x ~/.zshrc` matches a `cp /tmp/` prefix,
# and requiring every operand is the point.
#
# One operation may not straddle two roots, sources included, and a sibling checkout is a
# different root — `path_class` names the git tree, not just its kind. A copy out of a checkout
# into /tmp would be a read-exfiltration path around the `Read(**/secrets/**)` / `Read(**/*.pem)`
# deny rules, since the content lands where `Read(/tmp/**)` allows it to be read back, and one
# out of a repo the Read tool is not confined to would do the same for that repo. Keeping every
# operand of one operation inside a single root closes both without restating those rules here.
# The checkout root itself is what makes an in-repo `mv` or `chmod` auto-allowable: deleting a
# file there has never prompted, and moving or chmod-ing one is not the graver act.
#
# Deny-by-default tokenizing, in the same spirit as guard-rm-outside-tmp.sh: every path token
# must consist only of alphanumerics and `. _ / -`, the one exception being the leading `~/` or
# `$HOME/` that `expand_home_prefix` rewrites first. That set contains none of the characters
# bash uses for quoting, expansion, or command separation ($ ` ~ { } ( ) ' " \ ; & | < >), nor
# any glob character, so all of those forms fail by construction. `canon_path` then resolves
# `..` and existing symlinks, so `/tmp/link` pointing at /etc/passwd is caught.
#
# `tar` and `unzip` keep the stricter rule — /tmp only, and absolute operands only — because
# their positional grammar makes a bare word ambiguous: `tar P -xf ...` is --absolute-names,
# not a file named P, and resolving it as a path would put an option in a root and allow it.
# The other five take relative operands, resolved against the working directory that `cd`
# tracking maintains, since for those a bare word really is a path (a GNU option starts with
# `-`, and the option allowlist below rejects the ones that would change symlink handling).
#
# `tar` and `unzip` get their own parser: their write destination arrives as a flag VALUE
# (`-C`, `-d`) rather than a positional, and a bundle like `-xzf` consumes the token after it.
# Flags are an allowlist, not a denylist, so `-P` / `--absolute-names` — which turn off tar's
# refusal to extract `..` and absolute member paths — defer rather than needing enumeration.
# Extraction additionally requires an explicit destination under /tmp, or a working directory
# already under /tmp, since otherwise members land in the project checkout.
#
# Residual risk accepted: an archive whose members include a symlink pointing out of /tmp
# followed by a write through it can still escape, because tar applies member symlinks as it
# extracts. The archive itself must be under /tmp to get here, so this is a hazard only for
# archives fetched from an untrusted source into the scratch dir.
#
# Assumes `jq`. Path canonicalization goes through `canon_path`, which covers both the Linux dev
# env and macOS; with neither backend available it proves nothing and every op falls back.
set -uo pipefail
. "${BASH_SOURCE[0]%/*}/lib-guarded-verb.sh"

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0
cwd=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null)

# Every bail-out below goes through `defer`: `mv` and `chmod` prompt from here, since no rule
# covers them, while the other verbs stay silent and leave the decision to the normal flow.
guarded=0
for verb in mv chmod; do
  runs_verb "$verb" "$cmd" && { guarded=1; break; }
done
defer() {
  [ "$guarded" = 1 ] && decide ask "$1"
  exit 0
}

has_substitution "$cmd" && defer "command substitution in the command line"

# 0 iff the token is a literal path this hook may reason about. A glob never auto-allows: bash
# expands it only after the hook has decided, so realpath sees the unexpanded pattern —
# `/tmp/link*` canonicalizes to itself and passes, then expands onto a symlink whose target is
# outside, and `cp` and `chmod` follow a command-line symlink, so that is a write to the target.
# (guard-rm-outside-tmp.sh can allow globs because `rm` unlinks the symlink rather than following
# it.) The charset holds none of the characters bash uses for quoting, expansion or separation.
literal_path() {
  case "$1" in *[*?[]*) return 1 ;; esac
  [ -z "$(printf '%s' "$1" | tr -d 'A-Za-z0-9._/-')" ]
}

# Prints the root class of a path token, then the path it resolved to on a second line,
# resolving a relative one against the tracked working directory. Fails, printing nothing,
# when the token is unsafe to reason about or lands outside every root.
operand_class() {
  local t canon alt cls alt_cls=""
  t=$(expand_home_prefix "$1")
  literal_path "$t" || return 1
  case "$t" in
    /*) canon=$(canon_path "$t") ;;
    *)  # A `cd` may fail at runtime and leave the command where it started, so a relative
        # operand has to land in the same root either way.
        [ -n "$seg_cwd" ] || return 1
        canon=$(canon_path "$seg_cwd/$t")
        if [ -n "$alt_cwd" ]; then
          alt=$(canon_path "$alt_cwd/$t")
          [ -n "$alt" ] || return 1
          alt_cls=$(path_class "$alt") || return 1
        fi
        ;;
  esac
  [ -n "$canon" ] || return 1
  cls=$(path_class "$canon") || return 1
  [ -n "$alt_cls" ] && [ "$alt_cls" != "$cls" ] && return 1
  # Class and resolved path together: a caller runs this in a command substitution, so a global
  # set here would be set in that subshell and lost.
  printf '%s\n%s' "$cls" "$canon"
}

# 0 iff the token is charset-safe and resolves to a path strictly inside /tmp. The archive
# parser's stricter check; everything else goes through operand_class.
under_tmp() {
  local t canon
  t=$(expand_home_prefix "$1")
  literal_path "$t" || return 1
  case "$t" in /*) ;; *) return 1 ;; esac
  canon=$(canon_path "$t")
  [ -n "$canon" ] || return 1
  # /tmp itself is never a target — only paths strictly inside it.
  case "$canon" in "$TMP_ROOT"/?*) return 0 ;; esac
  return 1
}

# Proves one `tar` / `unzip` segment ($1 = the verb), whose tokens are in SEG_TOKS.
check_archive_segment() {
  local verb="$1" ok_flags val_flags t flags val
  local saw_archive=0 saw_dest=0 extracting=0 listing=0 end_opts=0 i=1
  case "$verb" in
    tar)   ok_flags='xctzjJavfC'; val_flags='fC' ;;
    unzip) ok_flags='oqnljvd';    val_flags='d'  ;;
  esac
  while [ "$i" -lt "${#SEG_TOKS[@]}" ]; do
    t="${SEG_TOKS[$i]}"
    i=$((i + 1))
    if [ "$end_opts" = 0 ]; then
      [ "$t" = "--" ] && { end_opts=1; continue; }
      case "$t" in
        -?*)
          flags="${t#-}"
          # Allowlist: a long option, -P/--absolute-names, --transform, -I and friends all
          # leave a residue here and defer rather than being enumerated as denials.
          [ -n "$(printf '%s' "$flags" | tr -d "$ok_flags")" ] && defer "unrecognized option \`$t\`"
          case "$flags" in *x*) extracting=1 ;; esac
          case "$verb$flags" in unzip*[lv]*) listing=1 ;; esac
          # A flag consuming the next token must be alone in its bundle's final position
          # (`-xzf a.tar`), else the token it eats is ambiguous.
          case "${flags%?}" in *[$val_flags]*) defer "ambiguous option bundle \`$t\`" ;; esac
          case "${flags: -1}" in
            [$val_flags])
              val="${SEG_TOKS[$i]:-}"
              i=$((i + 1))
              [ -n "$val" ] || defer "option \`$t\` has no value"
              under_tmp "$val" || defer "\`$val\` is outside /tmp"
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
    under_tmp "$t" || defer "\`$t\` is outside /tmp"
    [ "$verb" = "unzip" ] && saw_archive=1
  done

  # tar without -f reads a tape/stdin; unzip needs an archive
  [ "$saw_archive" = 1 ] || defer "no archive operand"
  # Writes land relative to the working directory unless a destination was given. `unzip -l`
  # and `-v` only list, so they need no destination.
  if [ "$extracting" = 1 ] || { [ "$verb" = "unzip" ] && [ "$listing" = 0 ]; }; then
    # An extraction with no destination lands in the working directory. Word splitting cannot
    # tell a `cd` inside a quoted string from one the shell runs, and believing a false one
    # would put an archive's members in the checkout, so once any `cd` is in the line only an
    # explicit destination will do.
    [ "$saw_dest" = 1 ] \
      || { [ "$saw_cd" = 0 ] && [ -n "$seg_cwd" ] && under_tmp "$seg_cwd"; } \
      || defer "extraction target is outside /tmp"
  fi
}

# Proves one `mkdir` / `cp` / `mv` / `touch` / `chmod` segment ($1 = the verb), whose tokens
# are in SEG_TOKS.
check_fileops_segment() {
  local verb="$1" takes_mode ok_opts t cls resolved dest seen_class=""
  local path_operand=0 seen_mode=0 end_opts=0 i=1 rel_operand=0
  local -a ops=()
  # Options are an allowlist per command, so anything that changes how symlinks are followed
  # defers instead of needing enumeration. `cp -L` / `-H` matter most: they dereference while
  # recursing, which copies the CONTENT of a symlink target from outside /tmp into a scratch
  # dir that `Read(/tmp/**)` then exposes. Plain `-r` and `-a` (which implies `-d`) recreate
  # such a symlink as a symlink instead, so no outside content is materialized.
  case "$verb" in
    mkdir) takes_mode=0; ok_opts='pv' ;;
    cp)    takes_mode=0; ok_opts='rRvfnpa' ;;
    mv)    takes_mode=0; ok_opts='vfn' ;;
    touch) takes_mode=0; ok_opts='acmv' ;;
    chmod) takes_mode=1; ok_opts='Rvfc' ;;   # chmod's first operand is a mode, not a path
  esac
  while [ "$i" -lt "${#SEG_TOKS[@]}" ]; do
    t="${SEG_TOKS[$i]}"
    i=$((i + 1))

    if [ "$end_opts" = 0 ]; then
      [ "$t" = "--" ] && { end_opts=1; continue; }
      # Checked at any position, not just before the first operand: GNU utils permute, so
      # `cp /tmp/tree -RL /tmp/out` still enables dereferencing recursion.
      case "$t" in
        -?*)
          # Allowlist: long options and the dereferencing flags leave a residue and defer.
          [ -n "$(printf '%s' "${t#-}" | tr -d "$ok_opts")" ] && defer "unrecognized option \`$t\`"
          continue
          ;;
      esac
    fi

    # chmod: consume the mode operand without a path check. Octal, or symbolic clauses.
    if [ "$takes_mode" = 1 ] && [ "$seen_mode" = 0 ]; then
      case "$t" in
        [0-7] | [0-7][0-7] | [0-7][0-7][0-7] | [0-7][0-7][0-7][0-7]) ;;
        *) printf '%s' "$t" | grep -Eq '^[ugoa]*[+=-][rwxXst]*(,[ugoa]*[+=-][rwxXst]*)*$' || defer "unrecognized mode \`$t\`" ;;
      esac
      seen_mode=1
      continue
    fi

    resolved=$(operand_class "$t") || defer "\`$t\` is outside /tmp and the MCP caches, and not inside a git checkout in \$HOME"
    cls="${resolved%%$'\n'*}"
    # Every operand of one operation stays in one root: see the exfiltration note above.
    [ -n "$seen_class" ] && [ "$cls" != "$seen_class" ] && defer "\`$t\` puts this $verb across two roots"
    seen_class="$cls"
    ops+=("${resolved#*$'\n'}")
    # Against the expanded token, since `~/a` is cwd-independent and only reads as relative
    # before `expand_home_prefix` has run.
    case "$(expand_home_prefix "$t")" in /*) ;; *) rel_operand=1 ;; esac
    path_operand=1
  done

  [ "$path_operand" = 1 ] || defer "no path operand"

  # In directory form the command writes a path it does not name: `cp x dir` writes `dir/x`,
  # and `cp` follows that child when it is a symlink — this checkout is full of them, every
  # `*_ee.rs` pointing into the sibling EE repo. Deriving that child would mean reproducing
  # which name the tool picks (the operand as written, not as resolved — a symlinked source
  # keeps its own name) and how deep `-r` recurses. The form is left unproved instead.
  case "$verb" in
    cp | mv)
      [ "${#ops[@]}" -ge 2 ] || return 0
      # Whether the destination is an existing directory is itself a question about which of
      # the two candidate working directories the command ran in, and only one of them is in
      # `ops`. A `cd` that fails at runtime would otherwise let the form through: the
      # destination resolved against the directory the command never reached is some path that
      # does not exist, while the one it actually ran in is a directory full of symlinks.
      [ -n "$alt_cwd" ] && [ "$rel_operand" = 1 ] \
        && defer "a relative operand after a \`cd\` lands in one of two directories"
      # Index arithmetic rather than `${ops[-1]}`: macOS ships bash 3.2, where a negative
      # subscript is a fatal error and would abort the guard mid-decision.
      dest="${ops[$((${#ops[@]} - 1))]}"
      [ -d "$dest" ] \
        && defer "\`$dest\` already exists as a directory, so this $verb writes a path it does not name"
      ;;
  esac
}

split_segments "$cmd"
seg_cwd="${cwd:-$PWD}"
alt_cwd=""                                # where a `cd` that failed would have left the command
saw_cd=0                                  # a `cd` moved the working directory somewhere
proved=0                                  # how many ops came out inside a single root
only_ours=1                               # ... and nothing else shares the command line

for seg in "${SEGMENTS[@]}"; do
  segment_tokens "$seg"
  case "${SEG_TOKS[0]:-}" in
    "") continue ;;
    mkdir | cp | mv | touch | chmod)
      check_fileops_segment "${SEG_TOKS[0]}"
      proved=$((proved + 1))
      continue
      ;;
    tar | unzip)
      check_archive_segment "${SEG_TOKS[0]}"
      proved=$((proved + 1))
      continue
      ;;
    cd)
      # A `cd` writes nothing, so it never blocks an allow; it only moves where a later relative
      # operand points, to one of the two candidates `apply_cd` describes.
      if [ "$saw_cd" = 0 ] && new_cwd=$(apply_cd "$seg_cwd" "${SEG_TOKS[@]:1}"); then
        alt_cwd="$seg_cwd"
        seg_cwd="$new_cwd"
      else
        # Not the harmless segment an allow assumes: whatever this guard could not account for
        # may be a redirect, and a redirect writes. Leave the line to the normal flow.
        seg_cwd="" alt_cwd=""
        only_ours=0
      fi
      saw_cd=1
      continue
      ;;
  esac
  # Some other command shares the line. If an `mv` or `chmod` runs inside it after all — behind
  # a wrapper, an env prefix or a path — this hook cannot say what it writes to.
  for verb in mv chmod; do
    segment_runs_verb "$verb" "$seg" && defer "$verb is not the leading command word in \`$seg\`"
  done
  only_ours=0
done

# Exactly one write per line. Each segment is proved against the filesystem as it stands now,
# and an earlier write can change what a later operand means: `cp -r /tmp/tree /tmp/live` that
# recreates a symlink out of /tmp turns `/tmp/live/link` — a path under /tmp when this ran —
# into a write through that symlink. Deletes compose safely and guard-rm-outside-tmp.sh allows
# several, because `rm` unlinks a symlink rather than following it.
[ "$proved" -ge 1 ] || exit 0
[ "$only_ours" = 1 ] && [ "$proved" = 1 ] && decide allow "every path operand is inside a single root"
exit 0
