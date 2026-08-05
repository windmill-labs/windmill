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

### Python dependency preparation

Before debugging a Python script, its imports are installed through `windmill prepare-deps`, which
runs `uv` without a database connection. It cannot read the instance settings, so it takes its
registry configuration from the environment of the debug service instead, and the Python server is
handed the resulting venv with `--venv-path`. The install runs in the service rather than in the
session because a private index URL usually embeds credentials and the Python server executes the
debugged script inside its own interpreter, where anything it holds is readable by that script.

Set these on the debug service. Where two names are listed the first wins; a worker reads the
`PIP_*` / `PY_*` names in the same way, except for the index URLs, whose worker env fallbacks are
only `PIP_INDEX_URL` / `PIP_EXTRA_INDEX_URL` (the `PY_*` spellings are accepted here for symmetry
with the other settings):

| Variable | Description | Default |
|----------|-------------|---------|
| `PY_INDEX_URL` / `PIP_INDEX_URL` | Package index (`--index-url`) | PyPI |
| `PY_EXTRA_INDEX_URL` / `PIP_EXTRA_INDEX_URL` | Extra indexes, comma-separated (`--extra-index-url`) | - |
| `PY_TRUSTED_HOST` / `PIP_TRUSTED_HOST` | Hosts to trust, whitespace-separated (`--trusted-host`) | - |
| `PY_INDEX_CERT` / `PIP_INDEX_CERT` | CA bundle for the index, passed to uv as `SSL_CERT_FILE` | - |
| `PY_NATIVE_CERT` / `UV_NATIVE_TLS` | `true` to also trust the platform certificate store (`--native-tls`) | false |
| `UV_INDEX_STRATEGY` | uv index strategy | unsafe-best-match |
| `UV_HTTP_TIMEOUT` | uv HTTP request timeout, in seconds | uv's own default |
| `DAP_PREPARE_DEPS_TIMEOUT_MS` | How long to wait for the install before starting the session without it | 120000 |

When the install fails, the CLI answers `success: false` and carries the installer's stderr in both
`error` and `install_stderr`; the service reports it to the client as an `output` event, so the
reason (unreachable mirror, untrusted certificate, unknown package) reaches the user instead of a
bare `ModuleNotFoundError` at the first import.

Proxy variables (`HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY`, in either case) are forwarded from the
service into each session, since the debugged script needs them for its own outbound calls, exactly
as a job's script does on a worker. When a proxy is set without a bypass list, `NO_PROXY` defaults
to `localhost,127.0.0.1` so calls to `BASE_INTERNAL_URL` are not proxied.

Keeping the settings out of the session's environment only bounds what the debugged script can read
from itself. An unsandboxed session runs under the same user as the service and can still read the
service's environment through `/proc`, the same way a job can read a worker's when the worker runs
unsandboxed. Isolating sessions from the service takes `--nsjail --nsjail-config
nsjail.debug.config.proto`: it is that config's PID namespace and `mount_proc` that put the service
out of reach, not the flag on its own.

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
