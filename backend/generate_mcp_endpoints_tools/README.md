## MCP Tools Generator

The `generate_mcp_tools.py` script parses the OpenAPI specification and generates Rust code for MCP (Model Context Protocol) tools.

### Setup

```bash
cd backend/generate_mcp_endpoints_tools
pip install -r requirements.txt
```

### Usage

```bash
python3 generate_mcp_tools.py
```

The script will:
1. Parse `backend/windmill-api/openapi.yaml`
2. Find all endpoints marked with `x-mcp-tool: true`
3. Generate `backend/windmill-api/src/mcp_tools.rs` with a const array of tools

### Adding MCP Tools

To mark an endpoint as an MCP tool, add `x-mcp-tool: true` to the operation in the OpenAPI spec. You can also add `x-mcp-instructions` to complete the description of the tool with instructions on how to correctly use the endpoint:

```yaml
/w/{workspace}/scripts/list:
  get:
    x-mcp-tool: true
    x-mcp-instructions: you should call that with this or that arg
    summary: list scripts in workspace
    operationId: listScripts
    # ... rest of endpoint definition
```

### Body fields the agent never sees

`x-mcp-tool-body-constants` sets body fields on every call to the tool. They stay out of
the tool's input schema, so use it for a field whose only correct value over MCP is fixed —
exposing such a field earns nothing and invites a model to fill it with a placeholder the
API then rejects. A constant must name a real body property and must not also appear in
`x-mcp-tool-include-fields`; the generator fails otherwise.

```yaml
/w/{workspace}/scripts/create:
  post:
    x-mcp-tool: true
    x-mcp-tool-body-constants:
      auto_parent: true
```