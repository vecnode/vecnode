# Debian 12 slim
FROM debian:12-slim

# Install required dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    jq \
    bash \
    ca-certificates \
    pandoc \
    lmodern \
    texlive-latex-recommended \
    texlive-fonts-recommended \
    texlive-xetex \
    python3 \
    python3-pip \
    yt-dlp \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Install Python web dependencies
RUN python3 -m pip install --no-cache-dir \
    --break-system-packages \
    fastapi==0.115.6 \
    "uvicorn[standard]==0.32.1" \
    pillow==11.2.1 \
    python-multipart==0.0.20

# Set working directory
WORKDIR /app

# Copy local repository into image
COPY . /app

# Normalize script line endings and make scripts executable
RUN sed -i 's/\r$//' /app/docker/media-processor/main.sh \
    /app/docker/media-processor/ui_server.py \
    /app/docker/media-processor/api_server.py && \
    cp /app/assets/icons/black-square.svg /app/docker/media-processor/ui/favicon.svg && \
    chmod +x /app/docker/media-processor/main.sh && \
    chmod +x scripts/ubuntu22/*.sh

# Web access ports
EXPOSE 8085
EXPOSE 8086

# Run media-processor web services by default.
ENTRYPOINT ["/bin/bash", "/app/docker/media-processor/main.sh"]
