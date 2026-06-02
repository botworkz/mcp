# Containers

`botworkz/mcp` builds one container image:

- `mcp-echo`: Rust [`rmcp`](https://crates.io/crates/rmcp) echo server image.

## Building the container image with Earthly (EarthBuild)

This repository uses the maintained EarthBuild fork ([`EarthBuild/earthbuild`](https://github.com/EarthBuild/earthbuild)), not the sunset upstream `earthly/earthly`.

Install the pinned `v0.8.17` binary locally with checksum verification:

```bash
tmp="$(mktemp -d)"
base="https://github.com/EarthBuild/earthbuild/releases/download/v0.8.17"
curl -fsSL -o "${tmp}/earth-linux-amd64" "${base}/earth-linux-amd64"
curl -fsSL -o "${tmp}/checksum.asc" "${base}/checksum.asc"
( cd "${tmp}" && grep ' earth-linux-amd64$' checksum.asc | sha256sum -c - )
chmod +x "${tmp}/earth-linux-amd64"
install -m 0755 "${tmp}/earth-linux-amd64" /usr/local/bin/earthly
earthly bootstrap
```

Build the image locally with:

```bash
earthly +mcp-echo-image
```

This uses the repository root as the Docker build context, matching `docker build -f containers/mcp-echo/Dockerfile .`, and produces `botwork/mcp-echo:local`.

To build every EarthBuild image target in this repository, run `earthly +images`.

The `+mcp-echo-image` target name and `botwork/mcp-echo:local` tag are a stable contract: sibling/local builds in `botworkz/vm` consumes this target via `FROM ../mcp+mcp-echo-image`.

## Produce tarballs

If you still want Make-based convenience targets, `containers/Makefile` now routes image builds through EarthBuild before exporting tarballs:

```bash
make -C containers tarballs
```

That writes `containers/dist/<svc>.tar`, which consumers can load with `docker load`.

## GHCR namespace

Published images live at:

```text
ghcr.io/botworkz/mcp/<svc>
```
