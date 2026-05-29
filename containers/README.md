# Containers

`botworkz/mcp` builds container images:

- `mcp-echo`: Rust [`rmcp`](https://crates.io/crates/rmcp) echo server image.
- `mcp-glint`: Rust MCP server wrapping the `glint` whitespace linter from `envoyproxy/toolshed`.

## Build locally

Build every image locally with:

```bash
make -C containers containers
```

This produces `botwork/<svc>:local` for all services.

## Produce tarballs

Downstream consumers can export the locally built images as tarballs with:

```bash
make -C containers tarballs
```

That writes `containers/dist/<svc>.tar`, which consumers can load with `docker load`.

## GHCR namespace

Published images live at:

```text
ghcr.io/botworkz/mcp/<svc>
```
