"""Minimal MCP client for the Konnect KiCad server (JSON-RPC over stdio).

Schematic edits go through Konnect's validated engine, never raw text edits.
This lets a script issue many Konnect calls in one run.
"""
import json
import subprocess

KONNECT = "/home/alex/Konnect/target/release/konnect"


class Konnect:
    def __init__(self):
        self.p = subprocess.Popen([KONNECT], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.n = 0
        self._rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                                 "clientInfo": {"name": "ref-design-script", "version": "1"}})
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def _send(self, msg):
        self.p.stdin.write(json.dumps(msg) + "\n")
        self.p.stdin.flush()

    def _rpc(self, method, params):
        self.n += 1
        self._send({"jsonrpc": "2.0", "id": self.n, "method": method, "params": params})
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("konnect exited")
            msg = json.loads(line)
            if msg.get("id") == self.n:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg["result"]

    def tools(self):
        return self._rpc("tools/list", {})["tools"]

    def call(self, tool, **args):
        res = self._rpc("tools/call", {"name": tool, "arguments": args})
        text = "".join(c.get("text", "") for c in res.get("content", []))
        if res.get("isError"):
            raise RuntimeError(f"{tool}: {text[:500]}")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text

    def close(self):
        self.p.terminate()
