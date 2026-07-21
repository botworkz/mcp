# mcp

This repository hosts MCP servers used by [`phlax/botspace`](https://github.com/phlax/botspace).

- `botwork-mcp-echo/`: Rust `rmcp` Streamable HTTP echo server (`mcp-echo`).

## Building container images

Build the `echo` service image locally with:

```bash
docker build -f echo/Dockerfile -t botwork/mcp-echo:local .
```

To pass build args (e.g. for a tagged release build):

```bash
docker buildx build \
  --platform linux/amd64 \
  --build-arg BINARY_SOURCE=source \
  -t botwork/mcp-echo:local \
  -f echo/Dockerfile \
  .
```

Published images live in:

```text
ghcr.io/botworkz/mcp/<svc>
```

## Adding a new crate

When adding a new crate/containerized service, update:

1. the crate directory and `<crate>/Dockerfile`
2. the crate matrix and publish loop in `.github/workflows/ci.yml`

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
