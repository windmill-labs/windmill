#!/usr/bin/env bash
#
# Supervision tests for docker/entrypoint-extra.sh.
#
# The real script is run unmodified inside a container, with the four services it
# starts replaced by stubs, so what is under test is the shipped file and the
# shipped bash. The stubs are recognised by the argv the entrypoint uses
# (pyls_launcher.py, server.mjs, dap_debug_service.ts, gateway.mjs), which is
# also what keeps the test honest: change how a service is started and the
# corresponding case here stops matching.
#
# Usage:
#   bash docker/test_entrypoint_extra.sh
#
# The image defaults to the base of DockerfileExtra's chain (debian:trixie-slim,
# via windmill-ee-slim), so the bash built-ins behave as they do in the real
# image. Override with ENTRYPOINT_TEST_IMAGE to run it against another one, e.g.
#   ENTRYPOINT_TEST_IMAGE=windmill-extra:test bash docker/test_entrypoint_extra.sh

set -euo pipefail

IMAGE="${ENTRYPOINT_TEST_IMAGE:-debian:trixie-slim}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENTRYPOINT="$SCRIPT_DIR/entrypoint-extra.sh"

if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required to run this test" >&2
    exit 1
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# --- the pieces that run inside the container -----------------------------

cat > "$WORKDIR/stub" <<'STUB'
#!/bin/bash
# Stands in for python3 / node / bun. Works out which service it was started as
# from its arguments, then either stays up until it is signalled (recording that
# it was stopped) or exits straight away, to play the crashed service.
name=unknown
for arg in "$@"; do
    case "$arg" in
        pyls_launcher.py) name=lsp ;;
        server.mjs) name=multiplayer ;;
        dap_debug_service.ts) name=debugger ;;
        gateway.mjs) name=gateway ;;
    esac
done
echo "stub[$name] started"

if [ "$name" = "${STUB_IGNORE_TERM:-}" ]; then
    # Plays a service that is wedged or slow to honour SIGTERM.
    trap 'echo "stub[$name] ignoring SIGTERM"' TERM
else
    trap 'echo "stub[$name] got SIGTERM"; touch "/out/$name.stopped"; exit 0' TERM
fi
touch "/out/$name.ready"

if [ "$name" = "${STUB_DIE_AS:-}" ]; then
    # Only die once every stub in $STUB_WAIT_FOR has its trap installed. Without
    # this the entrypoint can reach its shutdown while a later stub is still
    # starting, and that stub takes SIGTERM's default action instead of running
    # its handler — a race in the test, not in the entrypoint.
    for _ in $(seq 1 600); do
        ready=1
        for other in ${STUB_WAIT_FOR:-}; do
            if [ ! -f "/out/$other.ready" ]; then ready=0; fi
        done
        if [ "$ready" -eq 1 ]; then break; fi
        sleep 0.1
    done
    echo "stub[$name] exiting with ${STUB_DIE_CODE:-1}"
    exit "${STUB_DIE_CODE:-1}"
fi
# `sleep & wait` rather than a bare `sleep`, so the trap runs as soon as the
# signal arrives instead of after the sleep returns. The loop matters: bash
# resumes after an interrupted `wait`, so without it even the trap that only
# logs would fall off the end of the script and exit.
while true; do
    sleep 3000 &
    wait $!
done
STUB

cat > "$WORKDIR/harness.sh" <<'HARNESS'
#!/bin/bash
# Runs inside the container: installs the stubs, then drives the real
# /entrypoint.sh through each scenario.
set -uo pipefail

mkdir -p /pyls /multiplayer /debugger /out
for tool in python3 node bun; do
    cp /harness/stub "/usr/local/bin/$tool"
    chmod +x "/usr/local/bin/$tool"
done

export ENABLE_LSP=true ENABLE_MULTIPLAYER=true ENABLE_DEBUGGER=true ENABLE_GATEWAY=true
# Every stub the dying one must see started before it exits.
export STUB_WAIT_FOR="lsp multiplayer debugger gateway"

failures=0
check() {
    local what="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        echo "  ok: $what is $expected"
    else
        echo "  FAIL: $what: expected $expected, got $actual"
        failures=$((failures + 1))
    fi
}

check_log() {
    local what="$1" pattern="$2"
    if grep -qE "$pattern" /out/log; then
        echo "  ok: $what"
    else
        echo "  FAIL: $what (no line matching /$pattern/)"
        failures=$((failures + 1))
    fi
}

stopped() {
    [ -f "/out/$1.stopped" ] && echo yes || echo no
}

# `wait` on a backgrounded entrypoint, with a watchdog that SIGKILLs it rather
# than letting a wedged shutdown block the run. Sets $wait_status.
wait_bounded() {
    local pid="$1" secs="${2:-30}" watchdog
    (sleep "$secs"; kill -KILL "$pid" 2>/dev/null) &
    watchdog=$!
    wait "$pid"
    wait_status=$?
    kill "$watchdog" 2>/dev/null
    wait "$watchdog" 2>/dev/null
}

