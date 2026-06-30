#!/usr/bin/env python3
"""media-downloader — a tiny web UI/API wrapping yt-dlp + ffmpeg.

Paste a video URL, pick MP3 / WAV / MP4, and the browser downloads the
converted file. No state is kept: each request downloads to a private temp
directory, streams the result back, then deletes it.
"""
import json
import os
import shutil
import subprocess
import tempfile
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("PORT", "8095"))
DOWNLOAD_TIMEOUT_SECONDS = int(os.environ.get("DOWNLOAD_TIMEOUT_SECONDS", "1800"))

CONTENT_TYPES = {
    "mp3": "audio/mpeg",
    "wav": "audio/wav",
    "mp4": "video/mp4",
}

PAGE = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>media downloader</title>
<style>
:root{--bg:#0B0E14;--surface:#141925;--ink:#E7ECF3;--muted:#8993A6;--accent:#0EA5E9;--line:#232B3B;}
*{box-sizing:border-box;}
body{margin:0;background:var(--bg);color:var(--ink);font-family:-apple-system,"Segoe UI",Helvetica,Arial,sans-serif;line-height:1.5;}
.wrap{max-width:640px;margin:0 auto;padding:64px 24px 48px;}
h1{font-size:26px;margin:0 0 6px;font-weight:600;}
p.sub{margin:0 0 28px;color:var(--muted);font-size:14.5px;}
.url{width:100%;padding:13px 15px;font-size:15px;background:var(--surface);border:1px solid var(--line);
  border-radius:10px;color:var(--ink);outline:none;}
.url:focus{border-color:var(--accent);}
.row{display:flex;gap:10px;margin-top:14px;flex-wrap:wrap;}
button.kind{flex:1 1 120px;padding:12px;font-size:14px;font-weight:600;background:var(--surface);
  border:1px solid var(--line);border-radius:10px;color:var(--ink);cursor:pointer;transition:border-color .12s;}
button.kind:hover{border-color:var(--accent);}
button.kind:disabled{opacity:.5;cursor:not-allowed;}
.status{margin-top:22px;padding:14px 16px;background:var(--surface);border:1px solid var(--line);
  border-radius:10px;font-size:13.5px;color:var(--muted);min-height:20px;white-space:pre-wrap;word-break:break-word;}
footer{margin-top:30px;color:var(--muted);font-size:12px;}
code{background:var(--surface);padding:1px 6px;border-radius:5px;}
</style></head>
<body>
<div class="wrap">
  <h1>media downloader</h1>
  <p class="sub">Paste a video link, pick a format. Powered by yt-dlp + ffmpeg.</p>
  <input id="url" class="url" type="text" placeholder="https://..." autocomplete="off">
  <div class="row">
    <button class="kind" data-kind="mp3" type="button">Download MP3</button>
    <button class="kind" data-kind="wav" type="button">Download WAV</button>
    <button class="kind" data-kind="mp4" type="button">Download MP4</button>
  </div>
  <div id="status" class="status">Ready.</div>
  <footer>API: <code>POST /api/download</code> &middot; Health: <code>/health</code></footer>
</div>
<script>
const urlInput = document.getElementById('url');
const statusEl = document.getElementById('status');
const buttons = Array.from(document.querySelectorAll('button.kind'));

function setButtonsDisabled(disabled) {
  for (const b of buttons) b.disabled = disabled;
}

async function startDownload(kind) {
  const url = urlInput.value.trim();
  if (!url) { statusEl.textContent = 'Paste a URL first.'; return; }

  setButtonsDisabled(true);
  const startedAt = performance.now();
  statusEl.textContent = 'Downloading (' + kind.toUpperCase() + ')...';
  const timer = setInterval(() => {
    const elapsed = ((performance.now() - startedAt) / 1000).toFixed(0);
    statusEl.textContent = 'Downloading (' + kind.toUpperCase() + ')... ' + elapsed + 's elapsed';
  }, 1000);

  try {
    const res = await fetch('/api/download', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, kind }),
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      throw new Error(err.detail || res.statusText);
    }
    const disposition = res.headers.get('Content-Disposition') || '';
    const match = /filename="?([^"]+)"?/.exec(disposition);
    const filename = match ? match[1] : ('download.' + kind);
    const blob = await res.blob();
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    statusEl.textContent = 'Done: ' + filename;
  } catch (error) {
    statusEl.textContent = 'Error: ' + error.message;
  } finally {
    clearInterval(timer);
    setButtonsDisabled(false);
  }
}

