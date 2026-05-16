from __future__ import annotations

import base64
import io
import subprocess
from datetime import datetime, timezone

from fastapi import FastAPI, File, HTTPException, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from PIL import Image

app = FastAPI(title="vecnode media-processor API", version="0.1.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:8085", "http://127.0.0.1:8085"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
def health() -> dict[str, object]:
    return {
        "status": "ok",
        "service": "media-processor-api",
        "time": datetime.now(timezone.utc).isoformat(),
    }


@app.get("/tools")
def tools() -> dict[str, str]:
    return {
        "python3": run_version(["python3", "--version"]),
        "pandoc": run_version(["pandoc", "--version"]),
        "yt_dlp": run_version(["yt-dlp", "--version"]),
        "ffmpeg": run_version(["ffmpeg", "-version"]),
    }


@app.get("/pandoc/version")
def pandoc_version() -> dict[str, str]:
    return {
        "version": run_full_output(["pandoc", "--version"]),
    }


@app.post("/process")
async def process(file: UploadFile = File(...)) -> dict[str, object]:
    if not file.content_type or not file.content_type.startswith("image/"):
        raise HTTPException(status_code=400, detail="Uploaded file must be an image.")

    raw = await file.read()

    try:
        img = Image.open(io.BytesIO(raw))
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Cannot decode image: {exc}") from exc

    width, height = img.size
    mode = img.mode
    fmt = img.format or "unknown"

    # Convert to grayscale and encode as base64 PNG for the browser to display
    gray = img.convert("L")
    buf = io.BytesIO()
    gray.save(buf, format="PNG")
    gray_b64 = base64.b64encode(buf.getvalue()).decode()

    return {
        "filename": file.filename,
        "format": fmt,
        "mode": mode,
        "width": width,
        "height": height,
        "size_bytes": len(raw),
        "grayscale_png_b64": gray_b64,
    }


def run_version(command: list[str]) -> str:
    try:
        output = subprocess.check_output(command, stderr=subprocess.STDOUT, text=True)
        first_line = output.splitlines()[0] if output else "unknown"
        return first_line.strip()
    except Exception:
        return "unavailable"


def run_full_output(command: list[str]) -> str:
    try:
        output = subprocess.check_output(command, stderr=subprocess.STDOUT, text=True)
        return output.strip() if output else "unknown"
    except Exception:
        return "unavailable"
