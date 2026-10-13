import asyncio
import logging
import shutil
import subprocess
import threading
import time
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

SHELLCHECK_BIN = shutil.which("shellcheck")
# The linter is configured here rather than by the editor's initializationOptions.
SHELLCHECK_ARGS = ["--norc", "--format=json1", "-"]
SHELLCHECK_TIMEOUT_SECS = float(os.environ.get("SHELLCHECK_TIMEOUT_SECS", "10"))
SHELLCHECK_MAX_DOC_CHARS = int(os.environ.get("SHELLCHECK_MAX_DOC_CHARS", "200000"))
SHELLCHECK_MAX_DOCS_PER_CONN = 8
# A document whose checks finish faster than this is checked on every change
# with no pause; see ShellcheckLS._changed.
SHELLCHECK_INSTANT_SECS = 0.05
SHELLCHECK_MAX_DEBOUNCE_SECS = 1.0
# Shared by every connection: shellcheck is CPU-bound and single-threaded, so
# more concurrent runs than cores only makes all of them slower.
_shellcheck_slots = asyncio.Semaphore(os.cpu_count() or 2)

_SHELLCHECK_SEVERITIES = {"error": 1, "warning": 2, "info": 3, "style": 3}


def _utf16_col(line, col):
    """shellcheck counts code points, LSP positions count UTF-16 code units."""
    col = max(col - 1, 0)
    if line.isascii():
        return col
    return len(line[:col].encode("utf-16-le")) // 2


def shellcheck_diagnostics(text, output):
    lines = text.split("\n")

    def position(line, col):
        line = max(line - 1, 0)
        return {
            "line": line,
            "character": _utf16_col(lines[line], col) if line < len(lines) else 0,
        }

    diagnostics = []
    for c in json.loads(output).get("comments", []):
        code = "SC%d" % c["code"]
        diagnostics.append(
            {
                "range": {
                    "start": position(c["line"], c["column"]),
                    "end": position(c["endLine"], c["endColumn"]),
                },
                "severity": _SHELLCHECK_SEVERITIES.get(c["level"], 3),
                "code": code,
                "codeDescription": {"href": "https://www.shellcheck.net/wiki/" + code},
                "source": "shellcheck",
                "message": c["message"],
            }
        )
    return diagnostics


