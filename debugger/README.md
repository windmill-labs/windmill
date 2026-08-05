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

The service does not inherit its whole environment into debug subprocesses. Beyond `PATH` and
`HOME`, only the network configuration below is forwarded, so that debugged scripts and dependency
installation (`windmill prepare-deps`, which runs `uv venv` / `uv pip install`) work behind a
proxy, a custom CA, or a private package index.

Reaching the debugged script, matching what the worker gives a regular job:

- `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY` (and their lowercase spellings)
- `SSL_CERT_FILE`, `SSL_CERT_DIR`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`, `NODE_EXTRA_CA_CERTS`

Reaching `prepare-deps` only, since index URLs routinely embed registry credentials and a debug
session runs user-supplied code:

- `PIP_INDEX_URL`, `PY_INDEX_URL`, `PIP_EXTRA_INDEX_URL`, `PY_EXTRA_INDEX_URL`, `PIP_INDEX_CERT`,
  `PY_INDEX_CERT`, `PIP_TRUSTED_HOST`, `PY_TRUSTED_HOST`, `UV_NATIVE_TLS`, `PY_NATIVE_CERT`,
  `UV_HTTP_TIMEOUT`

Keeping that second set out of a debug runtime is why `dap_debug_service.ts` runs `prepare-deps`
itself and passes the Python DAP server only the resulting venv (`--venv-path`). A secret held by
an interpreter that also executes user code is readable from that code no matter where it is
stashed, so the service, which never runs user code, is the only process that holds them.

`prepare-deps` translates these into the spellings uv and bun actually read (`UV_INDEX_URL`,
`UV_EXTRA_INDEX_URL`, `UV_INSECURE_HOST`, `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`) before invoking
them, since uv reads none of pip's variables. The index settings apply to Python only; a private
npm registry for TypeScript sessions is not configurable here. `prepare-deps` runs without a
database, so a `pip_index_url` set under instance settings is invisible to it: for the debugger,
these have to be set in the container's environment.

To install through a TLS-intercepting proxy, uv needs the proxy's CA by one of two routes:

- Point `PY_INDEX_CERT`/`SSL_CERT_FILE` at a bundle containing it. This works on its own, but the
  file *replaces* uv's roots rather than adding to them, so it must be a complete bundle and not
  just the extra CA, or every public index becomes untrusted.
- Or install the CA into the system trust store with the container's `INIT_SCRIPT` (e.g.
  `INIT_SCRIPT=update-ca-certificates`) **and** set `PY_NATIVE_CERT`/`UV_NATIVE_TLS` to `true`. uv
  verifies against its own bundled roots by default and reads the system store only when that is
  set, so the `INIT_SCRIPT` alone is not enough.

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
