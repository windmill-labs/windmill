#!/usr/bin/env python3
"""
Script to parse the OpenAPI YAML file and generate Rust and TypeScript code with MCP tools.
Searches for endpoints tagged with 'x-mcp-tool: true' and creates:
- Rust code: A const array in backend/windmill-api/src/mcp/tools/auto_generated_endpoints.rs
- TypeScript code: An exported array in frontend/src/lib/mcpEndpointTools.ts
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Any, Optional

IMPORTS = """
use std::borrow::Cow;
use windmill_mcp::server::EndpointTool;
"""

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

def flatten_allof_schema(schema: Dict[str, Any]) -> Dict[str, Any]:
    """Flatten an allOf schema into a single object schema by merging all properties."""
    if 'allOf' not in schema:
        return schema

    merged = {"type": "object", "properties": {}, "required": []}

    def collect_from(s: Dict[str, Any]):
        if 'allOf' in s:
            for item in s['allOf']:
                if isinstance(item, dict):
                    collect_from(item)
        if 'properties' in s:
            merged['properties'].update(s['properties'])
        if 'required' in s and isinstance(s['required'], list):
            merged['required'].extend(s['required'])
        if 'description' in s and 'description' not in merged:
            merged['description'] = s['description']

    collect_from(schema)

    # Preserve additional top-level keys from the original schema
    preserved_keys = {'additionalProperties', 'title', 'nullable', 'default', 'example'}
    for key in preserved_keys:
        if key in schema and key not in merged:
            merged[key] = schema[key]

    if not merged['required']:
        del merged['required']

    return merged

def extract_separate_schemas(parameters: List[Dict[str, Any]], request_body: Optional[Dict[str, Any]], spec: Dict[str, Any], required_fields: Optional[List[str]] = None, base_path: str = "", include_fields: Optional[List[str]] = None, opaque_fields: Optional[List[str]] = None, include_query_params: Optional[List[str]] = None) -> tuple:
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
            param = resolve_schema_refs(param, spec, base_path)

        param_name = param.get('name', '')
        param_schema = param.get('schema', {'type': 'string'})
        param_required = param.get('required', False)
        param_description = param.get('description', '')
        param_in = param.get('in', 'query')

        # Resolve any refs in the parameter schema
        param_schema = resolve_schema_refs(param_schema, spec, base_path)
        
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
        elif param_in == 'query' and (include_query_params is None or param_name in include_query_params):
            query_params_schema['properties'][param_name] = param_schema
            if param_required:
                query_params_schema['required'].append(param_name)
    
    # Process request body if present
    if request_body:
        body_schema = extract_request_body_schema(request_body, spec, base_path)

        # Flatten allOf schemas into a single object schema for filtering
        if body_schema and (include_fields is not None or opaque_fields):
            body_schema = flatten_allof_schema(body_schema)

        # Apply include_fields filter: only keep listed top-level properties
        if body_schema and include_fields is not None and 'properties' in body_schema:
            body_schema['properties'] = {
                k: v for k, v in body_schema['properties'].items()
                if k in include_fields
            }
            if 'required' in body_schema:
                body_schema['required'] = [
                    r for r in body_schema['required'] if r in include_fields
                ]

        # Apply opaque_fields: simplify listed properties to {"type": "object"}
        if body_schema and opaque_fields and 'properties' in body_schema:
            for field in opaque_fields:
                if field in body_schema['properties']:
                    body_schema['properties'][field] = {"type": "object"}

        # If we have required fields specified and a body schema, update the required array
        if body_schema and required_fields:
            if 'required' not in body_schema:
                body_schema['required'] = []
            
            # Add each required field if it exists in the schema properties
            for field in required_fields:
                if 'properties' in body_schema and field in body_schema['properties']:
                    if field not in body_schema['required']:
                        body_schema['required'].append(field)
                else:
                    # Log warning when a required field is missing from schema properties
                    print(f"Warning: Required field '{field}' not found in body schema properties", file=sys.stderr)
    
    # Sanitize empty schemas for JSON Schema draft 2020-12 compliance
    path_params_schema = sanitize_empty_schemas(path_params_schema)
    query_params_schema = sanitize_empty_schemas(query_params_schema)
    body_schema = sanitize_empty_schemas(body_schema)

    # Convert enums to descriptions for client compatibility
    path_params_schema = convert_enums_to_descriptions(path_params_schema)
    query_params_schema = convert_enums_to_descriptions(query_params_schema)
    body_schema = convert_enums_to_descriptions(body_schema)

    # Detect overlapping property names across schemas and rename with suffixes
    path_keys = set(path_params_schema['properties'].keys()) if path_params_schema and path_params_schema.get('properties') else set()
    query_keys = set(query_params_schema['properties'].keys()) if query_params_schema and query_params_schema.get('properties') else set()
    body_keys = set(body_schema['properties'].keys()) if body_schema and body_schema.get('properties') else set()

    conflicts = (path_keys & query_keys) | (path_keys & body_keys) | (query_keys & body_keys)

    path_field_renames = {}
    query_field_renames = {}
    body_field_renames = {}

    for field in conflicts:
        schemas_and_renames = [
            (path_params_schema, path_keys, '__path', path_field_renames),
            (query_params_schema, query_keys, '__query', query_field_renames),
            (body_schema, body_keys, '__body', body_field_renames),
        ]
        for schema, keys, suffix, renames_map in schemas_and_renames:
            if field in keys and schema and 'properties' in schema:
                new_name = field + suffix
                # Rename in properties
                schema['properties'][new_name] = schema['properties'].pop(field)
                # Update description to clarify the renamed field
                if 'description' not in schema['properties'][new_name]:
                    schema['properties'][new_name] = dict(schema['properties'][new_name])
                prop = schema['properties'][new_name]
                if isinstance(prop, dict):
                    existing_desc = prop.get('description', '')
                    location = suffix.lstrip('_')
                    if not existing_desc:
                        prop['description'] = f"({location} parameter)"
                    else:
                        prop['description'] = f"{existing_desc} ({location} parameter)"
                # Rename in required array
                if 'required' in schema and field in schema['required']:
                    schema['required'] = [new_name if r == field else r for r in schema['required']]
                # Store the reverse mapping: renamed -> original
                renames_map[new_name] = field

    # Return None for empty schemas
    path_params_schema = path_params_schema if path_params_schema and path_params_schema.get('properties') else None
    query_params_schema = query_params_schema if query_params_schema and query_params_schema.get('properties') else None

    return (path_params_schema, query_params_schema, body_schema, path_field_renames, query_field_renames, body_field_renames)

# Cache for loaded external files
_external_file_cache: Dict[str, Dict[str, Any]] = {}

def load_external_file(file_path: str, base_path: str) -> Optional[Dict[str, Any]]:
    """Load an external YAML file relative to the base path."""
    if file_path in _external_file_cache:
        return _external_file_cache[file_path]

    try:
        import yaml
        from pathlib import Path

        # Resolve the path relative to the base file
        base_dir = Path(base_path).parent
        full_path = (base_dir / file_path).resolve()

        with open(full_path, 'r', encoding='utf-8') as f:
            content = yaml.safe_load(f)
            _external_file_cache[file_path] = content
            return content
    except Exception as e:
        print(f"Warning: Could not load external file {file_path}: {e}", file=sys.stderr)
        return None

def resolve_ref(ref_path: str, spec: Dict[str, Any], base_path: str = "") -> tuple:
    """Resolve a $ref path to the actual schema definition.

    Handles both internal refs (#/...) and external file refs (file.yaml#/...).

    Returns a tuple of (resolved_schema, resolved_spec) where resolved_spec is the spec
    that should be used for resolving any nested refs within the resolved schema.
    """
    # Check if this is an external file reference
    if '#' in ref_path and not ref_path.startswith('#'):
        # External file reference: "../../openflow.openapi.yaml#/components/schemas/Retry"
        file_part, fragment = ref_path.split('#', 1)
        external_spec = load_external_file(file_part, base_path)
        if external_spec is None:
            return None, spec
        # Resolve the fragment within the external file, and return external_spec for nested refs
        resolved, _ = resolve_ref('#' + fragment, external_spec, base_path)
        return resolved, external_spec

    if not ref_path.startswith('#/'):
        return None, spec

    # Remove the '#/' prefix and split by '/'
    path_parts = ref_path[2:].split('/')

    # Navigate through the spec following the path
    current = spec
    for part in path_parts:
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None, spec

    return (current if isinstance(current, dict) else None), spec

def resolve_schema_refs(schema: Dict[str, Any], spec: Dict[str, Any], base_path: str = "", _visited_refs: Optional[set] = None) -> Dict[str, Any]:
    """Recursively resolve all $ref references in a schema."""
    if _visited_refs is None:
        _visited_refs = set()

    if not isinstance(schema, dict):
        return schema

    # If this is a $ref, resolve it
    if '$ref' in schema:
        ref_path = schema['$ref']
        if ref_path in _visited_refs:
            # Circular reference detected - return empty object to break the cycle
            return {"type": "object"}
        _visited_refs = _visited_refs | {ref_path}
        resolved, resolved_spec = resolve_ref(ref_path, spec, base_path)
        if resolved:
            # Recursively resolve any refs in the resolved schema using the appropriate spec
            return resolve_schema_refs(resolved, resolved_spec, base_path, _visited_refs)
        else:
            print(f"Warning: Could not resolve $ref: {ref_path}")
            return schema

    # Recursively process all values in the schema
    resolved_schema = {}
    for key, value in schema.items():
        if isinstance(value, dict):
            resolved_schema[key] = resolve_schema_refs(value, spec, base_path, _visited_refs)
        elif isinstance(value, list):
            resolved_schema[key] = [
                resolve_schema_refs(item, spec, base_path, _visited_refs) if isinstance(item, dict) else item
                for item in value
            ]
        else:
            resolved_schema[key] = value

    return resolved_schema

def convert_enums_to_descriptions(schema: Any) -> Any:
    """Recursively convert enum arrays into description text to avoid client compatibility issues."""
    if isinstance(schema, list):
        return [convert_enums_to_descriptions(item) for item in schema]
    if not isinstance(schema, dict):
        return schema

    result = {}
    enum_value = None
    # First pass: copy all non-enum keys so 'description' is available before enum processing
    for key, value in schema.items():
        if key == 'enum':
            enum_value = value
        else:
            result[key] = convert_enums_to_descriptions(value)

    # Second pass: process enum using the already-copied description
    if enum_value is not None:
        values_str = ', '.join(str(v) for v in enum_value)
        existing = result.get('description', '')
        enum_desc = f"Possible values: {values_str}"
        result['description'] = f"{existing}. {enum_desc}" if existing else enum_desc

    return result

def sanitize_empty_schemas(schema: Any) -> Any:
    """Replace empty {} schemas with valid JSON Schema draft 2020-12 equivalents.

    In OpenAPI, {} means 'any value' but strict JSON Schema validators (e.g. Claude's API)
    reject empty objects. This converts them to proper schemas.
    """
    if isinstance(schema, list):
        return [sanitize_empty_schemas(item) for item in schema]
    if not isinstance(schema, dict):
        return schema

    result = {}
    for key, value in schema.items():
        if key == 'additionalProperties' and isinstance(value, dict) and len(value) == 0:
            result[key] = True
        elif key == 'properties' and isinstance(value, dict):
            # properties is a map of name -> schema; sanitize each property schema
            result[key] = {
                k: {"type": "object"} if isinstance(v, dict) and len(v) == 0 else sanitize_empty_schemas(v)
                for k, v in value.items()
            }
        else:
            result[key] = sanitize_empty_schemas(value)
    return result

def extract_request_body_schema(request_body: Dict[str, Any], spec: Dict[str, Any], base_path: str = "") -> Optional[Dict[str, Any]]:
    """Extract request body schema from OpenAPI requestBody definition and resolve refs."""
    if not request_body:
        return None

    content = request_body.get('content', {})
    json_content = content.get('application/json', {})
    schema = json_content.get('schema', {})

    if schema:
        # Resolve any $ref references in the schema
        return resolve_schema_refs(schema, spec, base_path)

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
                    'required_fields': operation.get('x-mcp-required-fields', []),
                    'include_fields': operation.get('x-mcp-tool-include-fields'),
                    'opaque_fields': operation.get('x-mcp-tool-opaque-fields'),
                    'include_query_params': operation.get('x-mcp-tool-include-query-params'),
                }
                tools.append(tool)
    
    return tools

def generate_typescript_code(tools: List[Dict[str, Any]], spec: Dict[str, Any], base_path: str = "") -> str:
    """Generate TypeScript code with MCP endpoint tools."""
    if not tools:
        return """// Auto-generated MCP tools from OpenAPI specification
// This file is generated by generate_mcp_tools.py - DO NOT EDIT MANUALLY

export interface EndpointTool {
    name: string;
    description: string;
    instructions: string;
    path: string;
    method: string;
    pathParamsSchema?: object;
    queryParamsSchema?: object;
    bodySchema?: object;
}

export const mcpEndpointTools: EndpointTool[] = [];
"""

    tool_definitions = []

    for tool in tools:
        tool_name = tool['name']
        description = tool['description'].replace('"', '\\"').replace('\n', '\\n')
        instructions = tool['instructions'].replace('"', '\\"').replace('\n', '\\n')
        path = tool['path']
        method = tool['method'].upper()

        # Generate separate schemas
        path_params_schema, query_params_schema, body_schema, path_field_renames, query_field_renames, body_field_renames = extract_separate_schemas(
            tool['parameters'], tool['requestBody'], spec, tool['required_fields'], base_path,
            tool.get('include_fields'), tool.get('opaque_fields'), tool.get('include_query_params')
        )

        # Convert schemas to TypeScript - use 'as const' for better type inference
        path_params_ts = json.dumps(path_params_schema, indent=8) if path_params_schema else "undefined"
        query_params_ts = json.dumps(query_params_schema, indent=8) if query_params_schema else "undefined"
        body_schema_ts = json.dumps(body_schema, indent=8) if body_schema else "undefined"
        path_field_renames_ts = json.dumps(path_field_renames, indent=8) if path_field_renames else "undefined"
        query_field_renames_ts = json.dumps(query_field_renames, indent=8) if query_field_renames else "undefined"
        body_field_renames_ts = json.dumps(body_field_renames, indent=8) if body_field_renames else "undefined"

        # Generate tool definition
        tool_def = f"""    {{
        name: "{tool_name}",
        description: "{description}",
        instructions: "{instructions}",
        path: "{path}",
        method: "{method}",
        pathParamsSchema: {path_params_ts},
        queryParamsSchema: {query_params_ts},
        bodySchema: {body_schema_ts},
        pathFieldRenames: {path_field_renames_ts},
        queryFieldRenames: {query_field_renames_ts},
        bodyFieldRenames: {body_field_renames_ts}
    }}"""
        tool_definitions.append(tool_def)

    # Combine everything
    tool_definitions_str = ",\n".join(tool_definitions)

    typescript_code = f"""// Auto-generated MCP tools from OpenAPI specification
// This file is generated by generate_mcp_tools.py - DO NOT EDIT MANUALLY

export interface EndpointTool {{
    name: string;
    description: string;
    instructions: string;
    path: string;
    method: string;
    pathParamsSchema?: object;
    queryParamsSchema?: object;
    bodySchema?: object;
    pathFieldRenames?: Record<string, string>;
    queryFieldRenames?: Record<string, string>;
    bodyFieldRenames?: Record<string, string>;
}}

export const mcpEndpointTools: EndpointTool[] = [
{tool_definitions_str}
];
"""

    return typescript_code

def generate_rust_code(tools: List[Dict[str, Any]], spec: Dict[str, Any], base_path: str = "") -> str:
    """Generate the complete Rust code with MCP tools."""
    if not tools:
        return f"""// No MCP tools found in the OpenAPI specification
{IMPORTS}
pub fn all_tools() -> Vec<EndpointTool> {{
    vec![]
}}
"""

    tool_definitions = []

    for tool in tools:
        tool_name = tool['name']
        description = tool['description']
        instructions = tool['instructions']
        path = tool['path']
        method = tool['method'].upper()

        # Generate separate schemas
        path_params_schema, query_params_schema, body_schema, path_field_renames, query_field_renames, body_field_renames = extract_separate_schemas(
            tool['parameters'], tool['requestBody'], spec, tool['required_fields'], base_path,
            tool.get('include_fields'), tool.get('opaque_fields'), tool.get('include_query_params')
        )

        path_params_rust = schema_to_rust_value(path_params_schema)
        query_params_rust = schema_to_rust_value(query_params_schema)
        body_schema_rust = schema_to_rust_value(body_schema)
        path_field_renames_rust = schema_to_rust_value(path_field_renames if path_field_renames else None)
        query_field_renames_rust = schema_to_rust_value(query_field_renames if query_field_renames else None)
        body_field_renames_rust = schema_to_rust_value(body_field_renames if body_field_renames else None)

        # Generate tool definition
        tool_def = f"""    EndpointTool {{
        name: Cow::Borrowed("{tool_name}"),
        description: Cow::Borrowed("{description}"),
        instructions: Cow::Borrowed("{instructions}"),
        path: Cow::Borrowed("{path}"),
        method: Cow::Borrowed("{method}"),
        path_params_schema: {path_params_rust},
        query_params_schema: {query_params_rust},
        body_schema: {body_schema_rust},
        path_field_renames: {path_field_renames_rust},
        query_field_renames: {query_field_renames_rust},
        body_field_renames: {body_field_renames_rust},
    }}"""
        tool_definitions.append(tool_def)

    # Combine everything
    tool_definitions_str = ",\n".join(tool_definitions)

    rust_code = f"""// Auto-generated MCP tools from OpenAPI specification
// This file is generated by generate_mcp_tools.py - DO NOT EDIT MANUALLY
{IMPORTS}
pub fn all_tools() -> Vec<EndpointTool> {{
    vec![
{tool_definitions_str}
    ]
}}
"""

    return rust_code

def main():
    """Main function to parse OpenAPI and generate Rust and TypeScript code."""
    script_dir = Path(__file__).parent
    backend_dir = script_dir.parent
    project_dir = backend_dir.parent

    openapi_file = backend_dir / "windmill-api" / "openapi.yaml"
    rust_output_file = backend_dir / "windmill-api" / "src" / "mcp" / "auto_generated_endpoints.rs"
    ts_output_file = project_dir / "frontend" / "src" / "lib" / "mcpEndpointTools.ts"

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

    # Generate and write Rust code
    print(f"Generating Rust code...")
    rust_code = generate_rust_code(tools, spec, str(openapi_file))

    print(f"Writing Rust code to: {rust_output_file}")
    rust_output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(rust_output_file, 'w', encoding='utf-8') as f:
        f.write(rust_code)

    # Generate and write TypeScript code
    print(f"Generating TypeScript code...")
    typescript_code = generate_typescript_code(tools, spec, str(openapi_file))

    print(f"Writing TypeScript code to: {ts_output_file}")
    ts_output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(ts_output_file, 'w', encoding='utf-8') as f:
        f.write(typescript_code)

    print("Done!")

if __name__ == "__main__":
    main()