# Private MCP Server URLs

Windmill validates external MCP server URLs before connecting to them from the
backend or workers. By default, MCP server URLs that target private, loopback,
link-local, or otherwise internal IP ranges are rejected to reduce SSRF risk.

Self-hosted and development deployments can opt in to private/internal MCP
servers by setting:

```bash
ALLOW_PRIVATE_MCP_SERVER_URLS=true
```

`1` is also accepted. When this variable is set, Windmill skips the private URL
SSRF guard for MCP server URLs, including MCP OAuth registration and token
endpoints discovered from server metadata.

Do not enable this in multi-tenant or untrusted workspaces unless backend and
worker network access is isolated appropriately. The setting is instance-wide
and allows workspace-authored MCP resources to reach services available from the
Windmill backend or worker network.
