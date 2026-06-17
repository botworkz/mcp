# botwork-mcp-echo

`mcp-echo` is the baseline MCP plugin for the botworkz stack. It implements an
MCP server with the official `rmcp` SDK and serves Streamable HTTP on
`0.0.0.0:8000` with the MCP endpoint mounted at `/mcp`.

It exposes a single tool, `echo`, which returns an HTTP-echo-style
diagnostic response: the input message together with server metadata and a
sorted snapshot of the process environment captured at startup. The shape
is deliberately useful for stack-level smoke tests:

```json
{
  "message": "<input verbatim>",
  "plugin": "mcp-echo",
  "version": "<cargo pkg version>",
  "env": [
    { "name": "BOTWORK_MCP_CONFIG", "value": "{\"…\":\"…\"}" },
    { "name": "BOTWORK_SECRET_GITHUB_PAT", "value": "<redacted len=40>" }
  ]
}
```

Returned via rmcp's `Json<T>` wrapper, so the value appears in
`structuredContent` on the MCP `CallToolResult`. A text rendering of the
same JSON is also placed in `content[0].text` for clients that haven't
migrated to structured output.

## Redaction rule

Any env var whose name starts with `BOTWORK_SECRET_` is rendered as
`<redacted len=N>` — the *name* is preserved (so tests can assert "the
expected secret made it in"), the *value* is not. The prefix is the
project's canonical contract for secret-bearing env vars and is kept in
sync with `botwork-launcher::validate::is_sensitive_env` and
`botwork-config-broker::registry::SECRET_ENV_PREFIX`.

## Env-snapshot timing

Env is captured exactly once at process startup. Subsequent calls return
that snapshot regardless of any later `setenv` calls in the process. The
smoke-test invariant we care about is "what did the broker inject when
the container was spawned?", which is what startup-env answers.

The snapshot is sorted by name and de-duplicated so two equivalent
invocations produce byte-identical responses regardless of process env
ordering.

This crate is the runtime payload for the repository's `mcp-echo`
container image.
