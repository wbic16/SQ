#!/usr/bin/env python3
"""SQ Gateway v0.5.5 — per-tenant auth proxy for SQ phext instances.

Routes authenticated requests to per-tenant SQ processes based on a TOML
config that maps API tokens to local ports. Stdlib only (Python 3.11+).
"""

import http.server, http.client, json, signal, sys, os, tomllib

CONFIG_PATH = os.environ.get("SQ_GATEWAY_CONFIG",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "sq-gateway.toml"))

def load_config():
    with open(CONFIG_PATH, "rb") as f:
        raw = tomllib.load(f)
    gw = raw.get("gateway", {})
    tenants = {t["token"]: t for t in raw.get("tenants", [])}
    return gw, tenants

GW, TENANTS = load_config()

def reload_config(signum=None, frame=None):
    global GW, TENANTS
    try:
        GW, TENANTS = load_config()
        print(f"[sq-gw] config reloaded: {len(TENANTS)} tenant(s)", flush=True)
    except Exception as e:
        print(f"[sq-gw] reload failed: {e}", file=sys.stderr, flush=True)

signal.signal(signal.SIGHUP, reload_config)

CORS_HEADERS = [
    ("Access-Control-Allow-Origin", "*"),
    ("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    ("Access-Control-Allow-Headers", "Authorization, Content-Type"),
    ("Access-Control-Max-Age", "86400"),
]

def extract_token(headers):
    auth = headers.get("Authorization", "")
    if auth.startswith("Bearer "):
        return auth[7:].strip()
    return ""

class GatewayHandler(http.server.BaseHTTPRequestHandler):

    def send_cors(self):
        for k, v in CORS_HEADERS:
            self.send_header(k, v)

    def respond(self, code, body, content_type="application/json"):
        data = body.encode() if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_cors()
        self.end_headers()
        self.wfile.write(data)

    def respond_json(self, code, obj):
        self.respond(code, json.dumps(obj))

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_cors()
        self.end_headers()

    def route(self):
        token = extract_token(self.headers)

        if self.path == "/health":
            return self.respond_json(200, {"status": "ok", "tenants": len(TENANTS)})

        if self.path == "/admin/tenants":
            if token != GW.get("admin_key", ""):
                return self.respond_json(401, {"error": "unauthorized"})
            return self.respond_json(200, {"tenants": [t["name"] for t in TENANTS.values()]})

        if not token or token not in TENANTS:
            return self.respond_json(401, {"error": "invalid or missing token"})

        self.proxy_to(TENANTS[token]["port"])

    def proxy_to(self, port):
        body = None
        length = self.headers.get("Content-Length")
        if length:
            body = self.rfile.read(int(length))

        fwd_headers = {k: v for k, v in self.headers.items() if k.lower() != "host"}

        try:
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
            conn.request(self.command, self.path, body=body, headers=fwd_headers)
            resp = conn.getresponse()
            resp_body = resp.read()

            self.send_response(resp.status)
            for k, v in resp.getheaders():
                if k.lower() not in ("transfer-encoding", "connection"):
                    self.send_header(k, v)
            self.send_cors()
            self.end_headers()
            self.wfile.write(resp_body)
            conn.close()
        except Exception as e:
            self.respond_json(502, {"error": f"backend: {e}"})

    do_GET = do_POST = do_PUT = do_DELETE = do_PATCH = route

    def log_message(self, fmt, *args):
        print(f"[sq-gw] {self.address_string()} {fmt % args}", flush=True)

if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else GW.get("listen_port", 8080)
    server = http.server.HTTPServer(("0.0.0.0", port), GatewayHandler)
    print(f"[sq-gw] v0.5.5 listening on :{port} — {len(TENANTS)} tenant(s)", flush=True)
    print(f"[sq-gw] send SIGHUP to reload config", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[sq-gw] shutdown", flush=True)
        server.shutdown()
