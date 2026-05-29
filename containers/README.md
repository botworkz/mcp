# Containers

`botworkz/mcp` builds one container image:

- `mcp-echo`: Rust [`rmcp`](https://crates.io/crates/rmcp) echo server image.

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
