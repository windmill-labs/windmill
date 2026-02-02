"""
Utility functions and constants for system prompts generation.

This module contains:
- Path constants for SDK files and output directories
- Field exclusion lists for CLI format
- Language metadata for script generation
- String/file utility functions
- Parsing utilities for TypeScript/Python code
- Schema transformation utilities
"""

import json
import re
from pathlib import Path

# =============================================================================
# Path Constants
# =============================================================================

SCRIPT_DIR = Path(__file__).parent
ROOT_DIR = SCRIPT_DIR.parent

TS_SDK_DIR = ROOT_DIR / "typescript-client"
PY_SDK_PATH = ROOT_DIR / "python-client" / "wmill" / "wmill" / "client.py"
OPENFLOW_SCHEMA_PATH = ROOT_DIR / "openflow.openapi.yaml"
BACKEND_OPENAPI_PATH = ROOT_DIR / "backend" / "windmill-api" / "openapi.yaml"

OUTPUT_SDKS_DIR = SCRIPT_DIR / "auto-generated" / "sdks"
OUTPUT_GENERATED_DIR = SCRIPT_DIR / "auto-generated"
OUTPUT_CLI_DIR = SCRIPT_DIR / "auto-generated" / "cli"
OUTPUT_SKILLS_DIR = SCRIPT_DIR / "auto-generated" / "skills"

# CLI guidance directory (DNT can't import from outside cli/, so we copy files there)
CLI_GUIDANCE_DIR = ROOT_DIR / "cli" / "src" / "guidance"

# CLI source paths for extracting command documentation
CLI_DIR = ROOT_DIR / "cli"
CLI_MAIN = CLI_DIR / "src" / "main.ts"
CLI_COMMANDS_DIR = CLI_DIR / "src" / "commands"

# =============================================================================
# Field Exclusion Lists
# =============================================================================

# Fields stripped by CLI/sync format (to_string_without_metadata equivalent)
# These are server-managed fields that don't appear in YAML/JSON files pulled via CLI
CLI_EXCLUDED_FIELDS = [
    'workspace_id', 'path', 'name', 'versions', 'id',
    'created_at', 'updated_at', 'created_by', 'updated_by',
    'edited_at', 'edited_by', 'archived', 'has_draft',
    'error', 'last_server_ping', 'server_id',
    'extra_perms', 'email', 'enabled', 'mode'
]

# =============================================================================
# Language Metadata
# =============================================================================

