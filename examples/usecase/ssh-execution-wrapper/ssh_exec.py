# Windmill SSH execution wrapper (python)
# --------------------------------------------------------------------------
# Python variant of ssh_exec.sh. Same contract: run a self-contained script on
# a remote host over SSH, stream stdout/stderr live, and fail the Windmill job
# when the remote script fails. Userland prototype, no backend changes.
#
# Arguments:
#   ssh_target      resource of type `ssh_target` (received as a dict)
#   script_content  the body of the script to run on the remote host
#   language        interpreter key: bash|sh|python|node|ruby|php|perl
#                   (default: bash; anything else is treated as a raw remote
#                    interpreter command)
#
# Worker requirements: an `ssh` client installed on the worker.

import os
import subprocess
import sys
import tempfile

# `python3 -u` forces unbuffered output so logs stream live.
INTERPRETERS = {
    "bash": "bash",
    "sh": "sh",
    "python": "python3 -u",
    "python3": "python3 -u",
    "node": "node",
    "javascript": "node",
    "ruby": "ruby",
    "php": "php",
    "perl": "perl",
}

# Single-quoted (raw) so $f / $? / $TMPDIR are evaluated remotely, not here.
# @@INTERP@@ is replaced with the chosen interpreter before sending.
REMOTE_BOOTSTRAP = (
    "set -u\n"
    'f=$(mktemp "${TMPDIR:-/tmp}/wmssh_job.XXXXXX") || exit 1\n'
    "trap 'rm -f \"$f\"' EXIT\n"   # remote-side cleanup, survives script failure
    'cat >"$f"\n'                   # read the streamed script body from stdin
    '@@INTERP@@ "$f"\n'             # execute with the chosen interpreter
    "exit $?\n"                     # propagate the remote exit code
)


def main(ssh_target: dict, script_content: str, language: str = "bash"):
    host = ssh_target["host"]
    user = ssh_target["user"]
    private_key = ssh_target["private_key"]
    port = str(ssh_target.get("port") or 22)
    host_pubkey = (ssh_target.get("host_pubkey") or "").strip()
    accept_unknown_host = bool(ssh_target.get("accept_unknown_host"))

    interp = INTERPRETERS.get(language, language)  # passthrough for unknown keys

    # 0600 temp files for the key and a job-local known_hosts.
    keyfile = tempfile.NamedTemporaryFile("w", delete=False)
    known_hosts = tempfile.NamedTemporaryFile("w", delete=False)
    try:
        keyfile.write(private_key.rstrip("\n") + "\n")  # trailing newline required by some keys
        keyfile.close()
        os.chmod(keyfile.name, 0o600)

        ssh_opts = [
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=15",
            "-o", f"UserKnownHostsFile={known_hosts.name}",
            "-p", port,
            "-i", keyfile.name,
        ]

        if host_pubkey:
            # Pin the server key; non-default ports use the [host]:port form.
            entry = f"{host} {host_pubkey}" if port == "22" else f"[{host}]:{port} {host_pubkey}"
            known_hosts.write(entry + "\n")
            known_hosts.close()
            ssh_opts += ["-o", "StrictHostKeyChecking=yes"]
        elif accept_unknown_host:
            known_hosts.close()
            print(
                "WARN: ssh_target.host_pubkey is empty; using TOFU (accept-new) "
                "because accept_unknown_host=true. Pin host_pubkey for production.",
                file=sys.stderr,
                flush=True,
            )
            ssh_opts += ["-o", "StrictHostKeyChecking=accept-new"]
        else:
            known_hosts.close()
            raise ValueError(
                "ssh_target.host_pubkey is empty. Pin the host key "
                f"(ssh-keyscan -t ed25519 {host}) or set accept_unknown_host=true "
                "to allow TOFU (insecure against MITM)."
            )

        remote = REMOTE_BOOTSTRAP.replace("@@INTERP@@", interp)
        # no -t/-tt: stdout and stderr stay separate for clean log capture.
        # `--` so a crafted user/host (e.g. "-oProxyCommand=...") can never be
        # parsed as an ssh option
        cmd = ["ssh", *ssh_opts, "--", f"{user}@{host}", remote]

        # stdout/stderr are inherited from this process so they stream live to
        # the Windmill job log; only stdin is a pipe for the script body.
        proc = subprocess.Popen(cmd, stdin=subprocess.PIPE)
        proc.communicate(input=(script_content + "\n").encode())
        rc = proc.returncode
    finally:
        for path in (keyfile.name, known_hosts.name):
            try:
                os.unlink(path)
            except OSError:
                pass

    # Raise on non-zero so the Windmill job fails with the remote exit code.
    if rc != 0:
        raise RuntimeError(f"Remote script exited with code {rc}")
    return {"ok": True, "exit_code": 0, "host": host}
