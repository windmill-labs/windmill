import logging
import subprocess
import threading
import os
import urllib.request
import urllib.error

from tornado import ioloop, process, web, websocket

from pylsp_jsonrpc import streams

try:
    import ujson as json
except Exception:  # pylint: disable=broad-except
    import json

log = logging.getLogger(__name__)
logging.basicConfig(level=os.environ.get("LOGLEVEL", "INFO"))


# Named explicitly by the python editor's ruff initializationOptions, which must
# stay in sync with this path (Editor.svelte explains why ruff needs it named).
RUFF_CONFIG_PATH = "/tmp/monaco/ruff.toml"
# How often to re-fetch the instance ruff config from the backend. Existing
# ruff server processes won't pick up the change mid-session, but the next
# editor reload / WebSocket reconnect will.
RUFF_CONFIG_POLL_INTERVAL_SECS = int(os.environ.get("RUFF_CONFIG_POLL_INTERVAL_SECS", "60"))

# Written whenever the instance sets no `ruff_config` of its own. Ruff's own
# defaults grow between releases (0.16 went from 59 to 413 enabled rules), so
# without this every ruff bump would change the diagnostics script authors see.
DEFAULT_RUFF_CONFIG = '[lint]\nselect = ["E4", "E7", "E9", "F"]\n'


def _write_ruff_config(content):
    """Write content to RUFF_CONFIG_PATH, skipping the write when unchanged."""
    try:
        if os.path.exists(RUFF_CONFIG_PATH):
            with open(RUFF_CONFIG_PATH, "r") as f:
                if f.read() == content:
                    return
        os.makedirs(os.path.dirname(RUFF_CONFIG_PATH), exist_ok=True)
        # Rename into place: a `ruff server` spawned mid-write must never read a
        # truncated config, which would drop that session onto ruff's defaults.
        tmp_path = RUFF_CONFIG_PATH + ".tmp"
        with open(tmp_path, "w") as f:
            f.write(content)
        os.replace(tmp_path, RUFF_CONFIG_PATH)
        log.info("Wrote ruff config to %s (%d bytes)", RUFF_CONFIG_PATH, len(content))
    except OSError as e:
        log.warning("Could not write ruff config to %s: %s", RUFF_CONFIG_PATH, e)


def _sync_ruff_config_once():
    """Fetch the instance ruff config from the windmill backend and write it to
    disk, falling back to DEFAULT_RUFF_CONFIG when the instance has none.
    Falls back without fetching when WINDMILL_BASE_URL is unset (e.g., running
    the LSP standalone for local development)."""
    base_url = os.environ.get("WINDMILL_BASE_URL") or os.environ.get("BASE_INTERNAL_URL")
    if not base_url:
        _write_ruff_config(DEFAULT_RUFF_CONFIG)
        return
    url = base_url.rstrip("/") + "/api/settings_u/ruff_config"
    try:
        with urllib.request.urlopen(url, timeout=5) as resp:
            body = resp.read().decode("utf-8")
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        log.warning("Could not fetch instance ruff config from %s: %s", url, e)
        return

    # A blank-but-not-empty setting (a stray newline left in the codearea) would
    # otherwise parse as a valid config selecting ruff's defaults.
    _write_ruff_config(body if body.strip() else DEFAULT_RUFF_CONFIG)


def start_ruff_config_poller():
    """Seed the default ruff config, fetch the instance override once
    synchronously, then poll in the background."""
    # Seeded first so a backend that is unreachable at startup still leaves the
    # editor on the pinned rule selection rather than on ruff's own defaults.
    _write_ruff_config(DEFAULT_RUFF_CONFIG)
    _sync_ruff_config_once()

    def loop():
        import time
        while True:
            time.sleep(RUFF_CONFIG_POLL_INTERVAL_SECS)
            _sync_ruff_config_once()

    t = threading.Thread(target=loop, name="ruff-config-poller", daemon=True)
    t.start()


