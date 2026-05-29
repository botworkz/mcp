# botwork-mcp-glint

It implements an MCP server with the official `rmcp` SDK.
The server exposes `glint_check` and `glint_fix` tools that wrap the
`glint` binary produced by `envoyproxy/toolshed`.
It serves Streamable HTTP on `0.0.0.0:8000` with the MCP endpoint mounted at `/mcp`.
The `glint` binary is resolved from `$GLINT_BIN` (default: `glint` on `$PATH`).
`glint_fix` returns `no changes` when glint emits empty stdout.
This crate is the runtime payload for the repository's `mcp-glint` container image.
