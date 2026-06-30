# Debian 12 slim
FROM debian:12-slim

# ffmpeg (audio/video conversion) + python3. yt-dlp comes from pip, not apt:
# the Debian apt package is years stale and breaks against current sites; pip
# always installs the latest extractor set at build time.
RUN apt-get update -o Acquire::Retries=5 && \
    apt-get install -y --no-install-recommends --fix-missing \
    -o Acquire::Retries=5 \
    ca-certificates \
    ffmpeg \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

RUN python3 -m pip install --no-cache-dir --break-system-packages -U yt-dlp

WORKDIR /app
COPY app.py /app/app.py

EXPOSE 8095
ENV PORT=8095

ENTRYPOINT ["python3", "/app/app.py"]
