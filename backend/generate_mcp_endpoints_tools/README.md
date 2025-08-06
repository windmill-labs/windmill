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