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

# An arbitrary non-root UID gets HOME=/ and cannot write the image's 0700 /root, so
# redirect $HOME before anything writes under it (netrc below, plus the bun/npm/go
# caches in the services). Keep the fallback UID-scoped: a leftover dir from a
# different UID on a shared /tmp is not writable. Root keeps HOME=/root.
HOME="${HOME:-/root}"
if [ ! -w "$HOME" ]; then
    echo "[entrypoint] HOME=$HOME is not writable for UID $(id -u), using HOME=/tmp/windmill-home-$(id -u)"
    HOME="/tmp/windmill-home-$(id -u)"
    mkdir -p "$HOME"
fi
export HOME

# Register CA certificates mounted into the image before anything opens a TLS connection.
# Best-effort on purpose, unlike INIT_SCRIPT below: a non-root UID cannot write /etc/ssl/certs, and
# a deployment that never needed a custom CA must still boot. Env var names and the default-off
# behavior match the server/worker binary, so one setting covers every container. What the system
# trust store does and does not reach is documented in debugger/README.md.
CA_CERT_DIR=/usr/local/share/ca-certificates

update_ca_certificates() {
    local reason="$1"
    local tool="${RUN_UPDATE_CA_CERTIFICATE_PATH:-/usr/sbin/update-ca-certificates}"
    local output
    if [ ! -x "$tool" ]; then
        echo "[entrypoint] $reason but $tool is not executable, skipping CA update"
        return
    fi
    echo "[entrypoint] $reason, running $tool"
    if output=$("$tool" 2>&1); then
        echo "[entrypoint] CA certificates updated"
    else
        # Carry the tool's own message: the usual cause is an unwritable /etc/ssl/certs under a
        # non-root UID, but guessing that in place of the real error hides everything else.
        echo "[entrypoint] WARNING: $tool failed (UID $(id -u)): ${output:-no output}; continuing" >&2
    fi
}

if [ "$(echo "${RUN_UPDATE_CA_CERTIFICATE_AT_START:-false}" | tr '[:upper:]' '[:lower:]')" = "true" ]; then
    update_ca_certificates "RUN_UPDATE_CA_CERTIFICATE_AT_START=true"
elif [ -n "$(find -L "$CA_CERT_DIR" -type f -name '*.crt' -print -quit 2>/dev/null)" ]; then
    # Certificates mounted there are unambiguous intent, and they do nothing until registered, so
    # take the same action without making the operator also find the env var.
    update_ca_certificates "Found certificates in $CA_CERT_DIR"
elif [ -n "$(ls -A "$CA_CERT_DIR" 2>/dev/null)" ]; then
    # Reporting success over a mount update-ca-certificates ignores would be worse than saying
    # nothing: .pem is the spelling people reach for, and only .crt is read.
    echo "[entrypoint] WARNING: $CA_CERT_DIR has files but none named *.crt, the only extension" \
        "update-ca-certificates reads; they will be ignored" >&2
fi

# INIT_SCRIPT is the documented hook for preparing the host before anything reaches the network
# (CA certificates, proxies, mounts), matching the worker's INIT_SCRIPT. It must therefore complete
# before any service starts, and a failure has to abort: services that come up with an unprepared
# trust store fail every TLS handshake instead, which is far harder to diagnose.
if [ -n "$INIT_SCRIPT" ]; then
    echo "[entrypoint] Running INIT_SCRIPT..."
    bash -c "$INIT_SCRIPT" || {
        code=$?
        echo "[entrypoint] ERROR: INIT_SCRIPT failed with exit code $code, aborting" >&2
        exit "$code"
    }
    echo "[entrypoint] INIT_SCRIPT completed"
fi

# Setup NETRC if provided (for LSP)
if [ -n "$NETRC" ]; then
    echo "$NETRC" > "$HOME/.netrc"
    chmod 600 "$HOME/.netrc"
fi

# Setup cache directory for LSP (falls back to the image's world-writable
# XDG_CACHE_HOME=/pyls/.cache when $HOME/.cache isn't mounted)
if [ -d "$HOME/.cache" ]; then
    export XDG_CACHE_HOME="$HOME/.cache"
    cp -r /pyls/.cache "$HOME/.cache" 2>/dev/null || true
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
echo "[entrypoint] ENABLE_GATEWAY=${ENABLE_GATEWAY:-true}"

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

# Start Gateway reverse proxy (routes /ws/*, /ws_mp/*, /ws_debug/* to the right service)
if [ "${ENABLE_GATEWAY:-true}" = "true" ]; then
    echo "[entrypoint] Starting Gateway on port ${GATEWAY_PORT:-3000}..."
    cd /multiplayer
    PORT=${GATEWAY_PORT:-3000} node gateway.mjs &
    PIDS+=($!)
    echo "[entrypoint] Gateway started (PID: ${PIDS[-1]})"
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
