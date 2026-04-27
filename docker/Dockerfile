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
    yt-dlp

# Set working directory
WORKDIR /app

# Copy local repository into image
COPY . /app

# Make scripts executable
RUN chmod +x scripts/ubuntu22/*.sh scripts/tools-cli/alpine/*.sh

# Run interactive vecnode Tools CLI by default.
ENTRYPOINT ["/bin/bash", "/app/scripts/tools-cli/alpine/main.sh"]
