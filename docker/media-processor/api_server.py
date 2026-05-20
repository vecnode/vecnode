from __future__ import annotations

import base64
import html
import io
import os
import re
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
    return await convert_markdown_to_pdf(files=files, paths=paths, mode="latex")


@app.post("/pandoc/markdown-to-pdf-viewer")
async def pandoc_markdown_to_pdf_viewer(
    files: list[UploadFile] = File(...),
    paths: list[str] = Form(default=[]),
) -> dict[str, object]:
    return await convert_markdown_to_pdf(files=files, paths=paths, mode="viewer")


@app.post("/pandoc/markdown-to-reveal")
async def pandoc_markdown_to_reveal(
    files: list[UploadFile] = File(...),
    paths: list[str] = Form(default=[]),
) -> dict[str, object]:
    if not files:
        raise HTTPException(status_code=400, detail="No files received.")

    started_at = time.perf_counter()

    output_base, output_note = get_output_base_dir()
    timestamp = datetime.now().strftime("%Y-%m-%d-%H-%M-%S")
    output_dir = output_base / f"reveal-{timestamp}"
    output_dir.mkdir(parents=True, exist_ok=False)

    generated_files: list[str] = []
    uploaded_count = 0

    with tempfile.TemporaryDirectory(prefix="pandoc-reveal-") as temp_root:
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
            input_path.write_bytes(prepare_markdown_for_reveal(raw))

            output_html_rel = safe_relative.with_suffix(".html")
            output_html = output_dir / output_html_rel
            output_html.parent.mkdir(parents=True, exist_ok=True)

            try:
                run_markdown_reveal_command(input_path=input_path, output_html=output_html)
            except subprocess.CalledProcessError as exc:
                output_text = (exc.output or "").strip() or str(exc)
                raise HTTPException(
                    status_code=500,
                    detail=f"Pandoc Reveal.js build failed for {safe_relative.as_posix()}: {output_text}",
                ) from exc

            generated_files.append(output_html_rel.as_posix())

    if not generated_files:
        try:
            output_dir.rmdir()
        except OSError:
            pass
        raise HTTPException(status_code=400, detail="No Markdown files found (.md or .markdown).")

    presentation_urls = build_presentation_urls(output_base=output_base, output_dir=output_dir, generated_files=generated_files)

    return {
        "uploaded_count": uploaded_count,
        "generated_count": len(generated_files),
        "output_folder": str(output_dir),
        "generated_files": generated_files,
        "presentation_urls": presentation_urls,
        "duration_seconds": round(time.perf_counter() - started_at, 2),
        "note": output_note,
    }


@app.post("/pandoc/reveal-from-markdown")
def pandoc_reveal_from_markdown() -> dict[str, object]:
    started_at = time.perf_counter()

    template_file = Path(__file__).resolve().parent / "default_reveal_presentation.md"
    if not template_file.exists():
        raise HTTPException(
            status_code=500,
            detail=f"Default markdown template not found: {template_file}",
        )

    output_base, output_note = get_output_base_dir()
    timestamp = datetime.now().strftime("%Y-%m-%d-%H-%M-%S")
    output_dir = output_base / f"reveal-{timestamp}"
    output_dir.mkdir(parents=True, exist_ok=False)

    output_html_rel = Path("default-presentation.html")
    output_html = output_dir / output_html_rel

    raw_template = template_file.read_bytes()
    with tempfile.TemporaryDirectory(prefix="pandoc-reveal-template-") as temp_root:
        temp_input = Path(temp_root) / "default_reveal_presentation.md"
        temp_input.write_bytes(prepare_markdown_for_reveal(raw_template))

        try:
            run_markdown_reveal_command(input_path=temp_input, output_html=output_html)
        except subprocess.CalledProcessError as exc:
            output_text = (exc.output or "").strip() or str(exc)
            raise HTTPException(
                status_code=500,
                detail=f"Pandoc Reveal.js build failed for markdown template: {output_text}",
            ) from exc

    generated_files = [output_html_rel.as_posix()]
    presentation_urls = build_presentation_urls(
        output_base=output_base,
        output_dir=output_dir,
        generated_files=generated_files,
    )

    return {
        "template_file": str(template_file),
        "generated_count": len(generated_files),
        "output_folder": str(output_dir),
        "generated_files": generated_files,
        "presentation_urls": presentation_urls,
        "duration_seconds": round(time.perf_counter() - started_at, 2),
        "note": output_note,
    }


