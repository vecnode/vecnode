# Lightweight viewer/manager for the vecnode library/ folder.
# Only app.py is copied in (the build context is this folder), so no PDFs ever
# enter the image. The library is bind-mounted at runtime. PyMuPDF is included
# to render first-page thumbnails for the grid view (cached under
# library/.portal/thumbs/). Debian slim is used for reliable PyMuPDF wheels.
FROM python:3.12-slim

RUN pip install --no-cache-dir "pymupdf>=1.24,<2"

WORKDIR /app
COPY app.py /app/app.py

ENV LIBRARY_DIR=/library \
    PORT=8090

EXPOSE 8090

CMD ["python", "app.py"]
