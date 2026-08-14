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
  if [ "$got" = "$want" ]; then
    printf '  ok   %-5s %s\n' "$got" "$cmd"
  else
    printf 'FAIL   want=%-5s got=%-5s %s\n       %s\n' "$want" "$got" "$cmd" "$out"
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
run $A none  "cp $CWD/AGENTS.md /tmp/a"
run $A none  "tar -xzf /tmp/a.tar.gz -C $OUT"
run $A none  "cargo build"

echo
[ "$fails" = 0 ] && echo "ALL PASS" || { echo "$fails FAILURES"; exit 1; }
