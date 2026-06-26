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
4. the crate's `mcp-package.yaml` (producer-side plugin descriptor — see [`botworkz/space#303`](https://github.com/botworkz/space/issues/303))

## Versioning

The repo-root `/VERSION` file is the source of truth. Each crate inlines
its contents at compile time via `include_str!`, and the published
`botwork-version` crate (cross-repo git dep) formats it into a
`<version>[+<sha>]` string for daemon startup logs.

On dev branches CI populates `BOTWORK_GIT_SHA` from `$GITHUB_SHA`, and
the startup banner emits e.g.:

```
botwork-mcp-echo 0.2.0-dev+abc1234
```

On clean releases the sha is suppressed:

```
botwork-mcp-echo 0.1.3
```

Container images carry the OCI standard labels for introspection
without booting the container:

```bash
docker image inspect ghcr.io/botworkz/mcp/mcp-echo:0.1.3 \
  --format '{{ json .Config.Labels }}' | jq
# {
#   "org.opencontainers.image.revision": "<full-git-sha>",
#   "org.opencontainers.image.source": "https://github.com/botworkz/mcp",
#   "org.opencontainers.image.version": "0.1.3"
# }
```
