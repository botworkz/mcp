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

## Host / Origin allowlists

By default the rmcp streamable-HTTP transport only accepts requests whose
`Host` header is a loopback authority. In the botwork deployment the
launcher assigns a per-session container name (e.g.
`mcp_session_<id>:8000`) as the forwarded authority, which is not
localhost and would be rejected by the default allowlist.

Two environment variables override the allowlist behaviour:

| Variable | Purpose |
| -------- | ------- |
| `MCP_ALLOWED_HOSTS` | Comma-separated `host` or `host:port` authorities that the `Host` header is validated against. |
| `MCP_ALLOWED_ORIGINS` | Comma-separated origins (with scheme, e.g. `https://app.example.com`) that the `Origin` header is validated against. |

**Default (unset or empty) = bypass (allow all).** When either variable is
unset, empty, or set to the literal `*`, the corresponding allowlist is
empty. rmcp treats an empty list as "allow all" — the DNS-rebinding guard
is bypassed. This is the intentional default behind the botwork edge
(envoy + launcher), because the authority is a per-session launcher-
assigned container name that cannot be predicted in advance.

**Hardening mode** — set either variable to a comma-separated list of the
specific values that should be permitted. Any request whose `Host` or
`Origin` header does not appear in the list is rejected with `403
Forbidden`.

```
# Example: restrict to a known hostname
MCP_ALLOWED_HOSTS=my-plugin.internal:8000

# Comma-separated list
MCP_ALLOWED_HOSTS=my-plugin.internal:8000,localhost

# Explicit bypass (same as unset)
MCP_ALLOWED_HOSTS=*
```

This crate is the runtime payload for the repository's `mcp-echo`
container image.
