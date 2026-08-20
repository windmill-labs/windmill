#!/usr/bin/env bash
# Decision table for the two scratch-dir PreToolUse guards. Run: bash .claude/hooks/test-hooks.sh
#
# What this pins is the `ask` column: a matcher change that turns one into a no-decision drops
# that command's only prompt (see lib-guarded-verb.sh). The wrapper, nested-command and quoted
# rows are the ones that catch it.
#
# The `allow` column carries its own weight, because a decision covers the whole command line:
# `allow` may only appear where every segment was proved here, and a line that also runs
# something unexamined has to come out `none` so the normal permission flow still sees it.
set -uo pipefail
H="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
CWD="$(git -C "$H" rev-parse --show-toplevel)"
OUT="$HOME/not-a-git-tree"   # never written to; only the guards' path checks look at it
fails=0

# A tree's own root is auto-allowable only when it is a LINKED worktree, whose `.git` is a
# pointer file so the history lives in the main repo and survives; a primary checkout's `.git`
# is the history itself. The suite runs from either kind, so the rows that name the root follow
# the one it is run in — which is also what pins both halves of that rule.
if [ -f "$CWD/.git" ]; then
  ROOT_SOLO=allow ROOT_CHAINED=none      # linked worktree
else
  ROOT_SOLO=ask ROOT_CHAINED=ask         # primary checkout
fi

run() { # run <hook> <allow|ask|none> <command>
  local hook="$1" want="$2" cmd="$3" out got
  out=$(jq -nc --arg c "$cmd" --arg w "$CWD" \
        '{tool_name:"Bash",tool_input:{command:$c},cwd:$w}' | "$H/$hook" 2>&1)
  if [ -z "$out" ]; then
    got=none
  else
    got=$(printf '%s' "$out" | jq -r '.hookSpecificOutput.permissionDecision // "PARSE-ERROR"' 2>/dev/null || echo PARSE-ERROR)
  fi
  local shown="${cmd//$'\n'/ ⏎ }"
  if [ "$got" = "$want" ]; then
    printf '  ok   %-5s %s\n' "$got" "$shown"
  else
    printf 'FAIL   want=%-5s got=%-5s %s\n       %s\n' "$want" "$got" "$shown" "$out"
    fails=$((fails + 1))
  fi
}

