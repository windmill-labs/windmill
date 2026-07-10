#!/usr/bin/env bash
# Windmill SSH execution wrapper (bash)
# --------------------------------------------------------------------------
# Runs a self-contained script on a remote host reachable over SSH from the
# Windmill worker, streaming stdout/stderr back live and propagating the
# REMOTE exit code so a failed remote script fails the Windmill job.
#
# This is a userland prototype: it makes NO backend changes. See README.md for
# when to use it (only-a-jump-node reachable, self-contained scripts) and why
# agent workers are the recommended default.
#
# Windmill positional bash arguments:
#   1. ssh_target     resource of type `ssh_target` (Windmill passes it as JSON)
#   2. script_content the body of the script to run on the remote host
#   3. language       interpreter key: bash|sh|python|node|ruby|php|perl
#                     (default: bash; anything else is treated as a raw remote
#                      interpreter command)
#
# Worker requirements: `ssh` client and `jq` must be installed on the worker.

ssh_target="$1"
script_content="$2"
language="${3:-bash}"

set -euo pipefail
umask 077

command -v jq  >/dev/null || { echo "FATAL: jq is required on the worker" >&2; exit 127; }
command -v ssh >/dev/null || { echo "FATAL: ssh client is required on the worker" >&2; exit 127; }

# --- parse the ssh_target resource -----------------------------------------
# `jq -e` exits non-zero when the field is null/absent, so a missing required
# field fails fast with a clear message.
host=$(jq -er '.host'        <<<"$ssh_target") || { echo "FATAL: ssh_target.host missing"        >&2; exit 2; }
user=$(jq -er '.user'        <<<"$ssh_target") || { echo "FATAL: ssh_target.user missing"        >&2; exit 2; }
private_key=$(jq -er '.private_key' <<<"$ssh_target") || { echo "FATAL: ssh_target.private_key missing" >&2; exit 2; }
port=$(jq -r  '.port // 22'  <<<"$ssh_target")
host_pubkey=$(jq -r '.host_pubkey // empty' <<<"$ssh_target")
accept_unknown_host=$(jq -r '.accept_unknown_host // false' <<<"$ssh_target")

# --- interpreter dispatch table --------------------------------------------
# `python3 -u` forces unbuffered output so logs stream live. For chatty bash
# scripts that buffer, wrap the remote interpreter with `stdbuf -oL` (see README).
case "$language" in
  bash)            interp="bash" ;;
  sh)              interp="sh" ;;
  python|python3)  interp="python3 -u" ;;
  node|javascript) interp="node" ;;
  ruby)            interp="ruby" ;;
  php)             interp="php" ;;
  perl)            interp="perl" ;;
  *)               interp="$language" ;;   # passthrough: raw remote interpreter command
esac

# --- private key -> 0600 temp file + job-local known_hosts ------------------
keyfile=$(mktemp "${TMPDIR:-/tmp}/wmssh_key.XXXXXX")
known_hosts=$(mktemp "${TMPDIR:-/tmp}/wmssh_kh.XXXXXX")
cleanup() { rm -f "$keyfile" "$known_hosts"; }
trap cleanup EXIT

printf '%s\n' "$private_key" >"$keyfile"   # trailing newline: some keys require it
chmod 600 "$keyfile"

# --- host-key handling ------------------------------------------------------
ssh_opts=(
  -o BatchMode=yes
  -o ConnectTimeout=15
  -o "UserKnownHostsFile=$known_hosts"
  -p "$port"
  -i "$keyfile"
)
if [ -n "$host_pubkey" ]; then
  # Pin the server key: non-default ports use the `[host]:port` known_hosts form.
  if [ "$port" = "22" ]; then
    printf '%s %s\n'      "$host"        "$host_pubkey" >"$known_hosts"
  else
    printf '[%s]:%s %s\n' "$host" "$port" "$host_pubkey" >"$known_hosts"
  fi
  ssh_opts+=( -o StrictHostKeyChecking=yes )
elif [ "$accept_unknown_host" = "true" ]; then
  echo "WARN: ssh_target.host_pubkey is empty; using TOFU (accept-new) because accept_unknown_host=true. Pin host_pubkey for production." >&2
  ssh_opts+=( -o StrictHostKeyChecking=accept-new )
else
  echo "FATAL: ssh_target.host_pubkey is empty. Pin the host key (ssh-keyscan -t ed25519 $host) or set accept_unknown_host=true to allow TOFU (insecure against MITM)." >&2
  exit 2
fi

# --- remote bootstrap (runs ON the remote host) ----------------------------
# Single-quoted heredoc: NOTHING is expanded locally. $f / $? / $TMPDIR are
# all evaluated remotely. The script body arrives on the remote's stdin.
remote_bootstrap=$(cat <<'REMOTE'
set -u
f=$(mktemp "${TMPDIR:-/tmp}/wmssh_job.XXXXXX") || exit 1
trap 'rm -f "$f"' EXIT          # remote-side cleanup, survives script failure
cat >"$f"                       # read the streamed script body from stdin
@@INTERP@@ "$f"                 # execute with the chosen interpreter
exit $?                         # propagate the remote exit code
REMOTE
)
# NOTE: `interp` is only controlled for the known dispatch keys above — the
# `*)` case passes `language` through verbatim into the remote bootstrap, so
# keep `language` author-controlled, never end-user input (see README).
remote_bootstrap=${remote_bootstrap//@@INTERP@@/$interp}

# --- execute ---------------------------------------------------------------
# - no -t/-tt: keep stdout and stderr separate for clean log capture
#   (add `-tt` ONLY when you need an interactive TTY, e.g. sudo password prompts)
# - body is streamed via stdin so it never touches a local temp file
# - PIPESTATUS[1] is ssh's exit code == the remote script's exit code
set +e
printf '%s\n' "$script_content" | ssh "${ssh_opts[@]}" -- "$user@$host" "$remote_bootstrap"
rc=${PIPESTATUS[1]}
exit "$rc"
