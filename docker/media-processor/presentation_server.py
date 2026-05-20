#!/usr/bin/env python3
from __future__ import annotations

import http.server
import os
import socketserver
import sys
from pathlib import Path


def main() -> None:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8087
    configured_dir = os.environ.get("HOST_DESKTOP_DIR", "").strip()
    if configured_dir:
        base_dir = Path(configured_dir).expanduser()
    else:
        home_desktop = Path.home() / "Desktop"
        if home_desktop.exists():
            base_dir = home_desktop
        else:
            base_dir = Path("/outputs")

    base_dir.mkdir(parents=True, exist_ok=True)

    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(base_dir), **kwargs)

    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    with ReusableTCPServer(("0.0.0.0", port), Handler) as httpd:
        print(f"[INFO] Presentation server listening on 0.0.0.0:{port} from {base_dir}")
        httpd.serve_forever()


if __name__ == "__main__":
    main()