echo "== guard-rm-outside-tmp.sh =="
G=guard-rm-outside-tmp.sh
run $G allow "rm -rf /tmp/scratch/x"
run $G allow "rm -rf /tmp/scratch/*"
run $G allow "rm -rf $CWD/frontend/scratch"
run $G ask   "rm -rf /tmp"
run $G ask   "rm -rf $OUT"
run $G ask   "rm -rf $CWD/.git"
run $G ask   "rm -rf $CWD/.claude/hooks"          # the guards may not delete themselves
run $G ask   "rm $CWD/.claude/settings.json"
run $G ask   "rm $CWD/.claude/settings.local.json"
run $G ask   "rm -rf $CWD/backend/.env"
run $G ask   "rm -rf $CWD/.env.local"
run $G $ROOT_SOLO "rm -rf $CWD"
run $G ask   "rm -rf $CWD/*"
run $G ask   "rm -rf /etc/passwd"
# The MCP caches are the one allowed root outside /tmp and the checkouts, and `~/` and `$HOME/`
# the one expansion the charset check tolerates — so the row that matters is the one proving the
# prefix does not carry anything else along with it.
run $G allow "rm -rf ~/Library/Caches/ms-playwright-mcp"
run $G allow "rm -rf ~/.cache/ms-playwright-mcp"          # the Linux spelling of the same root
run $G allow 'rm -rf $HOME/Library/Caches/ms-playwright-mcp/mcp-chrome-*'
run $G ask   "rm -rf ~/.cache/ms-playwright-mcp-backup"   # a sibling, not the cache
run $G ask   "rm -rf ~/not-a-git-tree"
# The exclusion list is the whole protection for these paths — the `repo:` class allows deletes
# everywhere else in a checkout — and macOS resolves `.GIT` to `.git`, so the fold is what keeps
# the list from failing open there. Pattern-matched, so the row holds on either platform.
run $G ask   "rm -rf $CWD/.GIT"
run $G ask   "rm $CWD/.CLAUDE/settings.json"
run $G ask   "rm -rf $CWD/backend/.ENV"
run $G ask   'rm -rf "$HOME/x"'
run $G ask   "rm -rf /tmp/../$OUT"
run $G none  "ls /tmp && rm -rf /tmp/x"   # proved delete, unexamined neighbour
run $G ask   'echo $(rm -rf /etc)'
run $G ask   'echo `rm -rf /etc`'
run $G ask   "{ rm -rf /etc; }"
run $G allow "{ rm -rf /tmp/scratch/x; }"         # the keyword drops, the delete still proves
run $G ask   "find . -name x | xargs rm"
run $G ask   "timeout 5 rm -rf /tmp/x"
run $G ask   "stdbuf -o L rm -rf /etc"
run $G ask   "FOO=bar rm -rf /tmp/x"
run $G ask   "/bin/rm -rf /tmp/x"
run $G ask   "'rm' -rf /etc"
run $G ask   'r\m -rf /etc'
run $G ask   "! rm -rf /etc"
run $G ask   "if true; then rm -rf /etc; fi"
run $G ask   ">/dev/null rm -rf $OUT"
# Data that merely mentions a verb is not a command. Both of these prompted in the field.
run $G none  "$(printf 'gh pr create --body "$(cat <<%sEOF%s\ndrop `rm` and `mv` from the ask list\nrm is now guarded here\nEOF\n)"' "'" "'")"
run $G none  "$(printf 'claude -p "run these in order:\n1: rm -rf /tmp/a\n2: mv /tmp/b /tmp/c"')"
# A wrapper's own flags and assignments are unbounded, so they may not be charged against the
# scan that looks past it — these run rm and must prompt.
run $G ask   "env -i HOME=/tmp PATH=/usr/bin LANG=C USER=root SHELL=/bin/sh rm -rf /etc"
run $G ask   "sudo -E -H -u root FOO=1 BAR=2 rm -rf $OUT"
run $G ask   "xargs -a f -d d -E e -I {} -L 1 -n 1 rm /etc"
run $G ask   "env -u A -u B -u C -u D -u E -u F -u G rm -rf /etc"
run $G ask   "sudo -u 'root' rm -rf /etc"
run $G ask   "$(printf 'echo hi # cat <<EOF\nrm -rf /etc\nEOF')"
# A `<<` inside a quoted string or a comment opens no heredoc, so the command under it is real.
run $G ask   "$(printf 'echo "cat <<EOF"\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'echo "cat <<EOF and more"\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'echo "cat <<EOF "\nrm -rf /etc\nEOF')"
run $G ask   "$(printf '# usage: cat <<EOF\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'echo "cat <<EOF > f"\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'echo "cat <<true > /tmp/a"\nrm -rf /etc\ntrue')"
run $G ask   "$(printf "echo 'cat <<EOF | tee'\nrm -rf /etc\nEOF")"
# A body fed to a shell is executed, so it is commands and not data.
run $G ask   "$(printf 'bash <<EOF\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'cat <<EOF | bash\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'ssh host <<EOF\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'bash<<%sEOF%s\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf '/bin/sh <<EOF\nrm -rf /etc\nEOF')"
run $G ask   "$(printf 'cat <<%sEOF%s|bash\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf 'out=$(bash <<%sEOF%s\nrm -rf /etc\nEOF\n)' "'" "'")"
run $G ask   "$(printf 'bash \\\n  <<%sEOF%s\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf 'ash <<%sEOF%s\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf 'busybox sh <<%sEOF%s\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf 'sudo -s <<%sEOF%s\nrm -rf /etc\nEOF' "'" "'")"
run $G ask   "$(printf '(bash <<%sEOF%s)\nrm -rf /etc\nEOF' "'" "'")"

