# media-processor Docker

Professional media-processor container with:

- Web UI on port 8085
- API on port 8086
- Health endpoint at /health

## Start from vn (Windows)

Use the TUI action:

- vn run win11-open-media-processor

This script builds the image, starts the container, waits for health, and opens the browser.

## Manual run

From repository root:

```bash
docker build -t vecnode-media-processor:latest -f docker/media-processor/Dockerfile .
docker rm -f vecnode-media-processor 2>/dev/null || true
docker run -d --rm --name vecnode-media-processor -p 8085:8085 -p 8086:8086 vecnode-media-processor:latest
```

## URLs

- UI: http://localhost:8085
- API: http://localhost:8086
- Health: http://localhost:8086/health

## Operations

```bash
docker logs -f vecnode-media-processor
docker stop vecnode-media-processor
docker ps --filter "name=vecnode-media-processor"
```

## Troubleshooting

- If Docker is not running, start Docker Desktop and retry.
- If ports are busy, free 8085/8086 or adjust mappings in scripts/win11/open_media_processor.bat.
