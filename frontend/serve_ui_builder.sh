# node ./scripts/untar_ui_builder.js

# mkdir ui_builder_serve || true
# cp -r static/ui_builder ui_builder_serve/ui_builder || true
# rm -rf static/ui_builder || true
python3 -c "
import os
os.chdir('ui_builder_serve')
from http.server import HTTPServer, SimpleHTTPRequestHandler
class H(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Resource-Policy', 'cross-origin')
        super().end_headers()
HTTPServer(('', 4000), H).serve_forever()
"