# A redirect or pipe after the delimiter is still a real heredoc.
run $G none  "$(printf 'cat <<%sEOF%s > /tmp/a\nrm -rf /etc\nEOF' "'" "'")"
run $G none  "$(printf 'cat <<%sEOF%s 2>&1 | tee /tmp/a\nrm -rf /etc\nEOF' "'" "'")"
# An unquoted body is expanded before its consumer sees it, so it is code.
run $G ask   "$(printf 'cat <<EOF > /tmp/a\n$(rm -rf /etc)\nEOF')"
run $G ask   "$(printf 'cat <<EOF > /tmp/a\nrm -rf /etc\nEOF')"
# ... but a real command after a heredoc still is one.
run $G ask   "$(printf 'cat <<EOF > /tmp/s.sh\nhello\nEOF\nrm -rf %s' "$OUT")"
run $G ask   "$(printf 'echo "a << b"\nrm -rf %s' "$OUT")"
run $G none  "git rm frontend/foo.ts"
run $G none  'echo $(ls /tmp)'
run $G none  'grep -rn "rm" backend/'
run $G none  "cargo build --release"

# Chaining and line breaks are not themselves a reason to prompt: each segment is proved on its
# own operands, and a `cd` moves where a relative one points.
run $G allow "rm -f /tmp/a; rm -rf /tmp/b"
run $G allow "$(printf 'rm -f /tmp/a\nrm -rf %s/frontend/scratch' "$CWD")"
run $G allow "cd /tmp/scratch && rm -rf sub"
run $G none  "mkdir -p /tmp/x && rm -rf /tmp/x"
run $G ask   "$(printf 'ls /tmp\nrm -rf /etc')"
# A `cd` this guard can resolve is where the relative operand lands; one it cannot leaves the
# working directory unknown, and an unknown one proves nothing.
run $G ask   "cd /etc && rm -rf foo"
run $G ask   'cd "$D" && rm -rf foo'
run $G ask   "cd $CWD && rm -rf .git"
run $G ask   "cd /etc && cd /tmp/scratch && rm -rf sub"   # a cd out is not walked back
# A `cd` can fail at runtime, and `;` runs the delete from where the command started, so a
# relative operand is proved from both directories.
run $G ask   "cd /tmp/does-not-exist; rm -rf .git"
run $G ask   "cd /tmp/does-not-exist; rm -rf backend/.env"
run $G ask   "cd /tmp/a && cd /tmp/b && rm -rf sub"
run $G ask   "rm -rf /tmp/clone/.git"                     # history is never in a class
run $G ask   "rm -rf /tmp/scratch/id_rsa.key"
run $G none  "cd /tmp >$OUT; rm -f /tmp/a"
# A substitution is concatenated into its word, so splitting on it would prove only the literal
# half; a relative `cd` is not $cwd/$t either, since the shell searches $CDPATH first.
run $G ask   'rm -rf /tmp/a/`printf ../../etc`'
run $G ask   'rm -rf /tmp/a/$(printf ../../etc)'
run $G ask   "cd ssh && rm -rf moduli"

