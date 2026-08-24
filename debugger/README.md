# Windmill Debug Module

A DAP (Debug Adapter Protocol) implementation for debugging Python and TypeScript/Bun scripts in Windmill's Monaco editor.

## Overview

This module provides step-through debugging capabilities with breakpoints, variable inspection, and stack traces. It uses WebSocket communication between the Monaco editor frontend and language-specific debug backends.

## Supported Languages

- **Python** - Uses a bdb-based debugger via `dap_websocket_server.py`
- **TypeScript/Bun** - Uses V8 Inspector Protocol via `dap_websocket_server_bun.ts`

## Architecture

```
┌─────────────────────┐     WebSocket      ┌──────────────────────────┐
│  Monaco Editor      │◄──────────────────►│  DAP Debug Service       │
│  (dapClient.ts)     │    DAP Protocol    │  (dap_debug_service.ts)  │
└─────────────────────┘                    └──────────┬───────────────┘
                                                      │
                                           ┌──────────┴───────────┐
                                           │                      │
                                    ┌──────▼──────┐       ┌───────▼───────┐
                                    │   Python    │       │   Bun/TS      │
                                    │   Debugger  │       │   Debugger    │
                                    └─────────────┘       └───────────────┘
```

## Files

| File | Description |
|------|-------------|
| `dap_debug_service.ts` | Unified WebSocket server that routes to Python or Bun debuggers |
| `dap_websocket_server.py` | Python debugger backend (bdb-based) |
| `dap_websocket_server_bun.ts` | Bun/TypeScript debugger backend (V8 Inspector) |
| `dapClient.ts` | Client-side DAP WebSocket client with Svelte store |
| `MonacoDebugger.svelte` | Monaco editor integration component |
| `DebugToolbar.svelte` | Debug control buttons (step, continue, etc.) |
| `DebugPanel.svelte` | Variables and stack trace display panel |
| `index.ts` | Module exports |

## Usage

### Starting the Debug Service

```bash
bun run debug/dap_debug_service.ts
```

Options:
- `--port PORT` - Server port (default: 3003)
- `--host HOST` - Server host (default: 0.0.0.0)
- `--python-path PATH` - Python binary path (default: python3)
- `--bun-path PATH` - Bun binary path (default: bun)
- `--nsjail` - Enable nsjail sandboxing for debugger processes
- `--nsjail-config PATH` - Path to nsjail config file
- `--nsjail-path PATH` - Path to nsjail binary (default: nsjail)

### Endpoints

