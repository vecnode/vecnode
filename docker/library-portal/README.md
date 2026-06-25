# library-portal

A super-light, read-only web viewer for the repo's `library/` folder.

- Image: built locally as `vecnode-library-portal` from this folder
  (`python:3.12-alpine` + a single stdlib `app.py`, no pip dependencies).
- The build context is **only this folder**, so **no PDFs are baked into the image**.
- At runtime the launcher bind-mounts the repo `library/` **read-only** to `/library`.
  The server walks `/library` on each request and renders an Anthropic-style index of
  every PDF, streaming files inline for the browser's PDF viewer. It never writes or
  copies anything.

Run it from the vecnode TUI **Open** menu (`open-library-portal`) — it builds the image,
starts the container with the read-only mount on **port 8090**, and opens Chrome at
`http://localhost:8090`. Stop with `stop-library-portal`.

Env (set by the image, override if needed): `LIBRARY_DIR=/library`, `PORT=8090`.
