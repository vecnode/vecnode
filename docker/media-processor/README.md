# media-processor Docker

Professional media-processor container with:

- Web UI on port 8085
- API on port 8086
- Presentation server on port 8087
- Health endpoint at /health
- Debian 12 slim base image

## Start from vn (Windows)

Use the TUI action:

- vn run win11-open-media-processor

This script builds the image, starts the container, waits for health, and opens the browser.

## Manual run

From repository root:

```bash
docker build -t vecnode-media-processor:latest -f docker/media-processor/Dockerfile .
docker rm -f vecnode-media-processor 2>/dev/null || true
docker run -d --rm --name vecnode-media-processor \
	-p 8085:8085 \
	-p 8086:8086 \
	-p 8087:8087 \
	-e HOST_DESKTOP_DIR=/host/Desktop \
	-v "$HOME/Desktop:/host/Desktop" \
	vecnode-media-processor:latest
```

On Windows PowerShell, use:

```powershell
docker build -t vecnode-media-processor:latest -f docker/media-processor/Dockerfile .
docker rm -f vecnode-media-processor 2>$null
docker run -d --rm --name vecnode-media-processor `
	-p 8085:8085 `
	-p 8086:8086 `
	-p 8087:8087 `
	-e HOST_DESKTOP_DIR=/host/Desktop `
	-v "${env:USERPROFILE}\Desktop:/host/Desktop" `
	vecnode-media-processor:latest
```

## URLs

- UI: http://localhost:8085
- API: http://localhost:8086
- Presentation: http://localhost:8087
- Health: http://localhost:8086/health

## Reveal.js workflow

- Use Pandoc Processor > Markdown to Reveal.js in the UI.
- Generated presentations are saved on the host Desktop in a `reveal-YYYY-MM-DD-HH-MM-SS` folder.
- The API returns localhost links served from port 8087 and the UI opens the first presentation automatically.

## Operations

```bash
docker logs -f vecnode-media-processor
docker stop vecnode-media-processor
docker ps --filter "name=vecnode-media-processor"
```

## Troubleshooting

- If Docker is not running, start Docker Desktop and retry.
- If ports are busy, free 8085/8086 or adjust mappings in scripts/win11/open_media_processor.bat.