- `/python` - Python debugging
- `/typescript` - TypeScript/Bun debugging
- `/bun` - Alias for `/typescript`

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DAP_PORT` | Server port | 3003 |
| `DAP_HOST` | Server host | 0.0.0.0 |
| `DAP_PYTHON_PATH` | Python binary path | python3 |
| `DAP_BUN_PATH` | Bun binary path | bun |
| `DAP_NSJAIL_ENABLED` | Enable nsjail sandboxing | false |
| `DAP_NSJAIL_PATH` | nsjail binary path | nsjail |
| `DAP_NSJAIL_CONFIG` | nsjail config file path | - |

### Dependency preparation

Before debugging a script, its imports are installed through `windmill prepare-deps`, which runs
`uv` (Python) or `bun install` (TypeScript) without a database connection. The install runs in the
service rather than in the session because the registry configuration usually embeds credentials
and a debug server executes the submitted script inside a process the script can read; the Python
server is handed only the resulting venv, with `--venv-path`, and a Bun session only the resulting
`node_modules`.

`DAP_PREPARE_DEPS_TIMEOUT_MS` bounds the install (default 120000); past it the session starts
without its dependencies. When the install fails, the CLI answers `success: false` and carries the
installer's stderr in both `error` and `install_stderr`; the service reports it to the client as an
`output` event, so the reason (unreachable mirror, untrusted certificate, unknown package) reaches
the user instead of a bare `ModuleNotFoundError` at the first import.

### Registry configuration

Because `prepare-deps` has no database, the service reads the instance settings for it from
`GET /api/debug/registry_config` on `WINDMILL_BASE_URL` and passes them down over the CLI's stdin
request. It is authorized by the launch token of the session being started, and serves only the
settings that session's own installer runs on, so a TypeScript session's token cannot be used to
read the Python index credentials.

The token also reaches the browser, so what it can fetch is what a workspace member can fetch.
Sessions started by an operator are refused outright, since an operator cannot run a preview job
either; for a member who can, the npm settings are already exposed by a preview (a worker leaves
the same `.npmrc` / `bunfig.toml` in the directory the previewed script runs in), while the Python
index URL, which otherwise only appears as uv's argv, becomes readable where it was not before.

These settings are Enterprise-only, exactly as they are for jobs, and a CE instance reports that in
the session's output rather than applying them:

| Setting | Applies to |
|---------|------------|
| `npm_config_registry` | `bun install` registry and its `:_authToken=` |
| `npmrc` | written verbatim as `.npmrc`, taking precedence over `npm_config_registry` |
| `bunfig_install_scopes` | `[install.scopes]` in the generated `bunfig.toml` |
| `pip_index_url` | `uv --index-url` |
| `pip_extra_index_url` | `uv --extra-index-url`, comma-separated |

`uv_index_strategy` is served on any edition, like it is to a worker. An index URL holding the
`EPHEMERAL_TOKEN` placeholder is not served at all: only a worker can run the command that
substitutes it.

The credential-bearing files (`.npmrc`, `bunfig.toml`) are written under
`/var/tmp/windmill-debug-registry`, not into the directory the install runs in, and are deleted
when the install ends. That directory is not private to the install: a session resolves its
`node_modules` symlink back into it, and `nsjail.debug.config.proto` bind-mounts the whole of
`/tmp` into every session, so credentials left there would be readable by a concurrent session.
`/var/tmp` is a tmpfs in that same config, one instance per jail, so a session sees an empty one
and a jailed install's credentials go away with the jail even when it is killed (the service kills
an install with SIGKILL, which no cleanup in the installer can survive). An install running
unjailed writes to the host's `/var/tmp` instead, where a directory a kill left behind is removed
by the next install; a session running unjailed is unconfined anyway and sees the whole filesystem,
as it already does the rest of the service's state.

The rest of the registry configuration has no instance setting and is read from the environment of
the debug service. Where two names are listed the first wins; a worker reads the same names:

| Variable | Description | Default |
|----------|-------------|---------|
| `PY_TRUSTED_HOST` / `PIP_TRUSTED_HOST` | Hosts to trust, whitespace-separated (`--trusted-host`) | - |
| `PY_INDEX_CERT` / `PIP_INDEX_CERT` | CA bundle for the index, passed to uv as `SSL_CERT_FILE`. Falls back to `SSL_CERT_FILE`, then `REQUESTS_CA_BUNDLE`, then `CURL_CA_BUNDLE`, so a host that configures its CA under any of those names is picked up. Whichever is used **replaces** uv's own roots rather than adding to them, so it has to be a complete bundle: one holding only a private CA leaves every public index untrusted. `bun install` gets the same bundle as `NODE_EXTRA_CA_CERTS`, the only spelling Bun reads | - |
| `SSL_CERT_DIR` | Directory of certificates, forwarded to uv as-is. Replaces uv's roots the same way the bundle does, so a directory holding only a private CA leaves public indexes untrusted | - |
| `PY_NATIVE_CERT` / `UV_NATIVE_TLS` | `true` to also trust the platform certificate store (`--native-tls`) | false |
| `UV_HTTP_TIMEOUT` | uv HTTP request timeout, in seconds | uv's own default |
| `DAP_REGISTRY_CONFIG_TIMEOUT_MS` | How long to wait on the settings fetch before installing without it | 10000 |

`PY_INDEX_URL` / `PIP_INDEX_URL` and `PY_EXTRA_INDEX_URL` / `PIP_EXTRA_INDEX_URL`, along with
`UV_INDEX_STRATEGY`, are still read from the same environment whenever the fetch yields no index:
because the instance has none set, because this is a CE instance, or because the session was not
allowed the settings. A Python debug service configured that way therefore keeps working, but setting
them is an instance-wide decision to install Python dependencies from that index, independent of who
opened the session; leave them unset to let the instance settings alone decide. The npm settings have
no such fallback: the instance settings are the only source.

Proxy variables (`HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY`, in either case) are forwarded from the
service into each session, since the debugged script needs them for its own outbound calls, exactly
as a job's script does on a worker. When a proxy is set without a bypass list, `NO_PROXY` defaults
to `localhost,127.0.0.1` so calls to `BASE_INTERNAL_URL` are not proxied.

Trust roots are forwarded alongside them: `SSL_CERT_FILE`, `SSL_CERT_DIR`, `REQUESTS_CA_BUNDLE`,
`CURL_CA_BUNDLE` and `NODE_EXTRA_CA_CERTS`. Behind a TLS-intercepting proxy these are what let the
debugged script's own HTTPS calls verify, and installing the CA in the container's system store is
not enough on its own, since `requests` carries its own bundle and Node reads only
`NODE_EXTRA_CA_CERTS`. Registry settings are deliberately not forwarded: they carry credentials and
only the service needs them.

Registering that CA in the container's system store happens on its own: mount it into
`/usr/local/share/ca-certificates/` **named `*.crt`**, the only extension `update-ca-certificates`
reads, and `windmill_extra` runs it before starting any service. `RUN_UPDATE_CA_CERTIFICATE_AT_START=true` forces the same thing whether or not
certificates are mounted there, and `RUN_UPDATE_CA_CERTIFICATE_PATH` overrides the tool, matching
the server and worker. Both are best-effort: a UID that cannot write `/etc/ssl/certs` logs a warning
and the container still boots. `INIT_SCRIPT` remains the hook for anything more involved, and unlike
the CA update it aborts startup when it fails.

Note what the system store does *not* cover, which is most of what a debug session installs with:
uv trusts its own bundled roots unless `PY_NATIVE_CERT`/`UV_NATIVE_TLS` is `true`, Bun and Node read
only `NODE_EXTRA_CA_CERTS`, and `requests` carries certifi. Registering the CA fixes Python's stdlib
`ssl`, `curl` and `git`; the rest still needs the variables above.

Keeping the settings out of the session's environment only bounds what the debugged script can read
from itself. An unsandboxed session runs under the same user as the service and can still read the
service's environment through `/proc`, the same way a job can read a worker's when the worker runs
unsandboxed. Isolating sessions from the service takes `--nsjail --nsjail-config
nsjail.debug.config.proto`: it is that config's PID namespace and `mount_proc` that put the service
out of reach, not the flag on its own.

The installer is jailed on the same terms, in both languages: `uv pip install` builds source
distributions and `bun install` runs postinstall scripts, so a package's own code executes there
too. It keeps the service's environment across that boundary — the config sets `keep_env`, which is
how the settings above reach it — so replacing that with an allowlist would have to carry the
registry and CA variables in explicitly. It also runs in its own process group, because `uv` and
`bun` are grandchildren: signalling only the installer reparents them to init and they keep
downloading, which would make the timeout and the cancel-on-disconnect half-measures.

### Frontend Integration

```svelte
<script>
  import { MonacoDebugger } from './debug'
  let editor // Monaco editor instance
  let code = 'print("Hello")'
</script>

<MonacoDebugger {editor} {code} language="python3" />
```

## Testing

```bash
# Test Python debugger
bun run debug/test_dap_server.py

# Test Bun debugger
bun run debug/test_dap_server_bun.ts

# Test unified service
bun run debug/test_debug_service.ts
```
