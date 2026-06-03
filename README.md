# mcp

This repository hosts MCP servers used by [`phlax/botspace`](https://github.com/phlax/botspace).

- `botwork-mcp-echo/`: Rust `rmcp` Streamable HTTP echo server (`mcp-echo`).

## Building container images with Earthly (EarthBuild)

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

This uses the repository root as the Docker build context (equivalent to `docker build -f echo/Dockerfile .`) and produces `botwork/mcp-echo:local`.

To build every EarthBuild image target in this repository, run:

```bash
earthly +images
```

CI and release builds can reuse a prebuilt crate binary instead of rebuilding inside Docker:

```bash
earthly --push +mcp-echo-image --BINARY_SOURCE=prebuilt --TAG=<version>
```

The `+mcp-echo-image` target name and `botwork/mcp-echo:local` tag are a stable contract: sibling/local builds in `botworkz/vm` consume this target via `FROM ../mcp+mcp-echo-image`.

Published images live in:

```text
ghcr.io/botworkz/mcp/<svc>
```

## Adding a new crate

When adding a new crate/containerized service, update:

1. the crate directory and `<crate>/Dockerfile`
2. the crate matrix and publish loop in `.github/workflows/ci.yml`
3. image targets in `Earthfile` (including `+images`)
