#!/usr/bin/env python3
"""library-portal — a tiny read-only web viewer for the vecnode library/ folder.

Serves an Anthropic-style index of the PDFs found under /library (which is
bind-mounted read-only at runtime) and streams each file for in-browser viewing.
Nothing is copied into the image or container; the library is mounted live, so
the portal always reflects what is currently on disk. Pure Python stdlib — no
third-party dependencies — to keep the image as small as possible.
"""
import html
import os
import re
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LIBRARY = os.path.realpath(os.environ.get("LIBRARY_DIR", "/library"))
PORT = int(os.environ.get("PORT", "8090"))

NAME_RE = re.compile(r"^(\d{4})-([A-Za-z][A-Za-z0-9]*)-(.+)$")


def human_size(n: int) -> str:
    value = float(n)
    for unit in ("B", "KB", "MB", "GB"):
        if value < 1024 or unit == "GB":
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{n} B"


def split_camel(text: str) -> str:
    text = text.replace("_", " ")
    text = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", " ", text)
    text = re.sub(r"(?<=[A-Za-z])(?=[0-9])", " ", text)
    return text.strip()


def list_pdfs():
    items = []
    for root, dirs, files in os.walk(LIBRARY):
        dirs[:] = [d for d in dirs if not d.startswith(".")]  # skip .pdfding-data etc.
        for f in files:
            if not f.lower().endswith(".pdf"):
                continue
            full = os.path.join(root, f)
            rel = os.path.relpath(full, LIBRARY).replace(os.sep, "/")
            try:
                size = os.path.getsize(full)
            except OSError:
                size = 0
            stem = f[:-4]
            m = NAME_RE.match(stem)
            if m:
                year, author, title = m.group(1), m.group(2), split_camel(m.group(3))
            else:
                year, author, title = "", "", split_camel(stem)
            folder = os.path.dirname(rel)
            items.append(
                {"rel": rel, "file": f, "size": size, "year": year,
                 "author": author, "title": title, "folder": folder}
            )
    items.sort(key=lambda d: (d["year"], d["title"].lower()), reverse=True)
    return items


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Library</title>
<style>
  :root {{
    --bg: #F0EEE6; --surface: #FBFAF7; --ink: #181614; --muted: #73706A;
    --accent: #C9613E; --line: #E4E0D6;
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; background: var(--bg); color: var(--ink);
    font-family: -apple-system, "Segoe UI", Helvetica, Arial, sans-serif;
    line-height: 1.5; -webkit-font-smoothing: antialiased;
  }}
  .wrap {{ max-width: 880px; margin: 0 auto; padding: 56px 24px 96px; }}
  header h1 {{
    font-family: Georgia, "Times New Roman", serif; font-weight: 600;
    font-size: 40px; margin: 0 0 6px; letter-spacing: -0.5px;
  }}
  header p {{ margin: 0; color: var(--muted); font-size: 15px; }}
  .search {{
    width: 100%; margin: 28px 0 8px; padding: 13px 16px; font-size: 16px;
    background: var(--surface); border: 1px solid var(--line); border-radius: 12px;
    color: var(--ink); outline: none;
  }}
  .search:focus {{ border-color: var(--accent); }}
  .count {{ color: var(--muted); font-size: 13px; margin: 0 2px 18px; }}
  .item {{
    display: flex; align-items: baseline; gap: 14px; text-decoration: none;
    color: inherit; padding: 16px 18px; background: var(--surface);
    border: 1px solid var(--line); border-radius: 12px; margin-bottom: 10px;
    transition: border-color .12s ease, transform .12s ease;
  }}
  .item:hover {{ border-color: var(--accent); transform: translateY(-1px); }}
  .year {{
    flex: 0 0 auto; min-width: 46px; font-variant-numeric: tabular-nums;
    color: var(--accent); font-weight: 600; font-size: 14px; padding-top: 1px;
  }}
  .body {{ flex: 1 1 auto; min-width: 0; }}
  .title {{ font-size: 16px; font-weight: 500; }}
  .meta {{ color: var(--muted); font-size: 13px; margin-top: 2px;
           overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
  .size {{ flex: 0 0 auto; color: var(--muted); font-size: 12.5px;
           font-variant-numeric: tabular-nums; padding-top: 2px; }}
  .empty {{ color: var(--muted); padding: 40px 0; text-align: center; }}
  footer {{ color: var(--muted); font-size: 12px; margin-top: 30px; text-align: center; }}
</style>
</head>
<body>
  <div class="wrap">
    <header>
      <h1>Library</h1>
      <p>{count} documents · {total} · read-only</p>
    </header>
    <input id="q" class="search" type="search" placeholder="Search by title, author or year…" autofocus>
    <div id="count" class="count"></div>
    <div id="list">{rows}</div>
    <div id="empty" class="empty" style="display:none">No documents match your search.</div>
    <footer>library-portal · served live from <code>library/</code></footer>
  </div>
<script>
  const q = document.getElementById('q');
  const items = Array.from(document.querySelectorAll('.item'));
  const countEl = document.getElementById('count');
  const emptyEl = document.getElementById('empty');
  function apply() {{
    const term = q.value.trim().toLowerCase();
    let shown = 0;
    for (const el of items) {{
      const hit = !term || el.dataset.search.includes(term);
      el.style.display = hit ? '' : 'none';
      if (hit) shown++;
    }}
    countEl.textContent = shown + (shown === 1 ? ' result' : ' results');
    emptyEl.style.display = shown ? 'none' : '';
  }}
  q.addEventListener('input', apply);
  apply();
</script>
</body>
</html>"""


def render_index():
    items = list_pdfs()
    total = human_size(sum(i["size"] for i in items))
    rows = []
    for it in items:
        search = " ".join([it["title"], it["author"], it["year"], it["file"]]).lower()
        meta_bits = []
        if it["author"]:
            meta_bits.append(it["author"])
        if it["folder"] and it["folder"] != "pdfs":
            meta_bits.append(it["folder"])
        meta = html.escape(" · ".join(meta_bits))
        href = "/view/" + urllib.parse.quote(it["rel"])
        rows.append(
            f'<a class="item" href="{href}" target="_blank" rel="noopener" '
            f'data-search="{html.escape(search, quote=True)}">'
            f'<div class="year">{html.escape(it["year"])}</div>'
            f'<div class="body"><div class="title">{html.escape(it["title"])}</div>'
            f'<div class="meta">{meta}</div></div>'
            f'<div class="size">{human_size(it["size"])}</div></a>'
        )
    body = "\n".join(rows) if rows else '<div class="empty">No PDFs found in library/.</div>'
    return PAGE.format(count=len(items), total=total, rows=body)


def safe_path(rel: str):
    """Resolve a request path inside LIBRARY, blocking traversal/symlink escape."""
    rel = urllib.parse.unquote(rel)
    full = os.path.realpath(os.path.join(LIBRARY, rel))
    if full != LIBRARY and not full.startswith(LIBRARY + os.sep):
        return None
    if not os.path.isfile(full) or not full.lower().endswith(".pdf"):
        return None
    return full


class Handler(BaseHTTPRequestHandler):
    server_version = "library-portal"

    def _send(self, code, body=b"", ctype="text/html; charset=utf-8", extra=None):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        if body:
            self.wfile.write(body)

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path == "/" or path == "/index.html":
            self._send(200, render_index().encode("utf-8"))
            return
        if path.startswith("/view/"):
            full = safe_path(path[len("/view/"):])
            if not full:
                self._send(404, b"Not found", "text/plain; charset=utf-8")
                return
            try:
                with open(full, "rb") as fh:
                    data = fh.read()
            except OSError:
                self._send(404, b"Not found", "text/plain; charset=utf-8")
                return
            disp = 'inline; filename="%s"' % os.path.basename(full).replace('"', "")
            self._send(200, data, "application/pdf", {"Content-Disposition": disp})
            return
        if path == "/health":
            self._send(200, b"ok", "text/plain; charset=utf-8")
            return
        self._send(404, b"Not found", "text/plain; charset=utf-8")

    def log_message(self, *args):
        pass  # quiet


def main():
    print(f"[library-portal] serving {LIBRARY} on http://0.0.0.0:{PORT}", flush=True)
    ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
