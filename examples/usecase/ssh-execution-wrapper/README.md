SSH execution
=============

Run a self-contained script on a remote host that Windmill cannot place a worker
on, but can reach over SSH (a jump box / utility node).

> ⚠️ **Read this first: prefer agent workers.** For almost every "run code in an
> isolated/segmented environment" need, the recommended answer is an **agent
> worker**. See [When to use what](#when-to-use-what) below.

There are two ways to do this, sharing the same `ssh_target` resource type:

1. **The `#ssh` directive (recommended, enterprise).** Write a *normal* bash
   script and add one line — `#ssh <resource_path>` — at the top. The worker
   reroutes execution to the remote host with **full parity**: typed positional
   args in, structured result out, live streamed logs, cancellation, and the
   remote exit code fails the job. This is a first-class backend feature; see
   [The `#ssh` directive](#the-ssh-directive) below.
2. **The userland wrapper (no license required).** A reusable Windmill script
   (`ssh_exec.sh` / `ssh_exec.py`) that you *call*, passing your remote code as a
   string argument. No backend changes, no license — but you lose the editor
   experience and structured results. Use it when you can't run the enterprise
   image. Documented in [The userland wrapper](#the-userland-wrapper).

What's here
-----------

| File | Purpose |
| --- | --- |
| `ssh_target.resource-type.json` | Resource type: `host`, `port`, `user`, `private_key` (secret), `host_pubkey`, `accept_unknown_host`. Shared by both approaches. |
| `ssh_exec.sh` | The userland wrapper as a Windmill **bash** script. |
| `ssh_exec.py` | Userland wrapper as a Windmill **python** script (interpreter-dispatch table). |

The `#ssh` directive
--------------------

**Enable it (once, as a superadmin):** the feature is enterprise-gated and off by
default. Turn on the `ssh_execution_enabled` instance setting (Superadmin
settings), which requires a valid enterprise license.

**Use it:** create an `ssh_target` resource (see [Setup](#setup)), then write a
bash script with the directive on a leading comment line:

```bash
#ssh f/infra/jump_node
# ^ reroutes this script to run on the host described by the
#   ssh_target resource at f/infra/jump_node

Service="$1"          # typed positional args work as usual

systemctl is-active "$Service"
echo "{\"service\": \"$Service\", \"checked\": true}"   # last stdout line = result
```

The script runs on the remote host exactly as a local bash script would: the
arguments come from the run form, the result is collected the same way
(`result.json` > `result.out` > last stdout line), logs stream live, and a
non-zero remote exit fails the job. Only the *execution location* changes.

**Dynamic target (`#ssh $<arg_name>`):** instead of hardcoding a path, the
directive can name a job argument that supplies the target at call time — for
picking the host from the run form, or fanning out over hosts in a flow forloop:

```bash
#ssh $jump_host

target="$1"           # jump_host's position: always received as an empty string
df -h
```

The argument must be an `ssh_target` resource **path string** (with or without
the `$res:` prefix) — inline `ssh_target` objects are rejected, so the target is
always resolved through the runner's resource permissions and a caller can only
route execution to hosts whose resource they can read. Two things to note: the
target argument itself is forwarded to the remote script as an **empty string**
(its resolved value embeds the private key, which must never reach the remote
command line — its position is kept so the other `$1..$n` stay aligned), and
with a dynamic target the *runner* chooses where the code executes (bounded by
those resource permissions), whereas a hardcoded path lets the script author
pin it.

**Host-key pinning** is enforced (`StrictHostKeyChecking=yes`) whenever the
resource's `host_pubkey` is set. An empty `host_pubkey` refuses to run unless the
resource explicitly sets `accept_unknown_host: true`, which falls back to weaker
TOFU (`accept-new`) and logs a warning — development only.

**Parity boundary:** the remote receives the script body and its positional args
only. The Windmill runtime is *not* forwarded — `BASE_INTERNAL_URL`, the `wmill`
client, and reserved `WM_*` variables are unavailable remotely, so in-script
Windmill API callbacks won't work. The same trade-offs as the wrapper apply:
no remote dependency management, no nsjail sandbox, no S3 cache, per-job SSH
overhead. v1 is bash-only.

The userland wrapper
--------------------

The wrapper takes an `ssh_target` resource, a `script_content` string, and a
`language`, then:

1. writes the private key to a `0600` temp file (and a job-local `known_hosts`),
2. opens a single SSH connection (no TTY),
3. streams the script body to the remote host's stdin, where a small bootstrap
   `mktemp`s a file, `trap`s its removal on `EXIT`, runs it with the right
   interpreter, and exits with the script's exit code,
4. streams stdout/stderr back live and **propagates the remote exit code** so a
   failed remote script fails the Windmill job.

```
Windmill worker                         Remote jump node
┌────────────────────┐                  ┌─────────────────────────────┐
│ ssh_exec.sh        │   ssh (no -t)    │ sh -c <bootstrap>           │
│  key → 0600 tmp    │ ───────────────▶ │   f=$(mktemp)               │
│  known_hosts pin   │   body on stdin  │   trap 'rm -f $f' EXIT      │
│  printf body | ssh │ ────────────────▶│   cat > $f                  │
│                    │ ◀─────────────── │   <interp> $f   (live logs) │
│  exit = ssh rc     │   remote rc      │   exit $?                   │
└────────────────────┘                  └─────────────────────────────┘
```

Setup
-----

1. **Create the resource type.** Push it with the CLI:

   ```bash
   wmill resource-type push ssh_target.resource-type.json
   ```

   or recreate it in the UI (Resources → Resource Types) with the same schema.
   `private_key` is marked secret (`"password": true`); `host_pubkey` is optional.

2. **Create an `ssh_target` resource** for your jump node. Get `host_pubkey` from
   the server (the `keytype key` portion, comment optional):

   ```bash
   ssh-keyscan -t ed25519 your.jump.host        # → ssh-ed25519 AAAAC3Nz...
   ```

3. **Create a script** from `ssh_exec.sh` (bash) or `ssh_exec.py` (python). Mark
   the first argument as a resource of type `ssh_target`.

Usage
-----

Call the wrapper with the target, the remote script body, and its language:

```jsonc
{
  "ssh_target": "$res:u/me/my_jump_node",
  "script_content": "set -euo pipefail\ndf -h\nsystemctl is-active nginx",
  "language": "bash"
}
```

```jsonc
{
  "ssh_target": "$res:u/me/my_jump_node",
  "script_content": "import platform\nprint(platform.platform())",
  "language": "python"
}
```

Supported `language` keys: `bash`, `sh`, `python`/`python3`, `node`/`javascript`,
`ruby`, `php`, `perl`. Any other value is passed through as a raw remote
interpreter command. The remote host must already have that interpreter and any
dependencies installed (see tradeoffs).

Design notes (the details that make or break it)
------------------------------------------------

These are deliberate and worth preserving if you adapt the wrapper:

- **Exit-code propagation.** `ssh host cmd` returns the *remote* exit code. The
  bash wrapper reads it via `${PIPESTATUS[1]}` and re-`exit`s it; the python
  wrapper raises on non-zero. A failed remote script fails the Windmill job.
- **No TTY.** We never pass `-t`/`-tt`. A TTY merges stdout and stderr and
  mangles log capture. Enable `-tt` **only** for interactive remote prompts
  (e.g. `sudo` asking for a password).
- **Live, unbuffered logs.** `python -u` is used for python; for chatty bash that
  buffers when piped, wrap the remote interpreter with `stdbuf -oL` (edit the
  dispatch table, e.g. `interp="stdbuf -oL bash"`).
- **Remote cleanup survives failure.** The `trap 'rm -f "$f"' EXIT` is set on the
  **remote** side, inside the streamed bootstrap, so the temp file is removed
  even if the script errors out.
- **Host-key pinning.** With `host_pubkey` set, the wrapper pins it into a
  job-local `known_hosts` and enforces `StrictHostKeyChecking=yes` (non-default
  ports use the `[host]:port` form). With it empty, the wrapper refuses to run
  unless `accept_unknown_host: true` is set on the resource, which falls back to
  `accept-new` (TOFU) and warns — weaker; pin in production.
- **Quoted heredoc.** The remote bootstrap is built with `<<'REMOTE'` so `$f`,
  `$?`, `$TMPDIR` are evaluated *remotely*, not expanded on the worker.
- **Body via stdin.** The script body is streamed over stdin, never written to a
  local temp file or interpolated into the command line.
- **`--` before the destination.** OpenSSH parses a destination starting with
  `-` as an option, so without the separator a crafted `user` like
  `-oProxyCommand=...` in the resource would execute a local command on the
  worker before host-key validation. Keep the `--` if you adapt the wrapper.
- **Multiple round-trips?** This wrapper makes a single SSH connection. If you
  extend it to several `ssh` calls, add
  `-o ControlMaster=auto -o ControlPersist=60 -o ControlPath=<job-local>` to
  reuse one connection instead of re-authenticating each time.

When to use what
----------------

**Default: agent workers.** A Windmill *agent worker* is a lightweight worker
that runs *inside* the target environment and connects back to the Windmill
server over **outbound HTTP only** (using an `jwt_agent_*` token) — no inbound
ports, no DB access. It keeps everything Windmill workers normally give you:
automatic dependency management, nsjail sandboxing, the S3 binary cache, native
secrets, all languages, and no per-job connection overhead. If you can run a
process in the target environment, use an agent worker.

**Worker-group tags** are the right tool when you *can* place a full worker in
the environment and want to route specific scripts to it.

**This SSH wrapper** is for the narrow case where **both** are true:

- you can only reach a **jump/utility node** over SSH (you cannot place any
  worker or agent process there), and
- the scripts are **simple and self-contained** (no Windmill-managed deps).

What you lose with the SSH path
-------------------------------

- **No dependency management.** The remote host must already have the interpreter
  *and* every library/tool the script uses. Nothing is installed or locked.
- **No nsjail sandboxing.** The script runs as the SSH user with that user's full
  privileges. The jump node becomes a high-value target — scope the key and user
  tightly.
- **No S3 / binary cache.** No shared cache of dependencies or artifacts.
- **Per-job SSH overhead.** Each run pays connection + auth latency (mitigable
  with ControlMaster only if you make multiple round-trips).
- **No native Windmill integrations on the remote side** — no resource/variable
  injection, no `wmill` client, no flow step context beyond what you pass in.

Limitations of this prototype
-----------------------------

- Requires an `ssh` client (and `jq` for the bash variant) on the worker.
- Assumes a self-contained, non-interactive script. No stdin is forwarded to the
  remote script (stdin carries the script body).
- Unknown `language` values are passed through verbatim as the remote
  interpreter — keep `language` author-controlled, not end-user input.

Tested
------

Both wrappers were exercised against a local `sshd`: success path, remote
exit-code propagation (bash `${PIPESTATUS[1]}`, python raises), clean
stdout/stderr separation, `python -u` interpreter dispatch, host-key pinning
rejecting a wrong key (script never runs), the TOFU opt-in
(`accept_unknown_host: true`) and the refusal without it, and confirmed remote
*and* local temp-file cleanup.
