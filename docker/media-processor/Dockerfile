# Alpine Linux
FROM alpine:3.19

# Install required dependencies
RUN apk add --no-cache \
    git \
    curl \
    jq \
    bash \
    ca-certificates \
    pandoc \
    python3 \
    py3-pip \
    yt-dlp \
    ffmpeg

# Install Python web dependencies
RUN python3 -m pip install --no-cache-dir \
    --break-system-packages \
    fastapi==0.115.6 \
    "uvicorn[standard]==0.32.1"

# Set working directory
WORKDIR /app

# Copy local repository into image
COPY . /app

# Normalize script line endings and make scripts executable
RUN sed -i 's/\r$//' /app/docker/media-processor/main.sh \
    /app/docker/media-processor/ui_server.py \
    /app/docker/media-processor/api_server.py && \
    chmod +x /app/docker/media-processor/main.sh && \
    chmod +x scripts/ubuntu22/*.sh

# Web access ports
EXPOSE 8085
EXPOSE 8086

# Run media-processor web services by default.
ENTRYPOINT ["/bin/bash", "/app/docker/media-processor/main.sh"]