# Mapping of language file names to friendly names and descriptions
LANGUAGE_METADATA = {
    'bun': {
        'name': 'TypeScript (Bun)',
        'description': 'MUST use when writing Bun/TypeScript scripts.',
        'use_cases': 'TypeScript automation, npm packages, data processing, API integrations'
    },
    'deno': {
        'name': 'TypeScript (Deno)',
        'description': 'MUST use when writing Deno/TypeScript scripts.',
        'use_cases': 'TypeScript with Deno stdlib, secure sandboxed execution'
    },
    'nativets': {
        'name': 'Native TypeScript',
        'description': 'MUST use when writing Native TypeScript scripts.',
        'use_cases': 'simple API calls, lightweight TypeScript, no dependencies'
    },
    'bunnative': {
        'name': 'Bun Native',
        'description': 'MUST use when writing Bun Native scripts.',
        'use_cases': 'simple Bun scripts, lightweight, no dependencies'
    },
    'python3': {
        'name': 'Python',
        'description': 'MUST use when writing Python scripts.',
        'use_cases': 'Python automation, data processing, machine learning, scripting'
    },
    'bash': {
        'name': 'Bash',
        'description': 'MUST use when writing Bash scripts.',
        'use_cases': 'shell scripts, system administration, CLI tools'
    },
    'go': {
        'name': 'Go',
        'description': 'MUST use when writing Go scripts.',
        'use_cases': 'Go automation, high performance, concurrent processing'
    },
    'rust': {
        'name': 'Rust',
        'description': 'MUST use when writing Rust scripts.',
        'use_cases': 'Rust automation, high performance, memory safety'
    },
    'postgresql': {
        'name': 'PostgreSQL',
        'description': 'MUST use when writing PostgreSQL queries.',
        'use_cases': 'PostgreSQL database queries, data analysis'
    },
    'mysql': {
        'name': 'MySQL',
        'description': 'MUST use when writing MySQL queries.',
        'use_cases': 'MySQL database queries, data operations'
    },
    'mssql': {
        'name': 'MS SQL Server',
        'description': 'MUST use when writing MS SQL Server queries.',
        'use_cases': 'SQL Server database queries, enterprise data'
    },
    'bigquery': {
        'name': 'BigQuery',
        'description': 'MUST use when writing BigQuery queries.',
        'use_cases': 'BigQuery analytics, large-scale data analysis'
    },
    'snowflake': {
        'name': 'Snowflake',
        'description': 'MUST use when writing Snowflake queries.',
        'use_cases': 'Snowflake data warehouse queries, analytics'
    },
    'duckdb': {
        'name': 'DuckDB',
        'description': 'MUST use when writing DuckDB queries.',
        'use_cases': 'DuckDB analytics, local data processing, Ducklake'
    },
    'graphql': {
        'name': 'GraphQL',
        'description': 'MUST use when writing GraphQL queries.',
        'use_cases': 'GraphQL API calls, federated queries'
    },
    'php': {
        'name': 'PHP',
        'description': 'MUST use when writing PHP scripts.',
        'use_cases': 'PHP automation, web integrations'
    },
    'powershell': {
        'name': 'PowerShell',
        'description': 'MUST use when writing PowerShell scripts.',
        'use_cases': 'Windows automation, system administration'
    },
    'csharp': {
        'name': 'C#',
        'description': 'MUST use when writing C# scripts.',
        'use_cases': 'C# automation, .NET integrations'
    },
    'java': {
        'name': 'Java',
        'description': 'MUST use when writing Java scripts.',
        'use_cases': 'Java automation, enterprise integrations'
    },
}

# Languages that use TypeScript SDK
TS_SDK_LANGUAGES = ['bun', 'deno', 'nativets', 'bunnative']

# Languages that use Python SDK
PY_SDK_LANGUAGES = ['python3']

# =============================================================================
# String/File Utilities
# =============================================================================


def clean_jsdoc(jsdoc: str) -> str:
    """Clean up JSDoc comment, removing delimiters and leading asterisks."""
    # Remove /** and */
    jsdoc = re.sub(r'^/\*\*\s*', '', jsdoc)
    jsdoc = re.sub(r'\s*\*/$', '', jsdoc)
    # Remove leading * from each line
    lines = jsdoc.split('\n')
    cleaned = []
    for line in lines:
        line = re.sub(r'^\s*\*\s?', '', line)
        cleaned.append(line)
    return '\n'.join(cleaned).strip()


def clean_params(params: str) -> str:
    """Clean up parameter string."""
    if not params:
        return ''
    # Remove excessive whitespace and newlines
    params = re.sub(r'\s+', ' ', params).strip()
    return params


def escape_for_ts(content: str) -> str:
    """Escape content for TypeScript template literal."""
    return content.replace('\\', '\\\\').replace('`', '\\`').replace('${', '\\${')


def read_markdown_file(path: Path) -> str:
    """Read a markdown file and return its content."""
    if path.exists():
        return path.read_text()
    return ''


# =============================================================================
# Parsing Utilities
# =============================================================================


def extract_balanced(content: str, start_pos: int, open_char: str, close_char: str) -> tuple[str, int]:
    """
    Extract content between balanced brackets starting at start_pos.
    Returns (extracted_content, end_position) or ('', -1) if not found.
    """
    if start_pos >= len(content) or content[start_pos] != open_char:
        return '', -1

    depth = 0
    i = start_pos
    while i < len(content):
        if content[i] == open_char:
            depth += 1
        elif content[i] == close_char:
            depth -= 1
            if depth == 0:
                return content[start_pos + 1:i], i
        i += 1
    return '', -1


