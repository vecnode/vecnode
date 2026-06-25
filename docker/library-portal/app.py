#!/usr/bin/env python3
"""library-portal — a light web viewer/manager for the vecnode library/ folder.

Serves an Anthropic-style index of the PDFs under /library (bind-mounted at
runtime) and streams each file for in-browser viewing. It also lets you:

  * edit a document's display metadata (title / author / year)
  * rename the file on disk
  * add tags (e.g. "read")
  * switch between list and grid (thumbnail) views
  * sort by title (A-Z / Z-A) or year

App state (metadata overrides + tags) lives in a small sidecar file at
/library/.portal/portal.json, and thumbnails are cached under
/library/.portal/thumbs/. The PDFs themselves are only modified on an explicit
rename. Thumbnails are rendered with PyMuPDF if available; otherwise the grid
view shows a simple placeholder.
"""
import hashlib
import html
import json
import os
import re
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

try:
    import fitz  # PyMuPDF, for thumbnails
    fitz.TOOLS.mupdf_display_errors(False)
except Exception:  # pragma: no cover
    fitz = None

LIBRARY = os.path.realpath(os.environ.get("LIBRARY_DIR", "/library"))
PORT = int(os.environ.get("PORT", "8090"))
PORTAL_DIR = os.path.join(LIBRARY, ".portal")
SIDECAR = os.path.join(PORTAL_DIR, "portal.json")
THUMB_DIR = os.path.join(PORTAL_DIR, "thumbs")

NAME_RE = re.compile(r"^(\d{4})-([A-Za-z][A-Za-z0-9]*)-(.+)$")
_LOCK = threading.Lock()


# --------------------------------------------------------------------------- #
# helpers
# --------------------------------------------------------------------------- #
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


def load_sidecar() -> dict:
    try:
        with open(SIDECAR, encoding="utf-8") as fh:
            data = json.load(fh)
            return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def save_sidecar(data: dict) -> None:
    os.makedirs(PORTAL_DIR, exist_ok=True)
    tmp = SIDECAR + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(data, fh, ensure_ascii=False, indent=1)
    os.replace(tmp, SIDECAR)


def norm_tags(raw) -> list:
    if isinstance(raw, str):
        parts = re.split(r"[,\s]+", raw)
    elif isinstance(raw, list):
        parts = raw
    else:
        parts = []
    out, seen = [], set()
    for p in parts:
        t = str(p).strip().lstrip("#").lower()
        t = re.sub(r"[^a-z0-9_-]", "", t)
        if t and t not in seen:
            seen.add(t)
            out.append(t)
    return out


def list_pdfs() -> list:
    side = load_sidecar()
    items = []
    for root, dirs, files in os.walk(LIBRARY):
        dirs[:] = [d for d in dirs if not d.startswith(".")]
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
            ov = side.get(rel, {})
            items.append({
                "rel": rel,
                "file": f,
                "size": size,
                "year": str(ov.get("year") or year),
                "author": ov.get("author") or author,
                "title": ov.get("title") or title,
                "tags": norm_tags(ov.get("tags")),
                "folder": os.path.dirname(rel),
            })
    items.sort(key=lambda d: (d["year"], d["title"].lower()), reverse=True)
    return items


def safe_path(rel: str):
    rel = urllib.parse.unquote(rel)
    full = os.path.realpath(os.path.join(LIBRARY, rel))
    if full != LIBRARY and not full.startswith(LIBRARY + os.sep):
        return None
    if not os.path.isfile(full) or not full.lower().endswith(".pdf"):
        return None
    return full


def sanitize_name(name: str) -> str:
    name = os.path.basename(name.strip()).replace("\\", "").replace("/", "")
    name = re.sub(r'[<>:"|?*\x00-\x1f]', "", name)
    if not name:
        return ""
    if not name.lower().endswith(".pdf"):
        name += ".pdf"
    return name