echo
echo "== allow-fileops-in-tmp.sh =="
A=allow-fileops-in-tmp.sh
run $A allow "mv /tmp/a /tmp/b"
run $A allow "chmod 755 /tmp/a"
run $A allow "cp -r /tmp/a /tmp/b"
run $A allow "tar -xzf /tmp/a.tar.gz -C /tmp/out"
run $A ask   "mv /tmp/a $OUT"
run $A ask   "mv $CWD/AGENTS.md /tmp/a"
run $A $ROOT_SOLO "chmod -R 777 $CWD"
run $A none  "ls && mv /tmp/a /tmp/b"     # proved move, unexamined neighbour
run $A ask   'echo $(mv /tmp/a /etc)'
run $A ask   "timeout --signal KILL 5 mv /tmp/a /etc"
run $A ask   "time -f FORMAT chmod 777 $OUT"
run $A ask   "'mv' /tmp/a /etc"
run $A ask   'ch\mod 777 /etc'
run $A none  "$(printf 'claude -p "run these in order:\n1: rm -rf /tmp/a\n2: mv /tmp/b /tmp/c"')"
run $A ask   "env -i A=1 B=2 C=3 D=4 E=5 F=6 mv /tmp/a /etc"
run $A none  "cp $CWD/AGENTS.md /tmp/a"
run $A none  "tar -xzf /tmp/a.tar.gz -C $OUT"
run $A none  "cargo build"
run $A ask   "chmod -R 777 $CWD/.GIT"
run $A allow "chmod -R 755 ~/Library/Caches/ms-playwright-mcp"
run $A ask   "chmod -R 777 ~/Library/Caches/ms-playwright-mcp-backup"
# The home prefix reaches this guard through `operand_class`, not the rm guard's own resolver.
case "$CWD" in
  "$HOME"/*) run $A allow "mv ~${CWD#"$HOME"}/frontend/a.ts ~${CWD#"$HOME"}/frontend/b.ts" ;;
esac

run $A none  "mkdir -p /tmp/x; mv /tmp/a /tmp/x; chmod 755 /tmp/x"   # one write per line
run $A none  "$(printf 'mv /tmp/a /tmp/b\nchmod 755 /tmp/b')"
run $A ask   "ls && mv /tmp/a /etc"
run $A $ROOT_CHAINED "$(printf 'mkdir -p /tmp/x\nchmod -R 777 %s' "$CWD")"
run $A allow "cd /tmp/x && tar -xzf /tmp/a.tar.gz -C /tmp/out"
# The checkout is a root of its own, so an in-repo move or chmod is as auto-allowable as the
# in-repo delete already was — but one operation may not straddle it and /tmp.
run $A allow "chmod +x scripts/worktree-env"
run $A allow "mv backend/.sqlx backend/.sqlx.bad"
run $A allow "mv $CWD/frontend/a.ts $CWD/frontend/b.ts"
run $A ask   "mv /tmp/a $CWD/frontend/a.ts"
run $A ask   "chmod -R 777 $CWD/.git"
run $A ask   "mv $CWD/backend/.env $CWD/backend/.env.bak"
run $A ask   "mv $CWD/AGENTS.md $OUT"
run $A ask   "cd /etc && mv a b"
# An auto-allowed rename may not carry a path out of the `Read` deny globs.
run $A ask   "mv backend/server.pem backend/server.txt"
run $A none  "cp backend/secrets/token frontend/token.txt"   # cp has no prompt of its own,
                                                            # so what matters is it is not allowed
run $A ask   "mv $CWD/backend/credentials.json /tmp/x"
run $A ask   "cd /tmp/does-not-exist; mv .claude/settings.json settings.bak"
# A segment this hook cannot read whole may carry a redirect, and an earlier write can change
# what a later operand resolves to — neither may ride along on an allow.
run $A none  "cd /tmp >$OUT; mv /tmp/a /tmp/b"
run $A none  "cp -r /tmp/tree /tmp/live; cp /tmp/payload /tmp/live/link"
run $A ask   'mv /tmp/a/`printf ../../etc/x` /tmp/b'
# A sibling checkout is a different root: its files are outside what the Read tool is confined
# to, and copying them in would hand back what that confinement withholds.
EE="$(dirname "$CWD")/windmill-ee-private"   # a sibling checkout; absent elsewhere, still not a root
run $A ask   "mv $EE/backend/x.rs $CWD/backend/x.rs"
run $A none  "cp $EE/README.md $CWD/README.copy"
# Directory form writes a path the command does not name — DEST/basename(SRC) — and `cp`
# follows that child when it is a symlink, as every `*_ee.rs` in this checkout is.
run $A ask   "mv frontend/apps_ee.rs backend/windmill-api/src"
run $A none  "cp frontend/apps_ee.rs backend/windmill-api/src"
run $A none  "cp frontend/a.ts backend"
run $A ask   "mv /tmp/a $CWD/backend"
# ... and a `cd` that fails at runtime may not hide that form: the destination is a directory
# in the directory the command actually ran in, whichever of the two that turns out to be.
run $A none  "cd $CWD/AGENTS.md; cp frontend/apps_ee.rs backend/windmill-api/src"
run $A ask   "cd $CWD/AGENTS.md; mv frontend/apps_ee.rs backend/windmill-api/src"
run $A none  "cd /tmp/x && tar -xzf /tmp/a.tar.gz"   # no -C, and the cwd is now two candidates
run $A allow "cp frontend/a.ts backend/a.ts"  # ... naming the destination proves fine

echo
[ "$fails" = 0 ] && echo "ALL PASS" || { echo "$fails FAILURES"; exit 1; }