async def convert_markdown_to_pdf(
    files: list[UploadFile],
    paths: list[str],
    mode: str,
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
            input_path.write_bytes(normalize_markdown_frontmatter_bytes(raw))

            output_pdf_rel = safe_relative.with_suffix(".pdf")
            output_pdf = output_dir / output_pdf_rel
            output_pdf.parent.mkdir(parents=True, exist_ok=True)

            try:
                run_markdown_pdf_command(input_path=input_path, output_pdf=output_pdf, mode=mode)
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
        "engine": "xelatex",
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


def normalize_markdown_frontmatter_bytes(raw: bytes) -> bytes:
    try:
        text = raw.decode("utf-8-sig")
    except UnicodeDecodeError:
        return raw

    normalized = normalize_yaml_frontmatter(text)
    return normalized.encode("utf-8")


def prepare_markdown_for_reveal(raw: bytes) -> bytes:
    try:
        text = raw.decode("utf-8-sig")
    except UnicodeDecodeError:
        return raw

    normalized = normalize_yaml_frontmatter(text)
    enriched = inject_reveal_title_metadata(normalized)
    return enriched.encode("utf-8")


def normalize_yaml_frontmatter(text: str) -> str:
    lines = text.splitlines(keepends=True)
    if not lines:
        return text

    if lines[0].strip() != "---":
        return text

    end_index = -1
    for i in range(1, len(lines)):
        marker = lines[i].strip()
        if marker in {"---", "..."}:
            end_index = i
            break

    if end_index == -1:
        return text

    # Quote scalar values like: title: Week 1: Intro (contains colon) so YAML parses safely.
    frontmatter = lines[1:end_index]
    for i, line in enumerate(frontmatter):
        newline = "\n" if line.endswith("\n") else ""
        base = line[:-1] if newline else line

        match = re.match(r"^(\s*[A-Za-z0-9_-]+\s*:\s*)(.+?)\s*$", base)
        if not match:
            continue

        prefix, value = match.group(1), match.group(2).strip()
        if not value:
            continue

        if value[0] in {'"', "'", "[", "{", "|", ">", "!", "&", "*"}:
            continue

        if ":" not in value:
            continue

        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        frontmatter[i] = f'{prefix}"{escaped}"{newline}'

    rebuilt = [lines[0], *frontmatter, *lines[end_index:]]
    return "".join(rebuilt)


def inject_reveal_title_metadata(text: str) -> str:
    lines = text.splitlines(keepends=True)
    if not lines or lines[0].strip() != "---":
        return text

    frontmatter_end = -1
    for i in range(1, len(lines)):
        if lines[i].strip() in {"---", "..."}:
            frontmatter_end = i
            break

    if frontmatter_end == -1:
        return text

    meta_lines = lines[1:frontmatter_end]
    metadata: dict[str, str] = {}
    for line in meta_lines:
        match = re.match(r"^\s*([A-Za-z0-9_-]+)\s*:\s*(.*?)\s*$", line.strip())
        if not match:
            continue
        key = match.group(1).lower()
        value = match.group(2).strip().strip('"').strip("'")
        metadata[key] = value

    ordered_fields = [
        ("author", "Author"),
        ("unit", "Unit"),
        ("course", "Course"),
        ("week", "Week"),
        ("date", "Date"),
    ]
    items = [(label, metadata.get(key, "").strip()) for key, label in ordered_fields]
    items = [(label, value) for label, value in items if value]

    if not items:
        return text

    subtitle_lines: list[str] = []
    for label, value in items:
        subtitle_lines.append(f"<strong>{label}:</strong> {html.escape(value)}")

    subtitle_html = f"<small>{'<br/>'.join(subtitle_lines)}</small>"

    frontmatter = lines[1:frontmatter_end]
    cleaned_frontmatter: list[str] = []
    for line in frontmatter:
        if re.match(r"^\s*subtitle\s*:\s*", line.strip(), flags=re.IGNORECASE):
            continue
        cleaned_frontmatter.append(line)

    escaped_subtitle = subtitle_html.replace("\\", "\\\\").replace('"', '\\"')
    newline = "\n" if lines[frontmatter_end].endswith("\n") else ""
    cleaned_frontmatter.append(f'subtitle: "{escaped_subtitle}"{newline}')

    rebuilt = [lines[0], *cleaned_frontmatter, *lines[frontmatter_end:]]
    return "".join(rebuilt)


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


def get_presentation_base_url() -> str:
    configured = os.environ.get("PRESENTATION_BASE_URL", "").strip()
    if configured:
        return configured.rstrip("/")

    presentation_port = os.environ.get("PRESENTATION_PORT", "8087").strip() or "8087"
    return f"http://localhost:{presentation_port}"


def build_presentation_urls(output_base: Path, output_dir: Path, generated_files: list[str]) -> list[str]:
    base_url = get_presentation_base_url()
    try:
        relative_output_dir = output_dir.relative_to(output_base)
    except ValueError:
        relative_output_dir = Path(output_dir.name)

    base_route = PurePosixPath(relative_output_dir.as_posix())
    urls: list[str] = []

    for generated_file in generated_files:
        route = PurePosixPath(base_route, generated_file)
        urls.append(f"{base_url}/{route.as_posix()}")

    return urls


def run_markdown_pdf_command(input_path: Path, output_pdf: Path, mode: str) -> None:
    if mode == "viewer":
        subprocess.check_output(
            [
                "pandoc",
                str(input_path),
                "--from=gfm",
                "--pdf-engine=xelatex",
                "-V",
                "mainfont=Latin Modern Sans",
                "-V",
                "sansfont=Latin Modern Sans",
                "-V",
                "fontsize=11pt",
                "-V",
                "geometry:margin=1in",
                "-V",
                "colorlinks=true",
                "-V",
                "urlcolor=blue",
                "--highlight-style=tango",
                "-o",
                str(output_pdf),
            ],
            stderr=subprocess.STDOUT,
            text=True,
        )
        return

    subprocess.check_output(
        [
            "pandoc",
            str(input_path),
            "--pdf-engine=xelatex",
            "-o",
            str(output_pdf),
        ],
        stderr=subprocess.STDOUT,
        text=True,
    )


def run_markdown_reveal_command(input_path: Path, output_html: Path) -> None:
    base_args = [
        "pandoc",
        str(input_path),
        "-t",
        "revealjs",
        "-s",
        "-V",
        "revealjs-url=https://unpkg.com/reveal.js@5",
        "-V",
        "theme=white",
        "-V",
        "controls=true",
        "-V",
        "controlsLayout=bottom-right",
        "-V",
        "progress=true",
        "-V",
        "slideNumber=false",
        "-V",
        "showSlideNumber=all",
        "-M",
        "author=",
        "-M",
        "date=",
        "--standalone",
        "-o",
        str(output_html),
    ]

    try:
        subprocess.check_output(
            [*base_args[:-2], "--embed-resources", *base_args[-2:]],
            stderr=subprocess.STDOUT,
            text=True,
        )
        apply_reveal_title_scale(output_html=output_html, scale=0.8)
    except subprocess.CalledProcessError as exc:
        output_text = (exc.output or "").lower()
        if "unknown option --embed-resources" not in output_text:
            raise

        # Compatibility path for older pandoc versions.
        subprocess.check_output(
            [*base_args[:-2], "--self-contained", *base_args[-2:]],
            stderr=subprocess.STDOUT,
            text=True,
        )
        apply_reveal_title_scale(output_html=output_html, scale=0.8)


def apply_reveal_title_scale(output_html: Path, scale: float) -> None:
    try:
        html_text = output_html.read_text(encoding="utf-8")
    except Exception:
        return

    marker = "<!-- vecnode-title-scale -->"
    if marker in html_text:
        return

    style_block = (
        f"{marker}\n"
        "<style>\n"
        f".reveal .slides section h1 {{ font-size: {scale}em; }}\n"
        f".reveal .slides section.title-slide h1 {{ font-size: {scale}em; }}\n"
        "</style>\n"
    )

    if "</head>" in html_text:
        updated = html_text.replace("</head>", style_block + "</head>", 1)
    else:
        updated = style_block + html_text

    output_html.write_text(updated, encoding="utf-8")
