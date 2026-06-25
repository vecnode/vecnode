# Super-light read-only viewer for the vecnode library/ folder.
# Only app.py is copied in (the build context is this folder), so no PDFs ever
# enter the image. The library is bind-mounted read-only at runtime.
FROM python:3.12-alpine

WORKDIR /app
COPY app.py /app/app.py

ENV LIBRARY_DIR=/library \
    PORT=8090

EXPOSE 8090

# Pure stdlib server — no pip install needed.
CMD ["python", "app.py"]