class LanguageServerWebSocketHandler(websocket.WebSocketHandler):
    """Setup tornado websocket handler to host an external language server."""

    writer = None
    id = None
    proc = None
    loop = None

    def open(self):
        self.id = str(self)
        log.info("Spawning pylsp subprocess" + self.id)
        # Create an instance of the language server
        self.proc = process.Subprocess(
            self.procargs,
            env=os.environ,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )

        # Create a writer that formats json messages with the correct LSP headers
        self.writer = streams.JsonRpcStreamWriter(self.proc.stdin)

        # Create a reader for consuming stdout of the language server. We need to
        # consume this in another thread
        def consume():
            # Start a tornado IOLoop for reading/writing to the process in this thread
            self.loop = ioloop.IOLoop()
            reader = streams.JsonRpcStreamReader(self.proc.stdout)

            def on_listen(msg):
                try:
                    self.write_message(json.dumps(msg))
                except Exception as e:
                    log.error("Error writing message", e)

            reader.listen(on_listen)

        self.thread = threading.Thread(target=consume)
        self.thread.daemon = True
        self.thread.start()

    def on_message(self, message):
        """Forward client->server messages to the endpoint."""
        if not "Unhandled method" in message:
            self.writer.write(json.loads(message))

    def on_close(self) -> None:
        log.info("CLOSING: " + str(self.id))
        self.proc.proc.terminate()
        self.writer.close()
        self.loop.stop()

    def check_origin(self, origin):
        return True

class PyrightLS(LanguageServerWebSocketHandler):
    procargs = ["pipenv", "run", "pyright-langserver", "--stdio"]

class DiagnosticLS(LanguageServerWebSocketHandler):
    procargs = ["diagnostic-languageserver", "--stdio", "--log-level", "4"]

class RuffLS(LanguageServerWebSocketHandler):
    procargs = ["ruff", "server"]

class DenoLS(LanguageServerWebSocketHandler):
    procargs = ["deno", "lsp"]

class GoLS(LanguageServerWebSocketHandler):
    procargs = ["gopls", "serve"]

class MainHandler(web.RequestHandler):
    def get(self):
        self.write("ok")

class HealthHandler(web.RequestHandler):
    def set_default_headers(self):
        self.set_header("Access-Control-Allow-Origin", "*")
        self.set_header("Content-Type", "application/json")

    def get(self):
        log.info("HTTP GET %s", self.request.uri)
        self.write(json.dumps({"status": "ok", "service": "lsp"}))

    def options(self):
        self.set_status(204)
        self.finish()

class PingHandler(websocket.WebSocketHandler):
    def open(self):
        log.info("WS ping from %s", self.request.remote_ip)
        self.write_message(json.dumps({"type": "pong", "service": "lsp"}))
        self.close()

    def check_origin(self, origin):
        return True

if __name__ == "__main__":
    monaco_path = "/tmp/monaco"
    os.makedirs(monaco_path, exist_ok=True)
    print("The monaco directory is created!")
    go_mod_path = os.path.join(monaco_path, "go.mod")
    if not os.path.exists(go_mod_path):
        f = open(go_mod_path, "w")
        f.write("module mymod\ngo 1.26")
        f.close()

    # Sync instance-level ruff config into /tmp/monaco/ruff.toml so every
    # spawned `ruff server` picks it up.
    start_ruff_config_poller()

    port = int(os.environ.get("PORT", "3001"))
    app = web.Application(
        [
            (r"/ws/pyright", PyrightLS),
            (r"/ws/diagnostic", DiagnosticLS),
            (r"/ws/ruff", RuffLS),
            (r"/ws/deno", DenoLS),
            (r"/ws/go", GoLS),
            (r"/ws/ping", PingHandler),
            (r"/ws/health", HealthHandler),
            (r"/", MainHandler),
            (r"/health", HealthHandler),
        ]
    )
    app.listen(port, address="0.0.0.0")
    ioloop.IOLoop.current().start()
