#!/usr/bin/env bash
# PreToolUse guard for `rm`: auto-allow deletes whose every operand is a whitelisted target —
# under /tmp, or inside a git working tree located in $HOME (a version-controlled project dir).
# Any other command that runs `rm` gets an explicit `ask`, which is the ordinary permission
# prompt and the only one `rm` gets (see lib-guarded-verb.sh); a command that runs no `rm` at
# all makes no decision (exit 0).
#
# The command is read one segment at a time, so chaining and line breaks carry no weight of
# their own: `rm -f /tmp/a && rm -rf /tmp/b` is two deletes, each proved on its own operands.
# A decision covers the whole command line, so `allow` is emitted only when every segment is
# an `rm` this guard proved or a `cd` it could resolve. A line that mixes a proven `rm` with
# some other command makes no decision instead and leaves that line to the normal permission
# flow: the delete is not what needed a prompt, and waving the rest of the line through with
# it would turn a trailing `rm -f /tmp/x` into a way to auto-approve anything.
#
# Deny-by-default: every token must consist only of a safe character set (alphanumerics,
# `. _ / -` and glob chars `* ? [ ]`). That set contains none of the characters bash uses for
# quoting, expansion, or command separation ($ ` ~ { } ( ) ' " \ ; & | < >), so those forms
# fail by construction rather than needing to be enumerated. `realpath -m` then resolves `..`
# and existing symlinks (so a symlink out of the allowed roots is caught), and a wildcard in a
# non-final path segment is refused because it can expand through a symlink realpath can't see.
#
# Which targets those two roots cover, and the tradeoff they rest on, is `path_class` in
# lib-guarded-verb.sh. Globs auto-allow only under /tmp — elsewhere their expansion
# could reach `.git` or a dotfile the literal checks never see. Relative operands resolve
# against the working directory the command runs from, which a `cd` in an earlier segment
# moves; once a `cd` is one this guard cannot resolve, that directory is unknown and a
# relative operand can no longer be proved.
#
# Assumes GNU `realpath` (-m) and `jq`, both present in this repo's Linux dev env.
set -uo pipefail
. "${BASH_SOURCE[0]%/*}/lib-guarded-verb.sh"

input=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0
cwd=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null)

# Every bail-out below goes through `defer`, so the forms this guard refuses to reason about —
# wrapped, quoted, expanded — still reach the user as a prompt whenever an `rm` runs among them.
runs_verb rm "$cmd" && guarded=1 || guarded=0
defer() {
  [ "$guarded" = 1 ] && decide ask "$1"
  exit 0
}

# Proves one `rm` segment, whose tokens are in SEG_TOKS with `rm` at index 0, resolving relative
# operands against $seg_cwd. Returns only once every operand is an auto-allowable target;
# anything it cannot prove defers instead.
check_rm_segment() {
  local i=1 t canon had_operand=0 end_opts=0
  while [ "$i" -lt "${#SEG_TOKS[@]}" ]; do
    t="${SEG_TOKS[$i]}"
    i=$((i + 1))
    # Whitelist every token (flags included, so an operator hidden in a flag like `-rf;rm`
    # can't slip past): any character outside the safe set makes it unsafe to reason about.
    [ -n "$(printf '%s' "$t" | tr -d 'A-Za-z0-9._/*?[]-')" ] && defer "unsafe characters in \`$t\`"
    # A glob in an option-looking token (`-[-]`) can expand to `--` and turn a later `-name`
    # into an operand — never a real option, so defer.
    case "$t" in -*[*?[]*) defer "glob inside the option \`$t\`" ;; esac
    if [ "$end_opts" = 0 ]; then
      [ "$t" = "--" ] && { end_opts=1; continue; }
      # Skip real options only before the first operand. A bare `-` is a filename, and under
      # POSIXLY_CORRECT GNU rm stops option parsing at the first operand, so a later `-name`
      # is a filename too — validate it rather than skipping it.
      if [ "$had_operand" = 0 ]; then
        case "$t" in -?*) continue ;; esac
      fi
    fi
    had_operand=1
    # No wildcard in a non-final path segment (`a/*/b`): it can expand through a symlink
    # realpath can't see. A slashless glob (`*.rs`) is a final-segment match — fine.
    case "$t" in */*) case "${t%/*}" in *[*?[]*) defer "glob in a non-final segment of \`$t\`" ;; esac ;; esac
    case "$t" in
      /*) canon=$(realpath -m -- "$t" 2>/dev/null) ;;
      *)  [ -n "$seg_cwd" ] || defer "\`$t\` is relative to a working directory left by a \`cd\` this guard cannot resolve"
          canon=$(realpath -m -- "$seg_cwd/$t" 2>/dev/null) ;;
    esac
    [ -n "$canon" ] || defer "cannot resolve \`$t\`"
    # A glob may auto-allow only under /tmp, where everything is deletable. Elsewhere its
    # expansion could match `.git`, a dotfile like `.*`, or a nested checkout root that the
    # literal-path checks never see — so require literal operands in git repos.
    case "$t" in *[*?[]*) case "$canon" in /tmp/?*) ;; *) defer "glob \`$t\` is outside /tmp" ;; esac ;; esac
    path_class "$canon" >/dev/null || defer "\`$canon\` is outside /tmp and not inside a git checkout in \$HOME"
  done
  [ "$had_operand" = 1 ] || defer "no operand"
}

split_segments "$cmd"
seg_cwd="${cwd:-$PWD}"
proved=0                                  # at least one `rm` segment came out auto-allowable
only_ours=1                               # ... and nothing else shares the command line

for seg in "${SEGMENTS[@]}"; do
  segment_tokens "$seg"
  case "${SEG_TOKS[0]:-}" in
    "") continue ;;
    rm)
      check_rm_segment
      proved=1
      continue
      ;;
    cd)
      # A `cd` writes nothing, so it never blocks an allow; it only moves where the operands
      # of a later segment point. Tracking it is only sound while every step stays known and
      # inside the allowed roots, so an unresolvable destination, or one this guard would not
      # delete in, makes the working directory unknown — and a `cd` is only followed from a
      # known one, so a later `cd` cannot walk it back into a root it has already left.
      if [ -n "$seg_cwd" ] && new_cwd=$(apply_cd "$seg_cwd" "${SEG_TOKS[@]:1}") \
         && path_class "$new_cwd" >/dev/null; then
        seg_cwd="$new_cwd"
      else
        seg_cwd=""
      fi
      continue
      ;;
  esac
  # Some other command shares the line. If an `rm` runs inside it after all — behind a wrapper,
  # an env prefix or a path — this guard cannot say what it deletes.
  segment_runs_verb rm "$seg" && defer "rm is not the leading command word in \`$seg\`"
  only_ours=0
done

[ "$proved" = 1 ] || exit 0
[ "$only_ours" = 1 ] && decide allow 'rm operands are under /tmp or inside a git checkout in $HOME'
exit 0
