#!/usr/bin/env python3
"""media-downloader — a tiny web UI/API wrapping yt-dlp + ffmpeg.

Paste a video URL, pick MP3 / WAV / MP4, and the file is saved to the host
folder bind-mounted at OUTPUT_DIR (the host Desktop, in normal use).

Safety model (this app downloads from arbitrary, untrusted web links):
  * The container runs non-root, with all Linux capabilities dropped and
    no-new-privileges (set by the run scripts).
  * Only http/https URLs are accepted, and URLs whose host resolves to a
    loopback/private/link-local/reserved address are rejected (a basic SSRF
    guard so the tool can't be aimed at the host or LAN).
  * yt-dlp runs with --ignore-config (no attacker-supplied config is read),
    --restrict-filenames, --no-playlist and a --max-filesize cap. It never
    runs post-process commands.
  * The download lands in a temp dir *inside* OUTPUT_DIR, then the single
    produced file is moved up with a sanitized basename whose realpath is
    verified to stay within OUTPUT_DIR (no path traversal), and never
    overwrites an existing file (a numeric suffix is added on collision).
"""
import ipaddress
import json
import os
import re
import shutil
import socket
import subprocess
import tempfile
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("PORT", "8095"))
DOWNLOAD_TIMEOUT_SECONDS = int(os.environ.get("DOWNLOAD_TIMEOUT_SECONDS", "1800"))
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/output")
# Label shown to the user for the output folder (the run scripts set "Desktop").
OUTPUT_LABEL = os.environ.get("OUTPUT_LABEL", "the output folder")
MAX_FILESIZE = os.environ.get("MAX_FILESIZE", "4G")
# Set ALLOW_PRIVATE_HOSTS=1 only if you knowingly want to fetch from LAN hosts.
ALLOW_PRIVATE_HOSTS = os.environ.get("ALLOW_PRIVATE_HOSTS", "0") == "1"

KINDS = ("mp3", "wav", "mp4")

