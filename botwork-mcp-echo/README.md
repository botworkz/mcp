# botwork-mcp-echo

`botwork-mcp-echo` is a Rust rewrite of the former Python `botspace_mcp_echo` service.
It implements an MCP server with the official `rmcp` SDK.
The server exposes a single `echo` tool that returns the input message unchanged.
It serves Streamable HTTP on `0.0.0.0:8000` with the MCP endpoint mounted at `/mcp`.
This crate is the runtime payload for the repository's `mcp-echo` container image.
