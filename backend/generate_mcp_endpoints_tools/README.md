# Backend Scripts

This directory contains utility scripts for the Windmill backend.

## MCP Tools Generator

The `generate_mcp_tools.py` script parses the OpenAPI specification and generates Rust code for MCP (Model Context Protocol) tools.

### Setup

```bash
cd backend/scripts
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

To mark an endpoint as an MCP tool, add `x-mcp-tool: true` to the operation in the OpenAPI spec:

```yaml
/w/{workspace}/scripts/list:
  get:
    x-mcp-tool: true
    summary: list scripts in workspace
    operationId: listScripts
    # ... rest of endpoint definition
```