def extract_return_type(content: str, start_pos: int) -> tuple[str, int]:
    """
    Extract return type from position after ')', handling nested braces.
    Returns (return_type, end_position of function body open brace).
    """
    i = start_pos
    # Skip whitespace
    while i < len(content) and content[i] in ' \t\n':
        i += 1

    if i >= len(content) or content[i] != ':':
        # No return type, find opening brace
        while i < len(content) and content[i] != '{':
            i += 1
        return '', i

    i += 1  # Skip ':'

    # Now extract the return type, handling nested braces and angle brackets
    return_type_start = i
    brace_depth = 0
    angle_depth = 0

    while i < len(content):
        char = content[i]
        if char == '<':
            angle_depth += 1
        elif char == '>':
            angle_depth -= 1
        elif char == '{':
            if angle_depth > 0:
                # Inside a type like Promise<{...}>
                brace_depth += 1
            else:
                # This is the function body opening brace
                return content[return_type_start:i].strip(), i
        elif char == '}':
            brace_depth -= 1
        i += 1

    return '', -1


def parse_default_imports(content: str) -> dict[str, str]:
    """
    Parse default imports from TypeScript content.
    Returns a dict mapping variable names to relative file paths.
    E.g., 'import devCommand from "./dev.ts"' -> {'devCommand': './dev.ts'}
    """
    imports = {}
    # Match: import varName from "./path.ts" or import varName from './path.ts'
    import_pattern = re.compile(
        r'import\s+(\w+)\s+from\s+["\']([^"\']+)["\']',
        re.MULTILINE
    )
    for match in import_pattern.finditer(content):
        var_name, path = match.groups()
        imports[var_name] = path
    return imports


def extract_options(text: str, option_pattern: re.Pattern) -> list[dict]:
    """
    Extract options from text using the given regex pattern.

    The pattern should have 4 groups: (dq_flag, dq_desc, sq_flag, sq_desc)
    for double-quoted and single-quoted variants.

    Returns a list of dicts with 'flag' and 'description' keys.
    """
    options = []
    for match in option_pattern.finditer(text):
        groups = match.groups()
        # Pattern has 4 groups: (dq_flag, dq_desc, sq_flag, sq_desc)
        # Either double-quoted or single-quoted pair will be non-None
        flag = groups[0] or groups[2]
        desc = groups[1] or groups[3]
        if flag and desc:
            options.append({'flag': flag, 'description': desc})
    return options


# =============================================================================
# Schema Utilities
# =============================================================================


def extract_cli_schema(schema: dict, all_schemas: dict, openflow_schemas: dict | None = None) -> dict:
    """
    Transform an OpenAPI schema to CLI format by removing server-managed fields.
    Resolves $ref references and handles allOf compositions.

    Args:
        schema: The schema to transform
        all_schemas: All schemas from the backend OpenAPI
        openflow_schemas: Schemas from the openflow.openapi.yaml file (for external refs)
    """
    if not schema:
        return {}

    openflow_schemas = openflow_schemas or {}
    result = {'type': 'object', 'properties': {}, 'required': []}

    # Handle allOf (used for composition, e.g., TriggerExtraProperty)
    if 'allOf' in schema:
        for item in schema['allOf']:
            if '$ref' in item:
                ref_name = item['$ref'].split('/')[-1]
                if ref_name in all_schemas:
                    ref_schema = extract_cli_schema(all_schemas[ref_name], all_schemas, openflow_schemas)
                    result['properties'].update(ref_schema.get('properties', {}))
                    result['required'].extend(ref_schema.get('required', []))

    # Handle direct properties
    if 'properties' in schema:
        for key, value in schema['properties'].items():
            if key not in CLI_EXCLUDED_FIELDS:
                # Resolve $ref in property values
                if '$ref' in value:
                    ref_path = value['$ref']
                    # Handle local references
                    if ref_path.startswith('#/components/schemas/'):
                        ref_name = ref_path.split('/')[-1]
                        if ref_name in all_schemas:
                            result['properties'][key] = all_schemas[ref_name]
                        else:
                            result['properties'][key] = {'type': 'string', 'description': f'See {ref_name}'}
                    elif 'openflow.openapi.yaml' in ref_path:
                        # External reference to openflow schema - resolve it
                        ref_name = ref_path.split('/')[-1]
                        if ref_name in openflow_schemas:
                            result['properties'][key] = openflow_schemas[ref_name]
                        else:
                            result['properties'][key] = {'type': 'object', 'description': value.get('description', f'See {ref_name}')}
                    else:
                        # Other external reference
                        result['properties'][key] = {'type': 'object', 'description': value.get('description', f'See {ref_path}')}
                else:
                    result['properties'][key] = value

    # Handle required fields
    if 'required' in schema:
        result['required'].extend([
            r for r in schema['required']
            if r not in CLI_EXCLUDED_FIELDS
        ])

    # Remove duplicates from required
    result['required'] = list(dict.fromkeys(result['required']))

    # Filter out required fields that don't exist in properties
    result['required'] = [r for r in result['required'] if r in result['properties']]

    return result