# Block until the entrypoint has written $1, so no scenario acts on a script that
# has not reached the state it is about to be tested in.
await_log() {
    local pattern="$1"
    for _ in $(seq 1 600); do
        grep -qE "$pattern" /out/log && return 0
        sleep 0.1
    done
    echo "  FAIL: timed out waiting for /$pattern/ in the entrypoint log"
    failures=$((failures + 1))
    return 1
}

echo "== a service that exits takes the container down with it"
rm -rf /out && mkdir -p /out
STUB_DIE_AS=multiplayer STUB_DIE_CODE=3 timeout -k 5 60 bash /entrypoint.sh > /out/log 2>&1
check "exit status" 3 "$?"
cat /out/log
check_log "the dead service is named in the log" 'ERROR: Multiplayer \(PID: [0-9]+\) has exited'
# The survivors were signalled and ran their own shutdown, rather than being left
# running or killed outright.
for name in lsp debugger gateway; do
    check "$name stopped cleanly" yes "$(stopped "$name")"
done

echo
echo "== a service that exits 0 is still a failure for the container"
rm -rf /out && mkdir -p /out
STUB_DIE_AS=gateway STUB_DIE_CODE=0 timeout -k 5 60 bash /entrypoint.sh > /out/log 2>&1
check "exit status" 1 "$?"
check_log "the dead service is named in the log" 'ERROR: Gateway \(PID: [0-9]+\) has exited'

echo
echo "== a service that ignores SIGTERM cannot hold the container open"
rm -rf /out && mkdir -p /out
STUB_DIE_AS=multiplayer STUB_DIE_CODE=3 STUB_IGNORE_TERM=lsp SHUTDOWN_GRACE_SECS=2 \
    timeout -k 5 60 bash /entrypoint.sh > /out/log 2>&1
check "exit status" 3 "$?"
check_log "the wedged service is killed after the grace period" \
    'WARNING: PID [0-9]+ did not stop within 2s, killing it'
check_log "the shutdown still completes" 'All services stopped'
# The ones that do honour SIGTERM still stop the polite way.
for name in debugger gateway; do
    check "$name stopped cleanly" yes "$(stopped "$name")"
done

echo
echo "== SIGTERM is still a clean shutdown"
rm -rf /out && mkdir -p /out
bash /entrypoint.sh > /out/log 2>&1 &
entrypoint_pid=$!
if await_log 'All enabled services started'; then
    kill -TERM "$entrypoint_pid"
    wait_bounded "$entrypoint_pid"
    check "exit status" 0 "$wait_status"
    for name in lsp multiplayer debugger gateway; do
        check "$name stopped cleanly" yes "$(stopped "$name")"
    done
else
    kill -KILL "$entrypoint_pid" 2>/dev/null
fi

echo
echo "== with no services enabled the entrypoint sleeps instead of exiting"
rm -rf /out && mkdir -p /out
ENABLE_LSP=false ENABLE_MULTIPLAYER=false ENABLE_DEBUGGER=false ENABLE_GATEWAY=false \
    bash /entrypoint.sh > /out/log 2>&1 &
entrypoint_pid=$!
if await_log 'Sleeping indefinitely'; then
    # Staying up is the correct behaviour here, so there is nothing to wait for
    # but the passage of time.
    sleep 2
    check "still running" yes "$(kill -0 "$entrypoint_pid" 2>/dev/null && echo yes || echo no)"
fi
# SIGKILL, not SIGTERM: this branch blocks in a foreground `sleep infinity`, so
# bash defers the trap until it returns. That is pre-existing behaviour and is
# not what this test is about.
kill -KILL "$entrypoint_pid" 2>/dev/null
wait_bounded "$entrypoint_pid"

echo
if [ "$failures" -eq 0 ]; then
    echo "all entrypoint supervision checks passed"
else
    echo "$failures check(s) failed"
fi
exit "$failures"
HARNESS

echo "Running entrypoint supervision tests in $IMAGE"

# --entrypoint: the windmill-extra image's own ENTRYPOINT is ["/entrypoint.sh"]
# (docker/DockerfileExtra), which would swallow the command and start the real
# services instead of the harness. Harmless for a plain base image.
DOCKER_RUN=(docker run --rm --init
    --entrypoint bash
    -v "$ENTRYPOINT:/entrypoint.sh:ro"
    -v "$WORKDIR:/harness:ro"
    "$IMAGE" /harness/harness.sh)

# Bound the whole run where the host has a `timeout` (GNU coreutils; stock macOS
# ships none), so a stalled image pull cannot hang a CI job. Every scenario
# inside the container is bounded on its own regardless.
if command -v timeout >/dev/null 2>&1; then
    timeout "${ENTRYPOINT_TEST_TIMEOUT:-900}" "${DOCKER_RUN[@]}"
else
    "${DOCKER_RUN[@]}"
fi
