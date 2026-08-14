#!/usr/bin/env bash
# Decision table for the two scratch-dir PreToolUse guards. Run: bash .claude/hooks/test-hooks.sh
#
# What this pins is the `ask` column: a matcher change that turns one into a no-decision drops
# that command's only prompt (see lib-guarded-verb.sh). The wrapper, nested-command and quoted
# rows are the ones that catch it.
set -uo pipefail
H="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
CWD="$(git -C "$H" rev-parse --show-toplevel)"
OUT="$HOME/not-a-git-tree"   # never written to; only the guards' path checks look at it
fails=0

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
run $G ask   "rm -rf $CWD"
run $G ask   "rm -rf $CWD/*"
run $G ask   "rm -rf /etc/passwd"
run $G ask   'rm -rf "$HOME/x"'
run $G ask   "rm -rf /tmp/../$OUT"
run $G ask   "ls /tmp && rm -rf /tmp/x"
run $G ask   'echo $(rm -rf /etc)'
run $G ask   'echo `rm -rf /etc`'
run $G ask   "{ rm -rf /etc; }"
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

echo
echo "== allow-fileops-in-tmp.sh =="
A=allow-fileops-in-tmp.sh
run $A allow "mv /tmp/a /tmp/b"
run $A allow "chmod 755 /tmp/a"
run $A allow "cp -r /tmp/a /tmp/b"
run $A allow "tar -xzf /tmp/a.tar.gz -C /tmp/out"
run $A ask   "mv /tmp/a $OUT"
run $A ask   "mv $CWD/AGENTS.md /tmp/a"
run $A ask   "chmod -R 777 $CWD"
run $A ask   "ls && mv /tmp/a /tmp/b"
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

echo
[ "$fails" = 0 ] && echo "ALL PASS" || { echo "$fails FAILURES"; exit 1; }