def format_schema_as_json(schema: dict) -> dict:
    """Convert a CLI schema to a clean JSON Schema representation."""
    if not schema or not schema.get('properties'):
        return {}

    result = {
        'type': 'object',
        'properties': {},
    }

    props = schema.get('properties', {})
    required = schema.get('required', [])

    for key, value in props.items():
        prop_def = {}

        # Get type
        prop_type = value.get('type', 'string')
        if prop_type == 'array':
            items = value.get('items', {})
            prop_def['type'] = 'array'
            item_type = items.get('type', 'object')
            if item_type == 'object' and items.get('properties'):
                prop_def['items'] = {'type': 'object', 'properties': items.get('properties', {})}
            else:
                prop_def['items'] = {'type': item_type}
        elif '$ref' in value:
            # For refs, just indicate the type
            ref_name = value['$ref'].split('/')[-1]
            prop_def['type'] = ref_name
        elif prop_type == 'object' and value.get('properties'):
            # Nested object with properties - include them
            prop_def['type'] = 'object'
            prop_def['properties'] = value.get('properties', {})
        else:
            prop_def['type'] = prop_type

        # Add enum if present
        if 'enum' in value:
            prop_def['enum'] = value['enum']

        # Add description if present
        if value.get('description'):
            prop_def['description'] = value['description']

        result['properties'][key] = prop_def

    if required:
        result['required'] = required

    return result


def format_schema_for_markdown(schema: dict, schema_name: str, as_json_schema: bool = False) -> str:
    """Format a CLI schema as markdown documentation."""
    if not schema or not schema.get('properties'):
        return ''

    if as_json_schema:
        # Output as JSON Schema
        json_schema = format_schema_as_json(schema)
        schema_json = json.dumps(json_schema, indent=2)
        return f"## {schema_name}\n\nMust be a YAML file that adheres to the following schema:\n\n```json\n{schema_json}\n```"
    else:
        # Output as field list (for schedules)
        lines = [f"### {schema_name} Schema (CLI Format)\n"]
        lines.append("Fields available in `.yaml`/`.json` files:\n")

        props = schema.get('properties', {})
        required = set(schema.get('required', []))

        for key, value in sorted(props.items()):
            prop_type = value.get('type', 'any')
            if prop_type == 'array':
                items = value.get('items', {})
                item_type = items.get('type', 'any')
                prop_type = f"array[{item_type}]"
            elif '$ref' in value:
                prop_type = value['$ref'].split('/')[-1]

            req_marker = ' (required)' if key in required else ''
            desc = value.get('description', '')
            desc_str = f" - {desc}" if desc else ''

            lines.append(f"- `{key}`: {prop_type}{req_marker}{desc_str}")

        return '\n'.join(lines)