def thumb_file(rel: str) -> str:
    h = hashlib.sha1(rel.encode("utf-8")).hexdigest()
    return os.path.join(THUMB_DIR, h + ".png")


def make_thumb(full: str, out: str) -> bool:
    if fitz is None:
        return False
    try:
        os.makedirs(THUMB_DIR, exist_ok=True)
        doc = fitz.open(full)
        page = doc.load_page(0)
        zoom = min(1.2, 300.0 / max(1.0, page.rect.width))
        pix = page.get_pixmap(matrix=fitz.Matrix(zoom, zoom), alpha=False)
        pix.save(out)
        doc.close()
        return True
    except Exception:
        return False


# --------------------------------------------------------------------------- #
# HTML
# --------------------------------------------------------------------------- #
PAGE = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Library</title>
<style>
:root{{--bg:#F0EEE6;--surface:#FBFAF7;--ink:#181614;--muted:#73706A;--accent:#C9613E;--line:#E4E0D6;}}
*{{box-sizing:border-box;}}
body{{margin:0;background:var(--bg);color:var(--ink);font-family:-apple-system,"Segoe UI",Helvetica,Arial,sans-serif;line-height:1.5;-webkit-font-smoothing:antialiased;}}
.wrap{{max-width:1040px;margin:0 auto;padding:48px 24px 96px;}}
header h1{{font-family:Georgia,"Times New Roman",serif;font-weight:600;font-size:38px;margin:0 0 4px;letter-spacing:-.5px;}}
header p{{margin:0;color:var(--muted);font-size:15px;}}
.controls{{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin:26px 0 6px;}}
.search{{flex:1 1 240px;padding:12px 15px;font-size:15px;background:var(--surface);border:1px solid var(--line);border-radius:10px;color:var(--ink);outline:none;}}
.search:focus{{border-color:var(--accent);}}
select,.toggle button{{padding:11px 13px;font-size:14px;background:var(--surface);border:1px solid var(--line);border-radius:10px;color:var(--ink);cursor:pointer;}}
.toggle{{display:inline-flex;border:1px solid var(--line);border-radius:10px;overflow:hidden;}}
.toggle button{{border:none;border-radius:0;background:var(--surface);}}
.toggle button.active{{background:var(--accent);color:#fff;}}
.count{{color:var(--muted);font-size:13px;margin:2px 2px 18px;}}
#list.view-grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(190px,1fr));gap:14px;}}
.item{{position:relative;background:var(--surface);border:1px solid var(--line);border-radius:12px;transition:border-color .12s,transform .12s;}}
.item:hover{{border-color:var(--accent);transform:translateY(-1px);}}
/* list view */
.view-list .item{{display:flex;align-items:baseline;gap:14px;padding:15px 16px;margin-bottom:10px;}}
.view-list .thumb{{display:none;}}
.view-list .year{{flex:0 0 auto;min-width:44px;color:var(--accent);font-weight:600;font-size:14px;font-variant-numeric:tabular-nums;}}
.view-list .body{{flex:1 1 auto;min-width:0;}}
.view-list .size{{flex:0 0 auto;color:var(--muted);font-size:12px;font-variant-numeric:tabular-nums;}}
/* grid view */
.view-grid .open{{display:block;padding:0;}}
.view-grid .thumb{{display:block;width:100%;aspect-ratio:3/4;object-fit:cover;background:#eceae1;border-radius:12px 12px 0 0;border-bottom:1px solid var(--line);}}
.view-grid .body{{padding:10px 12px 12px;}}
.view-grid .year{{color:var(--accent);font-weight:600;font-size:12.5px;}}
.view-grid .size,.view-grid .meta{{display:none;}}
.open{{text-decoration:none;color:inherit;display:contents;}}
.title{{font-size:15.5px;font-weight:500;word-break:break-word;}}
.meta{{color:var(--muted);font-size:13px;margin-top:2px;}}
.tags{{margin-top:6px;display:flex;flex-wrap:wrap;gap:5px;}}
.tag{{font-size:11.5px;color:var(--accent);background:#F3E7E0;border-radius:20px;padding:1px 9px;}}
.edit{{position:absolute;top:8px;right:8px;border:1px solid var(--line);background:var(--surface);color:var(--muted);
  border-radius:8px;font-size:12px;padding:3px 8px;cursor:pointer;opacity:0;transition:opacity .12s;}}
.item:hover .edit{{opacity:1;}}
.empty{{color:var(--muted);padding:40px 0;text-align:center;}}
footer{{color:var(--muted);font-size:12px;margin-top:30px;text-align:center;}}
/* modal */
.backdrop{{position:fixed;inset:0;background:rgba(24,22,20,.34);display:none;align-items:center;justify-content:center;padding:20px;z-index:9;}}
.backdrop.show{{display:flex;}}
.modal{{background:var(--surface);border:1px solid var(--line);border-radius:16px;width:100%;max-width:460px;padding:22px 22px 18px;}}
.modal h3{{margin:0 0 14px;font-family:Georgia,serif;font-size:20px;}}
.modal label{{display:block;font-size:12.5px;color:var(--muted);margin:11px 0 4px;}}
.modal input{{width:100%;padding:9px 11px;font-size:14px;border:1px solid var(--line);border-radius:9px;background:#fff;color:var(--ink);outline:none;}}
.modal input:focus{{border-color:var(--accent);}}
.row2{{display:flex;gap:10px;}}.row2>div{{flex:1;}}
.actions{{display:flex;justify-content:flex-end;gap:10px;margin-top:18px;}}
.btn{{padding:9px 15px;font-size:14px;border-radius:9px;border:1px solid var(--line);background:var(--surface);color:var(--ink);cursor:pointer;}}
.btn.primary{{background:var(--accent);border-color:var(--accent);color:#fff;}}
</style></head>
<body>
<div class="wrap">
  <header><h1>Library</h1><p>{count} documents · {total}</p></header>
  <div class="controls">
    <input id="q" class="search" type="search" placeholder="Search title, author, year, #tag…" autofocus>
    <select id="sort">
      <option value="year-desc">Year (new → old)</option>
      <option value="year-asc">Year (old → new)</option>
      <option value="title-asc">Title (A → Z)</option>
      <option value="title-desc">Title (Z → A)</option>
    </select>
    <div class="toggle">
      <button id="btnList" class="active" type="button">List</button>
      <button id="btnGrid" type="button">Grid</button>
    </div>
  </div>
  <div id="count" class="count"></div>
  <div id="list" class="view-list">{rows}</div>
  <div id="empty" class="empty" style="display:none">No documents match.</div>
  <footer>library-portal · served live from <code>library/</code></footer>
</div>

<div id="backdrop" class="backdrop">
  <div class="modal">
    <h3>Edit document</h3>
    <input type="hidden" id="m_rel">
    <label>Filename</label><input id="m_file" type="text">
    <div class="row2">
      <div><label>Year</label><input id="m_year" type="text"></div>
      <div><label>Author letters</label><input id="m_author" type="text"></div>
    </div>
    <label>Title</label><input id="m_title" type="text">
    <label>Tags (space or comma separated, e.g. read to-read favourite)</label>
    <input id="m_tags" type="text">
    <div class="actions">
      <button class="btn" type="button" onclick="closeModal()">Cancel</button>
      <button class="btn primary" type="button" onclick="saveModal()">Save</button>
    </div>
  </div>
</div>

<script>
const listEl=document.getElementById('list');
const q=document.getElementById('q'), sortEl=document.getElementById('sort');
const countEl=document.getElementById('count'), emptyEl=document.getElementById('empty');
let items=Array.from(document.querySelectorAll('.item'));

function apply(){{
  const term=q.value.trim().toLowerCase();
  let shown=0;
  for(const el of items){{
    const hit=!term||el.dataset.search.includes(term);
    el.style.display=hit?'':'none'; if(hit)shown++;
  }}
  countEl.textContent=shown+(shown===1?' result':' results');
  emptyEl.style.display=shown?'none':'';
}}
function sortItems(){{
  const v=sortEl.value;
  items.sort((a,b)=>{{
    if(v.startsWith('year')){{
      const r=(a.dataset.year||'').localeCompare(b.dataset.year||'');
      return v==='year-asc'?r:-r;
    }} else {{
      const r=(a.dataset.title||'').localeCompare(b.dataset.title||'');
      return v==='title-asc'?r:-r;
    }}
  }});
  for(const el of items) listEl.appendChild(el);
}}
function setView(grid){{
  listEl.className=grid?'view-grid':'view-list';
  document.getElementById('btnGrid').classList.toggle('active',grid);
  document.getElementById('btnList').classList.toggle('active',!grid);
  localStorage.setItem('lp_view',grid?'grid':'list');
}}
q.addEventListener('input',apply);
sortEl.addEventListener('change',()=>{{sortItems();apply();}});
document.getElementById('btnGrid').onclick=()=>setView(true);
document.getElementById('btnList').onclick=()=>setView(false);

function openModal(el){{
  document.getElementById('m_rel').value=el.dataset.rel;
  document.getElementById('m_file').value=el.dataset.file;
  document.getElementById('m_year').value=el.dataset.year;
  document.getElementById('m_author').value=el.dataset.author;
  document.getElementById('m_title').value=el.dataset.titleRaw;
  document.getElementById('m_tags').value=el.dataset.tags;
  document.getElementById('backdrop').classList.add('show');
}}
function closeModal(){{document.getElementById('backdrop').classList.remove('show');}}
async function saveModal(){{
  const payload={{
    rel:document.getElementById('m_rel').value,
    newname:document.getElementById('m_file').value,
    year:document.getElementById('m_year').value,
    author:document.getElementById('m_author').value,
    title:document.getElementById('m_title').value,
    tags:document.getElementById('m_tags').value,
  }};
  const r=await fetch('/api/save',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(payload)}});
  if(r.ok){{ location.reload(); }}
  else {{ alert('Save failed: '+(await r.text())); }}
}}
document.querySelectorAll('.edit').forEach(b=>b.onclick=e=>{{e.preventDefault();e.stopPropagation();openModal(b.closest('.item'));}});
document.getElementById('backdrop').addEventListener('click',e=>{{if(e.target.id==='backdrop')closeModal();}});

// init
sortItems();
if(localStorage.getItem('lp_view')==='grid')setView(true);
apply();
</script>
</body></html>"""


def render_index() -> str:
    items = list_pdfs()
    total = human_size(sum(i["size"] for i in items))
    rows = []
    for it in items:
        tags = it["tags"]
        search = " ".join([it["title"], it["author"], it["year"], it["file"]]
                          + ["#" + t for t in tags]).lower()
        meta_bits = []
        if it["author"]:
            meta_bits.append(it["author"])
        if it["folder"] and it["folder"] != "pdfs":
            meta_bits.append(it["folder"])
        meta = html.escape(" · ".join(meta_bits))
        href = "/view/" + urllib.parse.quote(it["rel"])
        thumb = "/thumb/" + urllib.parse.quote(it["rel"])
        tag_html = "".join(f'<span class="tag">#{html.escape(t)}</span>' for t in tags)
        a = lambda s: html.escape(s, quote=True)
        rows.append(
            f'<div class="item" data-rel="{a(it["rel"])}" data-file="{a(it["file"])}" '
            f'data-title="{a(it["title"].lower())}" data-title-raw="{a(it["title"])}" '
            f'data-year="{a(it["year"])}" data-author="{a(it["author"])}" '
            f'data-tags="{a(" ".join(tags))}" data-search="{a(search)}">'
            f'<button class="edit" type="button">Edit</button>'
            f'<a class="open" href="{href}" target="_blank" rel="noopener">'
            f'<img class="thumb" loading="lazy" src="{thumb}" alt="">'
            f'<div class="year">{html.escape(it["year"]) or "&nbsp;"}</div>'
            f'<div class="body"><div class="title">{html.escape(it["title"])}</div>'
            f'<div class="meta">{meta}</div>'
            f'<div class="tags">{tag_html}</div></div>'
            f'<div class="size">{human_size(it["size"])}</div></a></div>'
        )
    body = "\n".join(rows) if rows else '<div class="empty">No PDFs found in library/.</div>'
    return PAGE.format(count=len(items), total=total, rows=body)


# --------------------------------------------------------------------------- #
# server
# --------------------------------------------------------------------------- #
class Handler(BaseHTTPRequestHandler):
    server_version = "library-portal"

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
            self._send(200, render_index())
        elif path == "/health":
            self._send(200, b"ok", "text/plain; charset=utf-8")
        elif path.startswith("/view/"):
            full = safe_path(path[len("/view/"):])
            if not full:
                return self._send(404, b"Not found", "text/plain")
            with open(full, "rb") as fh:
                data = fh.read()
            disp = 'inline; filename="%s"' % os.path.basename(full).replace('"', "")
            self._send(200, data, "application/pdf", {"Content-Disposition": disp})
        elif path.startswith("/thumb/"):
            rel = path[len("/thumb/"):]
            full = safe_path(rel)
            if not full:
                return self._send(404, b"", "image/png")
            out = thumb_file(urllib.parse.unquote(rel))
            try:
                fresh = os.path.exists(out) and os.path.getmtime(out) >= os.path.getmtime(full)
            except OSError:
                fresh = False
            if not fresh and not make_thumb(full, out):
                return self._send(404, b"", "image/png")
            with open(out, "rb") as fh:
                self._send(200, fh.read(), "image/png", {"Cache-Control": "max-age=86400"})
        else:
            self._send(404, b"Not found", "text/plain")

    def do_POST(self):
        if self.path != "/api/save":
            return self._send(404, b"Not found", "text/plain")
        try:
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length) or b"{}")
        except Exception:
            return self._json(400, {"error": "bad request"})

        rel = payload.get("rel", "")
        full = safe_path(rel)
        if not full:
            return self._json(404, {"error": "not found"})

        with _LOCK:
            side = load_sidecar()
            newrel = rel
            newname = sanitize_name(payload.get("newname", "") or os.path.basename(rel))
            if newname and newname != os.path.basename(full):
                target = os.path.join(os.path.dirname(full), newname)
                if os.path.exists(target):
                    return self._json(409, {"error": "a file with that name already exists"})
                try:
                    os.rename(full, target)
                except OSError as exc:
                    return self._json(500, {"error": f"rename failed: {exc}"})
                newrel = os.path.relpath(target, LIBRARY).replace(os.sep, "/")
                if rel in side:
                    side[newrel] = side.pop(rel)
                old_thumb = thumb_file(rel)
                if os.path.exists(old_thumb):
                    try:
                        os.remove(old_thumb)
                    except OSError:
                        pass

            entry = side.get(newrel, {})
            for key in ("title", "author", "year"):
                val = str(payload.get(key, "")).strip()
                if val:
                    entry[key] = val
                else:
                    entry.pop(key, None)
            tags = norm_tags(payload.get("tags"))
            if tags:
                entry["tags"] = tags
            else:
                entry.pop("tags", None)
            if entry:
                side[newrel] = entry
            else:
                side.pop(newrel, None)
            save_sidecar(side)

        self._json(200, {"ok": True, "rel": newrel})

    def log_message(self, *args):
        pass


def main():
    engine = "with thumbnails" if fitz else "no thumbnail engine"
    print(f"[library-portal] serving {LIBRARY} on http://0.0.0.0:{PORT} ({engine})", flush=True)
    ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
