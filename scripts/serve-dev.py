#!/usr/bin/env python3
"""Static server for local/LAN testing that never lets a browser cache.

Phones cache CSS and JS hard, which silently hides edits during a UI session.
Everything here is served no-store so a reload always shows the current build.
"""
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


class NoCacheHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        super().end_headers()

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    root = sys.argv[2] if len(sys.argv) > 2 else "."
    handler = partial(NoCacheHandler, directory=root)
    print(f"serving {root} on 0.0.0.0:{port} (no-store)")
    ThreadingHTTPServer(("0.0.0.0", port), handler).serve_forever()
