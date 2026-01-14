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
