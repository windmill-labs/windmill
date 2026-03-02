#!/bin/bash
set -e

# Windmill Extra Services Entrypoint
# Starts LSP, Multiplayer, and Debugger services based on environment variables

# Track PIDs for cleanup
PIDS=()

cleanup() {
    echo "[entrypoint] Shutting down services..."
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    wait
    echo "[entrypoint] All services stopped"
    exit 0
}

trap cleanup SIGTERM SIGINT

# Setup NETRC if provided (for LSP)
if [ -n "$NETRC" ]; then
    echo "$NETRC" > /root/.netrc
    chmod 600 /root/.netrc
fi

# Setup cache directory for LSP
if [ -d /root/.cache ]; then
    export XDG_CACHE_HOME=/root/.cache
    cp -r /pyls/.cache /root/.cache 2>/dev/null || true
fi

# Setup Monaco temp directory for LSP
mkdir -p /tmp/monaco
if [ ! -f /tmp/monaco/go.mod ]; then
    echo -e "module mymod\ngo 1.26" > /tmp/monaco/go.mod
fi

echo "[entrypoint] Starting Windmill Extra Services"
echo "[entrypoint] ENABLE_LSP=${ENABLE_LSP:-true}"
echo "[entrypoint] ENABLE_MULTIPLAYER=${ENABLE_MULTIPLAYER:-true}"
echo "[entrypoint] ENABLE_DEBUGGER=${ENABLE_DEBUGGER:-true}"

# Start LSP service
if [ "${ENABLE_LSP:-true}" = "true" ]; then
    echo "[entrypoint] Starting LSP on port ${LSP_PORT:-3001}..."
    cd /pyls
    PORT=${LSP_PORT:-3001} python3 pyls_launcher.py &
    PIDS+=($!)
    echo "[entrypoint] LSP started (PID: ${PIDS[-1]})"
fi

# Start Multiplayer service (custom y-websocket with logging)
if [ "${ENABLE_MULTIPLAYER:-true}" = "true" ]; then
    echo "[entrypoint] Starting Multiplayer on port ${MULTIPLAYER_PORT:-3002}..."
    cd /multiplayer
    PORT=${MULTIPLAYER_PORT:-3002} HOST=${HOST:-0.0.0.0} node server.mjs &
    PIDS+=($!)
    echo "[entrypoint] Multiplayer started (PID: ${PIDS[-1]})"
fi

# Start Debugger service
if [ "${ENABLE_DEBUGGER:-true}" = "true" ]; then
    echo "[entrypoint] Starting Debugger on port ${DEBUGGER_PORT:-3003}..."
    cd /debugger

    # Build debugger arguments
    DEBUGGER_ARGS="--host ${HOST:-0.0.0.0} --port ${DEBUGGER_PORT:-3003}"
    DEBUGGER_ARGS="$DEBUGGER_ARGS --windmill /usr/local/bin/windmill"

    # Enable nsjail if requested
    if [ "${ENABLE_NSJAIL:-false}" = "true" ]; then
        DEBUGGER_ARGS="$DEBUGGER_ARGS --nsjail --nsjail-config /debugger/nsjail.debug.config.proto"
    fi

    bun run dap_debug_service.ts $DEBUGGER_ARGS &
    PIDS+=($!)
    echo "[entrypoint] Debugger started (PID: ${PIDS[-1]})"
fi

# Check if any services were started
if [ ${#PIDS[@]} -eq 0 ]; then
    echo "[entrypoint] WARNING: No services enabled. Set ENABLE_LSP, ENABLE_MULTIPLAYER, or ENABLE_DEBUGGER to true."
    echo "[entrypoint] Sleeping indefinitely..."
    sleep infinity
fi

echo "[entrypoint] All enabled services started. Waiting..."

# Wait for any process to exit
wait -n "${PIDS[@]}" 2>/dev/null || true

# If one process exits, check which one and report
for i in "${!PIDS[@]}"; do
    if ! kill -0 "${PIDS[$i]}" 2>/dev/null; then
        echo "[entrypoint] Service (PID: ${PIDS[$i]}) has exited"
    fi
done

# Keep running and wait for remaining processes
wait