class ShellcheckLS(websocket.WebSocketHandler):
    """Lints shell documents by running shellcheck itself, speaking only the
    slice of LSP that takes: document sync in, diagnostics out."""

    def open(self):
        self.docs = {}
        self.tasks = {}
        self.cost = {}
        self.linted = {}

    def check_origin(self, origin):
        return True

    def on_message(self, message):
        try:
            msg = json.loads(message)
            method = msg.get("method")
            params = msg.get("params") or {}
            if method == "initialize":
                self._send(
                    {
                        "jsonrpc": "2.0",
                        "id": msg.get("id"),
                        "result": {
                            "capabilities": {
                                "textDocumentSync": {"openClose": True, "change": 1}
                            },
                            "serverInfo": {"name": "windmill-shellcheck"},
                        },
                    }
                )
            elif method == "textDocument/didOpen":
                doc = params["textDocument"]
                self._changed(doc["uri"], doc.get("version"), doc["text"])
            elif method == "textDocument/didChange":
                doc = params["textDocument"]
                self._changed(
                    doc["uri"], doc.get("version"), params["contentChanges"][-1]["text"]
                )
            elif method == "textDocument/didClose":
                self._forget(params["textDocument"]["uri"])
            elif method == "shutdown":
                self._send({"jsonrpc": "2.0", "id": msg.get("id"), "result": None})
            elif method is not None and "id" in msg:
                self._send(
                    {
                        "jsonrpc": "2.0",
                        "id": msg["id"],
                        "error": {"code": -32601, "message": "Method not found"},
                    }
                )
        except (ValueError, KeyError, IndexError, TypeError, AttributeError):
            log.warning("shellcheck: dropping malformed message")

    def on_close(self):
        for uri in list(self.docs):
            self._forget(uri)

    def _send(self, msg):
        try:
            self.write_message(json.dumps(msg))
        except websocket.WebSocketClosedError:
            pass

    def _forget(self, uri):
        task = self.tasks.pop(uri, None)
        if task:
            task.cancel()
        self.docs.pop(uri, None)
        self.cost.pop(uri, None)
        self.linted.pop(uri, None)

    def _changed(self, uri, version, text):
        if not isinstance(uri, str) or not isinstance(text, str):
            raise TypeError
        if uri not in self.docs and len(self.docs) >= SHELLCHECK_MAX_DOCS_PER_CONN:
            return
        if len(text) > SHELLCHECK_MAX_DOC_CHARS:
            # Not checked, so not kept either: reported as having no findings.
            text = None
        self.docs[uri] = (version, text)
        if uri in self.linted and self.linted[uri] == text:
            return
        running = self.tasks.pop(uri, None)
        # The pause before a check is what that document's checks cost: one that
        # is cheap is checked at once, one that is not waits out a typing burst
        # rather than burning a core on results the next keystroke discards.
        delay = self.cost.get(uri, 0)
        if delay < SHELLCHECK_INSTANT_SECS:
            delay = 0
        if running and not running.done():
            running.cancel()
            delay = max(delay, SHELLCHECK_INSTANT_SECS)
        self.tasks[uri] = asyncio.ensure_future(self._lint(uri, version, text, delay))

    async def _lint(self, uri, version, text, delay):
        if delay:
            await asyncio.sleep(delay)
        diagnostics = []
        if SHELLCHECK_BIN and text is not None:
            async with _shellcheck_slots:
                started = time.monotonic()
                try:
                    output = await self._run(text)
                    self.cost[uri] = 0
                finally:
                    # A killed check counts too, as a lower bound: it is the only
                    # measure a document edited faster than it can be checked gets.
                    if uri in self.docs:
                        self.cost[uri] = min(
                            max(time.monotonic() - started, self.cost.get(uri, 0)),
                            SHELLCHECK_MAX_DEBOUNCE_SECS,
                        )
            if output is None:
                return
            try:
                diagnostics = shellcheck_diagnostics(text, output)
            except (ValueError, KeyError, TypeError) as e:
                log.warning("shellcheck: unreadable output: %s", e)
                return
        if self.docs.get(uri) != (version, text):
            return
        self.linted[uri] = text
        params = {"uri": uri, "diagnostics": diagnostics}
        if version is not None:
            params["version"] = version
        self._send(
            {
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": params,
            }
        )

    async def _run(self, text):
        try:
            proc = await asyncio.create_subprocess_exec(
                SHELLCHECK_BIN,
                *SHELLCHECK_ARGS,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL,
            )
        except OSError as e:
            log.warning("shellcheck: could not start: %s", e)
            return None
        try:
            output, _ = await asyncio.wait_for(
                proc.communicate(text.encode("utf-8", "replace")),
                SHELLCHECK_TIMEOUT_SECS,
            )
            return output
        except asyncio.TimeoutError:
            log.warning("shellcheck: timed out on a %d char document", len(text))
            return None
        finally:
            # Also reached when a newer change cancels this check mid-run.
            if proc.returncode is None:
                proc.kill()
                await proc.wait()


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
        f.write("module mymod\ngo 1.27")
        f.close()

    # Sync instance-level ruff config into /tmp/monaco/ruff.toml so every
    # spawned `ruff server` picks it up.
    start_ruff_config_poller()

    if not SHELLCHECK_BIN:
        log.warning("shellcheck is not on PATH: shell scripts will not be linted")

    port = int(os.environ.get("PORT", "3001"))
    app = web.Application(
        [
            (r"/ws/pyright", PyrightLS),
            (r"/ws/diagnostic", ShellcheckLS),
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
