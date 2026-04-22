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


# Path where ruff (spawned with workspace rooted at /tmp/monaco) will discover
# a ruff.toml. Ruff walks up from the file being linted looking for a
# ruff.toml / .ruff.toml / pyproject.toml, so dropping it here covers every
# python editor session.
RUFF_CONFIG_PATH = "/tmp/monaco/ruff.toml"
# How often to re-fetch the instance ruff config from the backend. Existing
# ruff server processes won't pick up the change mid-session, but the next
# editor reload / WebSocket reconnect will.
RUFF_CONFIG_POLL_INTERVAL_SECS = int(os.environ.get("RUFF_CONFIG_POLL_INTERVAL_SECS", "60"))


def _sync_ruff_config_once():
    """Fetch the instance ruff config from the windmill backend and write it
    to disk. No-op when WINDMILL_BASE_URL is unset (e.g., running the LSP
    standalone for local development)."""
    base_url = os.environ.get("WINDMILL_BASE_URL") or os.environ.get("BASE_INTERNAL_URL")
    if not base_url:
        return
    url = base_url.rstrip("/") + "/api/settings_u/ruff_config"
    try:
        with urllib.request.urlopen(url, timeout=5) as resp:
            body = resp.read().decode("utf-8")
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        log.warning("Could not fetch instance ruff config from %s: %s", url, e)
        return

    try:
        existing = ""
        if os.path.exists(RUFF_CONFIG_PATH):
            with open(RUFF_CONFIG_PATH, "r") as f:
                existing = f.read()
        if existing == body:
            return
        if body:
            os.makedirs(os.path.dirname(RUFF_CONFIG_PATH), exist_ok=True)
            with open(RUFF_CONFIG_PATH, "w") as f:
                f.write(body)
            log.info("Wrote instance ruff config to %s (%d bytes)", RUFF_CONFIG_PATH, len(body))
        elif os.path.exists(RUFF_CONFIG_PATH):
            os.remove(RUFF_CONFIG_PATH)
            log.info("Removed %s (instance ruff config is empty)", RUFF_CONFIG_PATH)
    except OSError as e:
        log.warning("Could not write ruff config to %s: %s", RUFF_CONFIG_PATH, e)


def start_ruff_config_poller():
    """Fetch the ruff config once synchronously, then poll in the background."""
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