PAGE = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>media downloader</title>
<style>
:root{--bg:#0B0E14;--surface:#141925;--ink:#E7ECF3;--muted:#8993A6;--accent:#0EA5E9;--line:#232B3B;--ok:#34D399;--err:#F87171;}
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
.status.ok{color:var(--ok);border-color:var(--ok);}
.status.err{color:var(--err);border-color:var(--err);}
footer{margin-top:30px;color:var(--muted);font-size:12px;}
code{background:var(--surface);padding:1px 6px;border-radius:5px;}
</style></head>
<body>
<div class="wrap">
  <h1>media downloader</h1>
  <p class="sub">Paste a video link, pick a format. Files are saved to your <strong>__OUTPUT_LABEL__</strong>. Powered by yt-dlp + ffmpeg.</p>
  <input id="url" class="url" type="text" placeholder="https://..." autocomplete="off">
  <div class="row">
    <button class="kind" data-kind="mp3" type="button">Save MP3</button>
    <button class="kind" data-kind="wav" type="button">Save WAV</button>
    <button class="kind" data-kind="mp4" type="button">Save MP4</button>
  </div>
  <div id="status" class="status">Ready.</div>
  <footer>API: <code>POST /api/download</code> &middot; Health: <code>/health</code></footer>
</div>
<script>
const urlInput = document.getElementById('url');
const statusEl = document.getElementById('status');
const buttons = Array.from(document.querySelectorAll('button.kind'));

function setStatus(text, cls) {
  statusEl.textContent = text;
  statusEl.className = 'status' + (cls ? ' ' + cls : '');
}
function setButtonsDisabled(disabled) {
  for (const b of buttons) b.disabled = disabled;
}

async function startDownload(kind) {
  const url = urlInput.value.trim();
  if (!url) { setStatus('Paste a URL first.', 'err'); return; }

  setButtonsDisabled(true);
  const startedAt = performance.now();
  setStatus('Downloading (' + kind.toUpperCase() + ')...');
  const timer = setInterval(() => {
    const elapsed = ((performance.now() - startedAt) / 1000).toFixed(0);
    setStatus('Downloading (' + kind.toUpperCase() + ')... ' + elapsed + 's elapsed');
  }, 1000);

  try {
    const res = await fetch('/api/download', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, kind }),
    });
    const data = await res.json().catch(() => ({ detail: res.statusText }));
    if (!res.ok || !data.ok) {
      throw new Error(data.detail || res.statusText);
    }
    setStatus('Saved to ' + data.saved_to + ':\\n' + data.filename, 'ok');
  } catch (error) {
    setStatus('Error: ' + error.message, 'err');
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


def host_is_blocked(host: str) -> bool:
    """True if the host resolves to a non-public address (basic SSRF guard)."""
    if ALLOW_PRIVATE_HOSTS:
        return False
    host = host.strip("[]")
    try:
        infos = socket.getaddrinfo(host, None)
    except Exception:
        # Unresolvable here — let yt-dlp try and fail rather than hard-blocking.
        return False
    for info in infos:
        ip = info[4][0]
        try:
            addr = ipaddress.ip_address(ip)
        except ValueError:
            continue
        if (addr.is_loopback or addr.is_private or addr.is_link_local
                or addr.is_reserved or addr.is_multicast or addr.is_unspecified):
            return True
    return False


def sanitize_basename(name: str) -> str:
    """Reduce to a single safe path component (defense-in-depth on top of
    yt-dlp's --restrict-filenames)."""
    name = os.path.basename(name)
    name = name.replace("\\", "_")
    name = re.sub(r'[<>:"/|?*\x00-\x1f]', "_", name)
    name = name.strip(" .") or "download"
    return name[:200]


def collision_safe_path(directory: str, filename: str) -> str:
    stem, ext = os.path.splitext(filename)
    candidate = os.path.join(directory, filename)
    i = 1
    while os.path.exists(candidate):
        candidate = os.path.join(directory, f"{stem}-{i}{ext}")
        i += 1
    return candidate


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
            return self._send(200, PAGE.replace("__OUTPUT_LABEL__", OUTPUT_LABEL))
        if path == "/health":
            return self._json(200, {
                "status": "ok",
                "service": "media-downloader-api",
                "output_dir": OUTPUT_DIR,
                "output_writable": os.path.isdir(OUTPUT_DIR) and os.access(OUTPUT_DIR, os.W_OK),
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
        if kind not in KINDS:
            return self._json(400, {"detail": "kind must be mp3, wav, or mp4."})

        parsed = urllib.parse.urlparse(url)
        if parsed.scheme not in ("http", "https") or not parsed.hostname:
            return self._json(400, {"detail": "Please provide a valid http(s) URL."})
        if host_is_blocked(parsed.hostname):
            return self._json(403, {"detail": "Refusing to fetch from a local/private address."})

        if not (os.path.isdir(OUTPUT_DIR) and os.access(OUTPUT_DIR, os.W_OK)):
            return self._json(500, {
                "detail": f"Output folder {OUTPUT_DIR} is not mounted/writable. "
                          "Start the container via 'vn run ...-open-media-downloader'."
            })

        # Download into a temp dir INSIDE the output dir so the final move is a
        # same-filesystem rename and large files never touch container RAM/layers.
        try:
            job_dir = tempfile.mkdtemp(prefix=".vn-dl-", dir=OUTPUT_DIR)
        except OSError as exc:
            return self._json(500, {"detail": f"Cannot create temp dir in output: {exc}"})

        out_template = os.path.join(job_dir, "%(title).180B.%(ext)s")
        base_cmd = [
            "yt-dlp",
            "--ignore-config",
            "--no-playlist",
            "--restrict-filenames",
            "--no-exec",
            "--max-filesize", MAX_FILESIZE,
            "--retries", "3",
            "--socket-timeout", "30",
            "-o", out_template,
        ]
        if kind in ("mp3", "wav"):
            cmd = base_cmd + ["-x", "--audio-format", kind, url]
        else:
            cmd = base_cmd + ["-f", "bv*+ba/b", "--merge-output-format", "mp4", url]

        env = dict(os.environ)
        env["HOME"] = "/tmp"
        env["XDG_CACHE_HOME"] = "/tmp"

        try:
            proc = subprocess.run(
                cmd, capture_output=True, text=True,
                timeout=DOWNLOAD_TIMEOUT_SECONDS, env=env,
            )
        except subprocess.TimeoutExpired:
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(504, {"detail": "Download timed out."})

        if proc.returncode != 0:
            shutil.rmtree(job_dir, ignore_errors=True)
            tail = (proc.stderr or proc.stdout or "").strip()[-1500:]
            return self._json(400, {"detail": "yt-dlp failed: " + tail})

        # Pick the produced file matching the requested extension (largest, if several).
        produced = [
            os.path.join(job_dir, f) for f in os.listdir(job_dir)
            if f.lower().endswith("." + kind)
        ]
        if not produced:
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(500, {"detail": "Download produced no output file."})
        src = max(produced, key=lambda p: os.path.getsize(p))

        safe_name = sanitize_basename(os.path.basename(src))
        final_path = collision_safe_path(OUTPUT_DIR, safe_name)

        # Confirm the destination really stays inside OUTPUT_DIR (no traversal).
        out_real = os.path.realpath(OUTPUT_DIR)
        final_real = os.path.realpath(final_path)
        if final_real != out_real and not final_real.startswith(out_real + os.sep):
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(400, {"detail": "Rejected unsafe output path."})

        try:
            os.replace(src, final_path)
        except OSError as exc:
            shutil.rmtree(job_dir, ignore_errors=True)
            return self._json(500, {"detail": f"Could not save file: {exc}"})
        shutil.rmtree(job_dir, ignore_errors=True)

        return self._json(200, {
            "ok": True,
            "filename": os.path.basename(final_path),
            "saved_to": OUTPUT_LABEL,
        })

    def log_message(self, fmt, *args):
        print("[media-downloader] " + (fmt % args))


def main():
    ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
