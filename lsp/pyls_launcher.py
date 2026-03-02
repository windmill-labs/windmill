import logging
import subprocess
import threading
import os

from tornado import ioloop, process, web, websocket

from pylsp_jsonrpc import streams

try:
    import ujson as json
except Exception:  # pylint: disable=broad-except
    import json

log = logging.getLogger(__name__)
logging.basicConfig(level=os.environ.get("LOGLEVEL", "INFO"))


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

if __name__ == "__main__":
    monaco_path = "/tmp/monaco"
    os.makedirs(monaco_path, exist_ok=True)
    print("The monaco directory is created!")
    go_mod_path = os.path.join(monaco_path, "go.mod")
    if not os.path.exists(go_mod_path):
        f = open(go_mod_path, "w")
        f.write("module mymod\ngo 1.26")
        f.close()
    port = int(os.environ.get("PORT", "3001"))
    app = web.Application(
        [
            (r"/ws/pyright", PyrightLS),
            (r"/ws/diagnostic", DiagnosticLS),
            (r"/ws/ruff", RuffLS),
            (r"/ws/deno", DenoLS),
            (r"/ws/go", GoLS),
            (r"/", MainHandler),
            (r"/health", MainHandler),
        ]
    )
    app.listen(port, address="0.0.0.0")
    ioloop.IOLoop.current().start()
