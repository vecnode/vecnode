# vecnode Docs

This folder is a small Markdown book for the vecnode CLI.

It starts with two pages:

- This overview page
- The TUI button guide

The book can be served with the Dockerfile in this folder.

```bash
docker build -t vecnode-docs -f docs/Dockerfile docs
docker run --rm -p 3000:3000 vecnode-docs
```

Open the local site at `http://localhost:3000`.