from __future__ import annotations

import subprocess
from datetime import datetime, timezone
from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

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


@app.post("/process")
def process() -> dict[str, object]:
    workspace = Path("/app")
    return {
        "status": "accepted",
        "message": "Processing endpoint is ready for job wiring.",
        "workspace": str(workspace),
    }


def run_version(command: list[str]) -> str:
    try:
        output = subprocess.check_output(command, stderr=subprocess.STDOUT, text=True)
        first_line = output.splitlines()[0] if output else "unknown"
        return first_line.strip()
    except Exception:
        return "unavailable"
