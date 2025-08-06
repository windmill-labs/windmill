#!/usr/bin/env python3
"""
Script to parse the OpenAPI YAML file and generate Rust code with MCP tools.
Searches for endpoints tagged with 'x-mcp-tool: true' and creates a const array.
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Any, Optional

def load_openapi_spec(file_path: str) -> Dict[str, Any]:
    """Load and parse the OpenAPI YAML specification."""
    try:
        import yaml
        with open(file_path, 'r', encoding='utf-8') as f:
            return yaml.safe_load(f)
    except ImportError:
        print("PyYAML not found. Please install it with: pip install PyYAML", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error loading OpenAPI spec: {e}", file=sys.stderr)
        sys.exit(1)

def extract_separate_schemas(parameters: List[Dict[str, Any]], request_body: Optional[Dict[str, Any]], spec: Dict[str, Any]) -> tuple:
    """Extract separate schemas for path parameters, query parameters, and request body."""
    path_params_schema = {
        "type": "object",
        "properties": {},
        "required": []
    }
    
    query_params_schema = {
        "type": "object", 
        "properties": {},
        "required": []
    }
    
    body_schema = None
    
    # Process parameters
    for param in parameters:
        # Resolve $ref if present
        if '$ref' in param:
            param = resolve_schema_refs(param, spec)
        
        param_name = param.get('name', '')
        param_schema = param.get('schema', {'type': 'string'})
        param_required = param.get('required', False)
        param_description = param.get('description', '')
        param_in = param.get('in', 'query')
        
        # Resolve any refs in the parameter schema
        param_schema = resolve_schema_refs(param_schema, spec)
        
        # Add description if available
        if param_description:
            param_schema = dict(param_schema)
            param_schema['description'] = param_description
        
        # Route to appropriate schema based on parameter location
        if param_in == 'path':
            # Skip 'workspace' path parameter as it's automatically provided by the MCP context
            if param_name != 'workspace':
                path_params_schema['properties'][param_name] = param_schema
                if param_required:
                    path_params_schema['required'].append(param_name)
        elif param_in == 'query':
            query_params_schema['properties'][param_name] = param_schema
            if param_required:
                query_params_schema['required'].append(param_name)
    
    # Process request body if present
    if request_body:
        body_schema = extract_request_body_schema(request_body, spec)
    
    # Return None for empty schemas
    path_params_schema = path_params_schema if path_params_schema['properties'] else None
    query_params_schema = query_params_schema if query_params_schema['properties'] else None
    
    return (path_params_schema, query_params_schema, body_schema)

def resolve_ref(ref_path: str, spec: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Resolve a $ref path to the actual schema definition."""
    if not ref_path.startswith('#/'):
        return None
    
    # Remove the '#/' prefix and split by '/'
    path_parts = ref_path[2:].split('/')
    
    # Navigate through the spec following the path
    current = spec
    for part in path_parts:
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None
    
    return current if isinstance(current, dict) else None

def resolve_schema_refs(schema: Dict[str, Any], spec: Dict[str, Any]) -> Dict[str, Any]:
    """Recursively resolve all $ref references in a schema."""
    if not isinstance(schema, dict):
        return schema
    
    # If this is a $ref, resolve it
    if '$ref' in schema:
        ref_path = schema['$ref']
        resolved = resolve_ref(ref_path, spec)
        if resolved:
            # Recursively resolve any refs in the resolved schema
            return resolve_schema_refs(resolved, spec)
        else:
            print(f"Warning: Could not resolve $ref: {ref_path}")
            return schema
    
    # Recursively process all values in the schema
    resolved_schema = {}
    for key, value in schema.items():
        if isinstance(value, dict):
            resolved_schema[key] = resolve_schema_refs(value, spec)
        elif isinstance(value, list):
            resolved_schema[key] = [
                resolve_schema_refs(item, spec) if isinstance(item, dict) else item
                for item in value
            ]
        else:
            resolved_schema[key] = value
    
    return resolved_schema

