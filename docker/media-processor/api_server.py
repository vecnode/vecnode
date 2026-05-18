from __future__ import annotations

import base64
import io
import os
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
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


@app.post("/pandoc/markdown-to-pdf")
async def pandoc_markdown_to_pdf(
    files: list[UploadFile] = File(...),
    paths: list[str] = Form(default=[]),
) -> dict[str, object]:
    if not files:
        raise HTTPException(status_code=400, detail="No files received.")

    started_at = time.perf_counter()

    output_base, output_note = get_output_base_dir()
    timestamp = datetime.now().strftime("%Y-%m-%d-%H-%M-%S")
    output_dir = output_base / f"pandoc-{timestamp}"
    output_dir.mkdir(parents=True, exist_ok=False)

    converted_files: list[str] = []
    uploaded_count = 0

    with tempfile.TemporaryDirectory(prefix="pandoc-md-") as temp_root:
        temp_root_path = Path(temp_root)

        for index, upload in enumerate(files):
            raw = await upload.read()
            uploaded_count += 1

            submitted_path = paths[index] if index < len(paths) else upload.filename
            safe_relative = safe_relative_path(submitted_path or upload.filename or f"file-{index}.md")

            if safe_relative.suffix.lower() not in {".md", ".markdown"}:
                continue

            input_path = temp_root_path / safe_relative
            input_path.parent.mkdir(parents=True, exist_ok=True)
            input_path.write_bytes(raw)

            output_pdf_rel = safe_relative.with_suffix(".pdf")
            output_pdf = output_dir / output_pdf_rel
            output_pdf.parent.mkdir(parents=True, exist_ok=True)

            try:
                subprocess.check_output(
                    [
                        "pandoc",
                        str(input_path),
                        "--pdf-engine=tectonic",
                        "-o",
                        str(output_pdf),
                    ],
                    stderr=subprocess.STDOUT,
                    text=True,
                )
            except subprocess.CalledProcessError as exc:
                output_text = (exc.output or "").strip() or str(exc)
                raise HTTPException(
                    status_code=500,
                    detail=f"Pandoc failed for {safe_relative.as_posix()}: {output_text}",
                ) from exc

            converted_files.append(output_pdf_rel.as_posix())

    if not converted_files:
        try:
            output_dir.rmdir()
        except OSError:
            pass
        raise HTTPException(status_code=400, detail="No Markdown files found (.md or .markdown).")

    return {
        "uploaded_count": uploaded_count,
        "converted_count": len(converted_files),
        "output_folder": str(output_dir),
        "converted_files": converted_files,
        "engine": "tectonic",
        "duration_seconds": round(time.perf_counter() - started_at, 2),
        "note": output_note,
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


def safe_relative_path(path_value: str) -> Path:
    candidate = str(path_value or "").replace("\\", "/").strip()
    pure = PurePosixPath(candidate)
    safe_parts = [part for part in pure.parts if part not in {"", ".", ".."}]
    if not safe_parts:
        return Path("input.md")
    return Path(*safe_parts)


def get_output_base_dir() -> tuple[Path, str]:
    configured_host_desktop = os.environ.get("HOST_DESKTOP_DIR", "").strip()
    if configured_host_desktop:
        host_path = Path(configured_host_desktop)
        host_path.mkdir(parents=True, exist_ok=True)
        return host_path, "Saved to HOST_DESKTOP_DIR."

    local_desktop = Path.home() / "Desktop"
    if local_desktop.exists():
        return local_desktop, "Saved to local Desktop."

    fallback = Path("/outputs")
    fallback.mkdir(parents=True, exist_ok=True)
    return (
        fallback,
        "HOST_DESKTOP_DIR is not set; saved to /outputs inside container/runtime.",
    )