for (const b of buttons) {
  b.addEventListener('click', () => startDownload(b.dataset.kind));
}
urlInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') startDownload('mp4');
});
</script>
</body></html>
"""


def tool_version(command: list) -> str:
    try:
        out = subprocess.check_output(command, stderr=subprocess.STDOUT, text=True, timeout=10)
        return out.splitlines()[0].strip() if out else "unknown"
    except Exception:
        return "unavailable"


class Handler(BaseHTTPRequestHandler):
    server_version = "media-downloader"

    def _send(self, code, body=b"", ctype="text/html; charset=utf-8", extra=None):
        if isinstance(body, str):
            body = body.encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        if body:
            self.wfile.write(body)

    def _json(self, code, obj):
        self._send(code, json.dumps(obj), "application/json")

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", "/index.html"):
            return self._send(200, PAGE)
        if path == "/health":
            return self._json(200, {
                "status": "ok",
                "service": "media-downloader-api",
                "yt_dlp": tool_version(["yt-dlp", "--version"]),
                "ffmpeg": tool_version(["ffmpeg", "-version"]),
            })
        return self._send(404, b"Not found", "text/plain")

    def do_POST(self):
        if self.path.split("?", 1)[0] != "/api/download":
            return self._send(404, b"Not found", "text/plain")
        return self.handle_download()

    def handle_download(self):
        try:
            length = int(self.headers.get("Content-Length", "0") or "0")
            raw = self.rfile.read(length) if length else b"{}"
            payload = json.loads(raw or b"{}")
        except Exception:
            return self._json(400, {"detail": "Invalid JSON body."})

        url = str(payload.get("url", "")).strip()
        kind = str(payload.get("kind", "")).strip().lower()
        if kind not in CONTENT_TYPES:
            return self._json(400, {"detail": "kind must be mp3, wav, or mp4."})

        parsed = urllib.parse.urlparse(url)
        if parsed.scheme not in ("http", "https") or not parsed.netloc:
            return self._json(400, {"detail": "Please provide a valid http(s) URL."})

        job_dir = tempfile.mkdtemp(prefix="dl-")
        out_template = os.path.join(job_dir, "%(title).200B.%(ext)s")

        if kind in ("mp3", "wav"):
            cmd = [
                "yt-dlp", "--no-playlist", "--restrict-filenames",
                "-x", "--audio-format", kind,
                "-o", out_template, url,
            ]
        else:
            cmd = [
                "yt-dlp", "--no-playlist", "--restrict-filenames",
                "-f", "bv*+ba/b", "--merge-output-format", "mp4",
                "-o", out_template, url,
            ]

        try:
            proc = subprocess.run(
                cmd, capture_output=True, text=True, timeout=DOWNLOAD_TIMEOUT_SECONDS
            )
        except subprocess.TimeoutExpired:
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(504, {"detail": "Download timed out."})

        if proc.returncode != 0:
            shutil.rmtree(job_dir, ignore_errors=True)
            tail = (proc.stderr or proc.stdout or "").strip()[-1500:]
            return self._json(400, {"detail": "yt-dlp failed: " + tail})

        produced = sorted(os.listdir(job_dir))
        if not produced:
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(500, {"detail": "yt-dlp reported success but produced no file."})

        filename = produced[0]
        out_path = os.path.join(job_dir, filename)
        try:
            size = os.path.getsize(out_path)
            disp = 'attachment; filename="%s"' % filename.replace('"', "")
            self.send_response(200)
            self.send_header("Content-Type", CONTENT_TYPES[kind])
            self.send_header("Content-Length", str(size))
            self.send_header("Content-Disposition", disp)
            self.end_headers()
            with open(out_path, "rb") as fh:
                shutil.copyfileobj(fh, self.wfile)
        finally:
            shutil.rmtree(job_dir, ignore_errors=True)

    def log_message(self, fmt, *args):
        print("[media-downloader] " + (fmt % args))


def main():
    ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
