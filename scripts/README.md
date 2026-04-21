
# vecnode CLI

vecnode-cli = host/orchestration CLI.  
tools-cli = in-container tools workflow CLI (pandoc/python/yt-dlp workflows).

### Run

```bat
# Ubuntu 22.04
./scripts/ubuntu22/main.sh

# Windows 11
.\scripts\win11\main.bat
```

### CLI Dependencies

- curl
- git
- jq
- docker

Docker Alpine Dependencies

- pandoc
- python
- yt-dlp

### Silverbullet
```bash
# Stop container
docker stop silverbullet-local
```