def extract_request_body_schema(request_body: Dict[str, Any], spec: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Extract request body schema from OpenAPI requestBody definition and resolve refs."""
    if not request_body:
        return None
    
    content = request_body.get('content', {})
    json_content = content.get('application/json', {})
    schema = json_content.get('schema', {})
    
    if schema:
        # Resolve any $ref references in the schema
        return resolve_schema_refs(schema, spec)
    
    return None

def http_method_to_rust(method: str) -> str:
    """Convert HTTP method string to Rust http::Method enum."""
    method_map = {
        'get': 'http::Method::GET',
        'post': 'http::Method::POST',
        'put': 'http::Method::PUT',
        'delete': 'http::Method::DELETE',
        'patch': 'http::Method::PATCH',
        'head': 'http::Method::HEAD',
        'options': 'http::Method::OPTIONS'
    }
    return method_map.get(method.lower(), f'http::Method::{method.upper()}')

def schema_to_rust_value(schema: Optional[Dict[str, Any]]) -> str:
    """Convert a schema dict to a Rust serde_json::json! expression."""
    if schema is None:
        return "None"
    return f"Some(serde_json::json!({json.dumps(schema, indent=8)}))"

def find_mcp_tools(spec: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Find all endpoints marked with x-mcp-tool: true."""
    tools = []
    paths = spec.get('paths', {})
    
    for path, path_item in paths.items():
        for method, operation in path_item.items():
            if isinstance(operation, dict) and operation.get('x-mcp-tool') is True:
                # Extract tool information
                tool = {
                    'name': operation.get('operationId', f"{method}_{path.replace('/', '_').replace('{', '').replace('}', '')}"),
                    'description': operation.get('summary', operation.get('description', f'{method.upper()} {path}')),
                    'instructions': operation.get('x-mcp-instructions', ''),
                    'path': path,
                    'method': method.upper(),
                    'parameters': operation.get('parameters', []),
                    'requestBody': operation.get('requestBody'),
                }
                tools.append(tool)
    
    return tools

def generate_rust_code(tools: List[Dict[str, Any]], spec: Dict[str, Any]) -> str:
    """Generate the complete Rust code with MCP tools."""
    if not tools:
        return """// No MCP tools found in the OpenAPI specification

use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct EndpointTool {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub instructions: Cow<'static, str>,
    pub path: Cow<'static, str>,
    pub method: http::Method,
    pub path_params_schema: Option<serde_json::Value>,
    pub query_params_schema: Option<serde_json::Value>,
    pub body_schema: Option<serde_json::Value>,
}

pub fn all_tools() -> Vec<EndpointTool> {
    vec![]
}
"""
    
    tool_definitions = []
    
    for tool in tools:
        tool_name = tool['name']
        description = tool['description']
        instructions = tool['instructions']
        path = tool['path']
        method = http_method_to_rust(tool['method'])
        
        # Generate separate schemas
        path_params_schema, query_params_schema, body_schema = extract_separate_schemas(
            tool['parameters'], tool['requestBody'], spec
        )
        
        path_params_rust = schema_to_rust_value(path_params_schema)
        query_params_rust = schema_to_rust_value(query_params_schema)
        body_schema_rust = schema_to_rust_value(body_schema)
        
        # Generate tool definition
        tool_def = f"""    EndpointTool {{
        name: Cow::Borrowed("{tool_name}"),
        description: Cow::Borrowed("{description}"),
        instructions: Cow::Borrowed("{instructions}"),
        path: Cow::Borrowed("{path}"),
        method: {method},
        path_params_schema: {path_params_rust},
        query_params_schema: {query_params_rust},
        body_schema: {body_schema_rust},
    }}"""
        tool_definitions.append(tool_def)
    
    # Combine everything
    tool_definitions_str = ",\n".join(tool_definitions)
    
    rust_code = f"""// Auto-generated MCP tools from OpenAPI specification
// This file is generated by generate_mcp_tools.py - DO NOT EDIT MANUALLY

use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct EndpointTool {{
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub instructions: Cow<'static, str>,
    pub path: Cow<'static, str>,
    pub method: http::Method,
    pub path_params_schema: Option<serde_json::Value>,
    pub query_params_schema: Option<serde_json::Value>,
    pub body_schema: Option<serde_json::Value>,
}}

pub fn all_tools() -> Vec<EndpointTool> {{
    vec![
{tool_definitions_str}
    ]
}}
"""
    
    return rust_code

def main():
    """Main function to parse OpenAPI and generate Rust code."""
    script_dir = Path(__file__).parent
    backend_dir = script_dir.parent
    openapi_file = backend_dir / "windmill-api" / "openapi.yaml"
    output_file = backend_dir / "windmill-api" / "src" / "mcp_tools.rs"
    
    if not openapi_file.exists():
        print(f"OpenAPI file not found: {openapi_file}", file=sys.stderr)
        sys.exit(1)
    
    print(f"Loading OpenAPI specification from: {openapi_file}")
    spec = load_openapi_spec(str(openapi_file))
    
    print("Searching for endpoints with x-mcp-tool: true...")
    tools = find_mcp_tools(spec)
    
    if tools:
        print(f"Found {len(tools)} MCP tool(s):")
        for tool in tools:
            print(f"  - {tool['name']}: {tool['method']} {tool['path']}")
    else:
        print("No MCP tools found (no endpoints with x-mcp-tool: true)")
    
    print(f"Generating Rust code...")
    rust_code = generate_rust_code(tools, spec)
    
    print(f"Writing to: {output_file}")
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(rust_code)
    
    print("Done!")

if __name__ == "__main__":
    main()