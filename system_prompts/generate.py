#!/usr/bin/env python3
"""
Generate system prompts documentation from SDKs and OpenFlow schema.

This script:
1. Parses TypeScript SDK to extract function signatures
2. Parses Python SDK using Python's ast module
3. Parses OpenFlow YAML schema
4. Generates markdown files in sdks/ and schemas/
5. Assembles complete prompts and generates TypeScript exports in generated/

Usage:
    python generate.py
    python generate.py --plugin-dir /path/to/windmill-claude-plugin
"""

import argparse
import ast
import copy
import json
import re
import shutil
from pathlib import Path

import yaml

from utils import (
    # Path constants
    SCRIPT_DIR,
    TS_SDK_DIR,
    PY_SDK_PATH,
    OPENFLOW_SCHEMA_PATH,
    BACKEND_OPENAPI_PATH,
    OUTPUT_SDKS_DIR,
    OUTPUT_GENERATED_DIR,
    OUTPUT_CLI_DIR,
    OUTPUT_SKILLS_DIR,
    OUTPUT_SCHEMAS_DIR,
    CLI_GUIDANCE_DIR,
    CLI_MAIN,
    CLI_COMMANDS_DIR,
    # Language metadata
    LANGUAGE_METADATA,
    TS_SDK_LANGUAGES,
    PY_SDK_LANGUAGES,
    # Schema mappings
    SCHEMA_MAPPINGS,
    # String/file utilities
    clean_jsdoc,
    clean_params,
    escape_for_ts,
    read_markdown_file,
    # Parsing utilities
    extract_balanced,
    extract_return_type,
    parse_default_imports,
    extract_options,
    # Schema utilities
    extract_cli_schema,
    format_schema_for_markdown,
    format_schema_as_json,
)


# =============================================================================
# TypeScript SDK Parsing
# =============================================================================


def extract_ts_functions(content: str) -> list[dict]:
    """Extract exported function signatures from TypeScript SDK."""
    functions = []
    seen_names = set()

    # Pattern to find export function declarations (with or without JSDoc)
    # Captures JSDoc if present, then the function declaration
    pattern = re.compile(
        r'(?:(/\*\*(?:[^*]|\*(?!/))*\*/)\s*)?'  # Optional JSDoc comment
        r'export\s+(async\s+)?function\s+(\w+)\s*'  # export [async] function name
        r'(<[^>]+>)?\s*',  # optional generic
        re.MULTILINE
    )

    for match in pattern.finditer(content):
        jsdoc_raw, is_async, name, generic = match.groups()

        if name in seen_names:
            continue

        # Find the opening parenthesis for parameters
        pos = match.end()
        while pos < len(content) and content[pos] in ' \t\n':
            pos += 1

        if pos >= len(content) or content[pos] != '(':
            continue

        # Extract balanced parameters
        params, paren_end = extract_balanced(content, pos, '(', ')')
        if paren_end == -1:
            continue

        # Extract return type (handles multi-line types like Promise<{...}>)
        return_type, _ = extract_return_type(content, paren_end + 1)

        if not return_type:
            return_type = 'Promise<void>' if is_async else 'void'

        docstring = clean_jsdoc(jsdoc_raw) if jsdoc_raw else ''
        seen_names.add(name)
        functions.append({
            'name': name,
            'generic': generic or '',
            'params': clean_params(params),
            'return_type': return_type,
            'async': bool(is_async),
            'docstring': docstring
        })

    return functions


def extract_ts_types(content: str) -> list[dict]:
    """Extract exported type definitions from TypeScript SDK."""
    types = []

    # Pattern for exported type aliases
    type_pattern = re.compile(
        r'export\s+type\s+(\w+)\s*=\s*([^;]+);',
        re.MULTILINE
    )

    # Pattern for exported interfaces
    interface_pattern = re.compile(
        r'export\s+interface\s+(\w+)\s*\{([^}]+)\}',
        re.MULTILINE | re.DOTALL
    )

    for match in type_pattern.finditer(content):
        name, definition = match.groups()
        types.append({
            'name': name,
            'kind': 'type',
            'definition': definition.strip()
        })

    for match in interface_pattern.finditer(content):
        name, body = match.groups()
        types.append({
            'name': name,
            'kind': 'interface',
            'definition': body.strip()
        })

    return types


# =============================================================================
# Python SDK Parsing
# =============================================================================


def extract_py_functions(content: str) -> list[dict]:
    """Extract function signatures from Python SDK using AST."""
    functions = []
    seen_names = set()

    try:
        tree = ast.parse(content)
    except SyntaxError as e:
        print(f"Warning: Could not parse Python SDK: {e}")
        return functions

    def process_function(node):
        """Process a function node and add to functions list if not duplicate."""
        # Skip private functions
        if node.name.startswith('_') and not node.name.startswith('__'):
            return
        # Skip duplicates
        if node.name in seen_names:
            return

        # Get docstring
        docstring = ast.get_docstring(node) or ''

        # Build parameter list
        params = []
        args = node.args

        # Handle regular args
        num_defaults = len(args.defaults)
        num_args = len(args.args)

        for i, arg in enumerate(args.args):
            if arg.arg == 'self':
                continue
            param_str = arg.arg
            if arg.annotation:
                param_str += f": {ast.unparse(arg.annotation)}"
            # Check if has default
            default_idx = i - (num_args - num_defaults)
            if default_idx >= 0:
                default = args.defaults[default_idx]
                param_str += f" = {ast.unparse(default)}"
            params.append(param_str)

        # Handle *args
        if args.vararg:
            params.append(f"*{args.vararg.arg}")

        # Handle keyword-only args
        for i, arg in enumerate(args.kwonlyargs):
            param_str = arg.arg
            if arg.annotation:
                param_str += f": {ast.unparse(arg.annotation)}"
            if args.kw_defaults[i]:
                param_str += f" = {ast.unparse(args.kw_defaults[i])}"
            params.append(param_str)

        # Handle **kwargs
        if args.kwarg:
            params.append(f"**{args.kwarg.arg}")

        # Get return type
        return_type = ''
        if node.returns:
            return_type = ast.unparse(node.returns)

        seen_names.add(node.name)
        functions.append({
            'name': node.name,
            'params': ', '.join(params),
            'return_type': return_type,
            'docstring': docstring,
            'async': isinstance(node, ast.AsyncFunctionDef)
        })

    # Process top-level functions and class methods (but not nested functions)
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            process_function(node)
        elif isinstance(node, ast.ClassDef):
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    process_function(item)

    return functions


def extract_py_classes(content: str) -> list[dict]:
    """Extract class definitions from Python SDK."""
    classes = []

    try:
        tree = ast.parse(content)
    except SyntaxError:
        return classes

    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            methods = []
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if not item.name.startswith('_') or item.name == '__init__':
                        docstring = ast.get_docstring(item) or ''
                        methods.append({
                            'name': item.name,
                            'docstring': docstring
                        })

            classes.append({
                'name': node.name,
                'docstring': ast.get_docstring(node) or '',
                'methods': methods
            })

    return classes


# =============================================================================
# CLI Command Parsing
# =============================================================================


# Reusable option pattern for CLI parsing
OPTION_PATTERN = re.compile(
    r'\.option\(\s*"([^"]+)"\s*,\s*"([^"]+)"'  # double-quoted
    r'|'
    r"\.option\(\s*'([^']+)'\s*,\s*'([^']+)'",  # single-quoted
    re.MULTILINE | re.DOTALL
)


def parse_command_block(content: str, file_path: Path | None = None) -> dict:
    """
    Parse a Cliffy Command() definition block and extract metadata.
    Returns a dict with: description, options, subcommands, arguments, alias

    If file_path is provided, imported subcommands will be resolved by parsing
    the imported files.
    """
    result = {
        'description': '',
        'options': [],
        'subcommands': [],
        'arguments': '',
        'alias': ''
    }

    # Find the command block
    command_match = re.search(
        r'(?:const\s+command\s*=\s*)?new\s+Command\(\)([\s\S]*?)(?=export\s+default)',
        content
    )
    if not command_match:
        return result

    block = command_match.group(1)

    # Find where subcommands start
    first_subcommand_pos = block.find('.command(')
    if first_subcommand_pos == -1:
        first_subcommand_pos = len(block)
    top_section = block[:first_subcommand_pos]

    # Extract main description
    desc_match = re.search(r'\.description\(\s*["\']([^"\']+)["\']\s*,?\s*\)', top_section, re.DOTALL)
    if not desc_match:
        desc_match = re.search(r'\.description\(\s*`([^`]+)`\s*,?\s*\)', top_section, re.DOTALL)
    if desc_match:
        result['description'] = desc_match.group(1).strip()

    # Extract alias
    alias_match = re.search(r'\.alias\(\s*["\']([^"\']+)["\']\s*\)', top_section)
    if alias_match:
        result['alias'] = alias_match.group(1)

    # Extract top-level options (before any .command() or .action())
    top_section_until_action = re.split(r'\.action\(', top_section)[0]
    result['options'] = extract_options(top_section_until_action, OPTION_PATTERN)

    # Extract top-level arguments
    args_match = re.search(r'\.arguments\(\s*["\']([^"\']+)["\']\s*\)', top_section)
    if args_match:
        result['arguments'] = args_match.group(1)

    # Parse imports if we have a file path (for resolving imported subcommands)
    imports = parse_default_imports(content) if file_path else {}

    # Extract subcommands
    subcommand_sections = re.split(r'(?=\.command\()', block)

    for section in subcommand_sections:
        cmd_match = re.match(r'\.command\(\s*["\']([^"\']+)["\']\s*(?:,\s*("(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'|[^)]+))?\s*\)', section)
        if not cmd_match:
            continue

        # Explicit source marker for backwards-compatible CLI commands that
        # should not be suggested in generated system prompts.
        if '@deprecated' in section:
            continue

        cmd_name = cmd_match.group(1)
        second_arg = cmd_match.group(2).strip() if cmd_match.group(2) else ''

        # Check if second arg is a string (description) or a variable (imported command)
        is_string_desc = second_arg.startswith('"') or second_arg.startswith("'")

        if is_string_desc:
            cmd_desc = second_arg.strip('"\'')
        elif second_arg and second_arg in imports and file_path:
            # Imported command - resolve and parse the imported file
            import_path = imports[second_arg]
            if import_path.startswith('./') or import_path.startswith('../'):
                imported_file = (file_path.parent / import_path).resolve()
                if imported_file.exists():
                    try:
                        imported_content = imported_file.read_text()
                        imported_cmd = parse_command_block(imported_content, imported_file)
                        result['subcommands'].append({
                            'name': cmd_name,
                            'description': imported_cmd.get('description', ''),
                            'arguments': imported_cmd.get('arguments', ''),
                            'options': imported_cmd.get('options', [])
                        })
                        continue
                    except Exception as e:
                        print(f"  Warning: Could not parse imported command {second_arg}: {e}")
            cmd_desc = ''
        else:
            cmd_desc = ''

        # Check for description in chained .description() call
        desc_match = re.search(r'\.description\(\s*["\']([^"\']+)["\']\s*,?\s*\)', section, re.DOTALL)
        if desc_match:
            cmd_desc = desc_match.group(1).strip()

        # Check for arguments
        args_match = re.search(r'\.arguments\(\s*["\']([^"\']+)["\']\s*\)', section)
        cmd_args = args_match.group(1) if args_match else ''

        # Extract options specific to this subcommand (before .action())
        section_until_action = re.split(r'\.action\(', section)[0]
        cmd_options = extract_options(section_until_action, OPTION_PATTERN)

        result['subcommands'].append({
            'name': cmd_name,
            'description': cmd_desc,
            'arguments': cmd_args,
            'options': cmd_options
        })

    return result


def find_command_file(cmd_name: str) -> Path | None:
    """Find the command file for a given command name."""
    standard_path = CLI_COMMANDS_DIR / cmd_name / f"{cmd_name}.ts"
    if standard_path.exists():
        return standard_path
    return None


def extract_cli_commands() -> dict:
    """
    Extract CLI command metadata from the CLI source files.
    Returns a dict with global_options and commands.
    """
    result = {
        'version': '',
        'global_options': [],
        'commands': []
    }

    if not CLI_MAIN.exists():
        print(f"Warning: CLI main file not found at {CLI_MAIN}")
        return result

    main_content = CLI_MAIN.read_text()

    # Extract version
    version_match = re.search(r'export\s+const\s+VERSION\s*=\s*["\']([^"\']+)["\']', main_content)
    if version_match:
        result['version'] = version_match.group(1)

    # Extract global options from main.ts
    global_opt_pattern = re.compile(
        r'\.globalOption\(\s*["\']([^"\']+)["\']\s*,\s*["\']([^"\']+)["\']\s*\)',
        re.MULTILINE
    )
    for match in global_opt_pattern.finditer(main_content):
        flag, desc = match.groups()
        result['global_options'].append({'flag': flag, 'description': desc})

    # Extract command registrations from main.ts
    cmd_reg_pattern = re.compile(
        r'\.command\(\s*["\']([^"\']+)["\']\s*,\s*(\w+)\s*\)',
        re.MULTILINE
    )

    inline_cmd_pattern = re.compile(
        r'\.command\(\s*["\']([^"\']+)["\']\s*,\s*["\']([^"\']+)["\']\s*\)',
        re.MULTILINE
    )

    registered_commands = []

    for match in cmd_reg_pattern.finditer(main_content):
        cmd_name = match.group(1).split()[0]
        registered_commands.append(cmd_name)

    # Process each registered command
    for cmd_name in registered_commands:
        cmd_file = find_command_file(cmd_name)
        if cmd_file:
            try:
                cmd_content = cmd_file.read_text()
                cmd_data = parse_command_block(cmd_content, cmd_file)
                cmd_data['name'] = cmd_name
                result['commands'].append(cmd_data)
            except Exception as e:
                print(f"Warning: Could not parse command file for {cmd_name}: {e}")

    # Handle special inline commands from main.ts
    for match in inline_cmd_pattern.finditer(main_content):
        cmd_name = match.group(1).split()[0]
        cmd_desc = match.group(2)
        if cmd_name not in [c['name'] for c in result['commands']]:
            result['commands'].append({
                'name': cmd_name,
                'description': cmd_desc,
                'options': [],
                'subcommands': [],
                'arguments': '',
                'alias': ''
            })

    return result


# =============================================================================
# Markdown Generation
# =============================================================================


def generate_cli_commands_markdown(cli_data: dict) -> str:
    """Generate markdown documentation from extracted CLI command data."""
    md = "# Windmill CLI Commands\n\n"
    md += "The Windmill CLI (`wmill`) provides commands for managing scripts, flows, apps, and other resources.\n\n"

    # Global options
    if cli_data.get('global_options'):
        md += "## Global Options\n\n"
        for opt in cli_data['global_options']:
            flag = opt['flag']
            desc = opt['description']
            md += f"- `{flag}` - {desc}\n"
        md += "\n"

    # Commands
    if cli_data.get('commands'):
        md += "## Commands\n\n"

        for cmd in sorted(cli_data['commands'], key=lambda x: x['name']):
            md += f"### {cmd['name']}\n\n"

            if cmd.get('description'):
                md += f"{cmd['description']}\n\n"

            if cmd.get('alias'):
                md += f"**Alias:** `{cmd['alias']}`\n\n"

            if cmd.get('arguments'):
                md += f"**Arguments:** `{cmd['arguments']}`\n\n"

            # Top-level options for this command
            if cmd.get('options'):
                md += "**Options:**\n"
                for opt in cmd['options']:
                    md += f"- `{opt['flag']}` - {opt['description']}\n"
                md += "\n"

            # Subcommands
            if cmd.get('subcommands'):
                md += "**Subcommands:**\n\n"
                for sub in cmd['subcommands']:
                    sub_name = sub['name']
                    sub_args = f" {sub['arguments']}" if sub.get('arguments') else ""
                    sub_desc = sub.get('description', '')

                    md += f"- `{cmd['name']} {sub_name}{sub_args}`"
                    if sub_desc:
                        md += f" - {sub_desc}"
                    md += "\n"

                    # Subcommand options
                    if sub.get('options'):
                        for opt in sub['options']:
                            md += f"  - `{opt['flag']}` - {opt['description']}\n"

                md += "\n"

    return md


def generate_ts_sdk_markdown(functions: list[dict], _types: list[dict]) -> str:
    """Generate compact documentation for TypeScript SDK."""
    md = "# TypeScript SDK (windmill-client)\n\n"
    md += "Import: import * as wmill from 'windmill-client'\n\n"

    for i, func in enumerate(functions):
        if func.get('docstring'):
            # Format docstrings with JSDoc /** */ syntax
            md += "/**\n"
            docstring_lines = func['docstring'].split('\n')
            for line in docstring_lines:
                md += f" * {line}\n"
            md += " */\n"
        async_prefix = 'async ' if func['async'] else ''
        md += f"{async_prefix}{func['name']}{func['generic']}({func['params']}): {func['return_type']}"
        md += "\n"
        if i < len(functions) - 1:
            md += "\n"

    return md


def generate_py_sdk_markdown(functions: list[dict], _classes: list[dict]) -> str:
    """Generate compact documentation for Python SDK."""
    md = "# Python SDK (wmill)\n\n"
    md += "Import: import wmill\n\n"

    for func in functions:
        # Skip private functions
        if func['name'].startswith('_'):
            continue
        docstring = func.get('docstring')
        if docstring:
            # Format multi-line docstrings with # prefix on each line
            docstring_lines = docstring.split('\n')
            for line in docstring_lines:
                md += f"# {line}\n"
        async_prefix = 'async ' if func['async'] else ''
        return_annotation = f" -> {func['return_type']}" if func['return_type'] else ''
        md += f"{async_prefix}def {func['name']}({func['params']}){return_annotation}\n"
        md += "\n"

    return md


def generate_ts_exports(prompts: dict[str, str]) -> str:
    """Generate TypeScript file that exports all prompts."""
    ts = "// Auto-generated by generate.py - DO NOT EDIT\n\n"

    for name, content in prompts.items():
        escaped = escape_for_ts(content)
        ts += f"export const {name} = `{escaped}`;\n\n"

    return ts


# =============================================================================
# Schema File Generation
# =============================================================================


def generate_schema_files(cli_schemas: dict[str, dict]) -> dict[str, str]:
    """
    Generate standalone YAML schema files for triggers and schedules.

    Returns a dict mapping schema keys (e.g., 'http_trigger') to YAML content.
    """
    print("Generating standalone schema files...")

    # Ensure schemas directory exists
    OUTPUT_SCHEMAS_DIR.mkdir(parents=True, exist_ok=True)

    schema_yaml_content = {}

    # Collect all schema types from SCHEMA_MAPPINGS
    for skill_name, schema_types in SCHEMA_MAPPINGS.items():
        for schema_name, file_suffix in schema_types:
            if schema_name not in cli_schemas:
                print(f"  Warning: Schema '{schema_name}' not found, skipping")
                continue

            # Convert the schema to JSON Schema format
            json_schema = format_schema_as_json(cli_schemas[schema_name])
            if not json_schema:
                print(f"  Warning: Empty schema for '{schema_name}', skipping")
                continue

            # Convert to YAML
            schema_yaml = yaml.dump(json_schema, default_flow_style=False, sort_keys=False, allow_unicode=True)

            # Write to file
            schema_file = OUTPUT_SCHEMAS_DIR / f"{file_suffix}.schema.yaml"
            schema_file.write_text(schema_yaml)

            # Store for return
            schema_yaml_content[file_suffix] = schema_yaml

    print(f"  Generated {len(schema_yaml_content)} schema files")
    return schema_yaml_content


# =============================================================================
# Workspace Tool Zod Schema Generation
# =============================================================================


WORKSPACE_TOOL_ZOD_SCHEMAS = [
    ('NewSchedule', 'scheduleRequestSchema'),
    ('NewHttpTrigger', 'httpTriggerRequestSchema'),
    ('NewWebsocketTrigger', 'websocketTriggerRequestSchema'),
    ('NewKafkaTrigger', 'kafkaTriggerRequestSchema'),
    ('NewNatsTrigger', 'natsTriggerRequestSchema'),
    ('NewPostgresTrigger', 'postgresTriggerRequestSchema'),
    ('NewMqttTrigger', 'mqttTriggerRequestSchema'),
    ('NewSqsTrigger', 'sqsTriggerRequestSchema'),
    ('GcpTriggerData', 'gcpTriggerRequestSchema'),
    ('AzureTriggerData', 'azureTriggerRequestSchema'),
]

WORKSPACE_TOOL_TRIGGER_SCHEMAS = [
    ('http', 'httpTriggerRequestSchema'),
    ('websocket', 'websocketTriggerRequestSchema'),
    ('kafka', 'kafkaTriggerRequestSchema'),
    ('nats', 'natsTriggerRequestSchema'),
    ('postgres', 'postgresTriggerRequestSchema'),
    ('mqtt', 'mqttTriggerRequestSchema'),
    ('sqs', 'sqsTriggerRequestSchema'),
    ('gcp', 'gcpTriggerRequestSchema'),
    ('azure', 'azureTriggerRequestSchema'),
]

WORKSPACE_TOOL_ZOD_OUTPUT_PATH = (
    SCRIPT_DIR.parent
    / 'frontend'
    / 'src'
    / 'lib'
    / 'components'
    / 'copilot'
    / 'chat'
    / 'workspaceToolsZod.gen.ts'
)


def _resolve_schema_refs(schema: dict, backend_schemas: dict, openflow_schemas: dict, seen: tuple[str, ...] = ()) -> dict:
    """Resolve OpenAPI refs so json-schema-to-zod emits concrete enums/objects."""
    if isinstance(schema, list):
        return [_resolve_schema_refs(item, backend_schemas, openflow_schemas, seen) for item in schema]

    if not isinstance(schema, dict):
        return schema

    if '$ref' in schema:
        ref = schema['$ref']
        ref_name = ref.split('/')[-1]
        if ref_name in seen:
            return {'type': 'object'}

        source = openflow_schemas if 'openflow.openapi.yaml' in ref or ref_name not in backend_schemas else backend_schemas
        ref_schema = source.get(ref_name)
        if not ref_schema:
            return {'type': 'object'}

        resolved = _resolve_schema_refs(copy.deepcopy(ref_schema), backend_schemas, openflow_schemas, (*seen, ref_name))
        for key, value in schema.items():
            if key != '$ref':
                resolved[key] = _resolve_schema_refs(value, backend_schemas, openflow_schemas, seen)
        return resolved

    return {
        key: _resolve_schema_refs(value, backend_schemas, openflow_schemas, seen)
        for key, value in schema.items()
    }


def _ts_string(value: str) -> str:
    return json.dumps(value)


def _zod_literal(value) -> str:
    return json.dumps(value)


def _apply_zod_metadata(expr: str, schema: dict) -> str:
    if schema.get('description'):
        expr += f".describe({_ts_string(schema['description'])})"
    if schema.get('nullable'):
        expr += ".nullable()"
    if 'default' in schema:
        expr += f".default({_zod_literal(schema['default'])})"
    return expr


def _json_schema_to_zod(schema: dict, indent: int = 0) -> str:
    schema = schema or {}

    if 'oneOf' in schema:
        raise ValueError('Unsupported oneOf in workspace tool Zod schema generation')

    if 'allOf' in schema:
        raise ValueError('Unsupported allOf in workspace tool Zod schema generation')

    if 'anyOf' in schema:
        expr = "z.union([{}])".format(
            ', '.join(_json_schema_to_zod(item, indent) for item in schema['anyOf'])
        )
        return _apply_zod_metadata(expr, schema)

    if 'enum' in schema:
        enum_values = ', '.join(_zod_literal(value) for value in schema['enum'])
        expr = f"z.enum([{enum_values}])"
        return _apply_zod_metadata(expr, schema)

    schema_type = schema.get('type')

    if schema_type == 'string':
        expr = 'z.string()'
        if schema.get('format') == 'date-time':
            expr += '.datetime({ offset: true })'
    elif schema_type == 'boolean':
        expr = 'z.boolean()'
    elif schema_type in ('number', 'integer'):
        expr = 'z.number()'
        if schema_type == 'integer':
            expr += '.int()'
        if 'minimum' in schema:
            expr += f".gte({_zod_literal(schema['minimum'])})"
        if 'maximum' in schema:
            expr += f".lte({_zod_literal(schema['maximum'])})"
    elif schema_type == 'array':
        expr = f"z.array({_json_schema_to_zod(schema.get('items', {}), indent)})"
    elif schema_type == 'object' or schema.get('properties') is not None or schema.get('additionalProperties') is not None:
        properties = schema.get('properties') or {}
        if not properties and schema.get('additionalProperties'):
            expr = 'z.record(z.string(), z.any())'
        else:
            required = set(schema.get('required') or [])
            prop_lines = []
            child_indent = '\t' * (indent + 1)
            closing_indent = '\t' * indent
            for key, value in properties.items():
                prop_expr = _json_schema_to_zod(value, indent + 1)
                if key not in required:
                    prop_expr += '.optional()'
                prop_lines.append(f"{child_indent}{_ts_string(key)}: {prop_expr}")
            if prop_lines:
                expr = "z.object({\n" + ",\n".join(prop_lines) + f"\n{closing_indent}}})"
            else:
                expr = 'z.object({})'
    else:
        expr = 'z.any()'

    return _apply_zod_metadata(expr, schema)


def generate_workspace_tool_zod_schemas(backend_schemas: dict, openflow_schemas: dict) -> None:
    """Generate Zod schemas used by frontend AI chat workspace mutation tools."""
    print("Generating workspace tool Zod schemas...")

    missing = [schema_name for schema_name, _ in WORKSPACE_TOOL_ZOD_SCHEMAS if schema_name not in backend_schemas]
    if missing:
        print(f"  Warning: Missing schemas for workspace tool Zod generation: {', '.join(missing)}")
        return

    trigger_path_description = (
        backend_schemas.get('NewHttpTrigger', {})
        .get('properties', {})
        .get('path', {})
        .get('description')
        or "The new trigger's Windmill path"
    )

    lines = [
        "// Auto-generated by generate.py - DO NOT EDIT",
        "",
        "import { z } from 'zod'",
        "",
    ]

    for schema_name, export_name in WORKSPACE_TOOL_ZOD_SCHEMAS:
        schema = _resolve_schema_refs(
            copy.deepcopy(backend_schemas[schema_name]),
            backend_schemas,
            openflow_schemas,
        )
        lines.append(f"export const {export_name} = {_json_schema_to_zod(schema)}")
        lines.append("")

    lines.extend([
        "export const triggerRequestSchemas = {",
        *[
            f"\t{kind}: {schema_name},"
            for kind, schema_name in WORKSPACE_TOOL_TRIGGER_SCHEMAS
        ],
        "} as const",
        "",
        f"const triggerPathSchema = z.string().min(1).describe({_ts_string(trigger_path_description)})",
        "",
        "export const createTriggerToolSchema = z.object({",
        "\tkind: z.enum([",
        *[
            f"\t\t{_ts_string(kind)},"
            for kind, _ in WORKSPACE_TOOL_TRIGGER_SCHEMAS
        ],
        "\t]),",
        "\tpath: triggerPathSchema,",
        "\tconfig: z.union([",
    ])
    for kind, schema_name in WORKSPACE_TOOL_TRIGGER_SCHEMAS:
        lines.append(f"\t\t{schema_name}.omit({{ path: true, script_path: true, is_flow: true }}),")
    lines.extend([
        "\t])",
        "})",
    ])
    lines.append("")

    WORKSPACE_TOOL_ZOD_OUTPUT_PATH.write_text("\n".join(lines))
    print("  Generated workspaceToolsZod.gen.ts")


# =============================================================================
# Datatable SDK Extraction
# =============================================================================


TS_SQL_UTILS_PATH = TS_SDK_DIR / "sqlUtils.ts"


def extract_datatable_ts_sdk() -> str:
    """Extract datatable-specific type definitions from TypeScript SDK (sqlUtils.ts).

    Reads the source file and extracts the public API surface:
    - SqlStatement<T> type (fetch, fetchOne, fetchOneScalar, execute methods)
    - DatatableSqlTemplateFunction interface (template tag + query method)
    - datatable() function signature
    """
    if not TS_SQL_UTILS_PATH.exists():
        print(f"  Warning: sqlUtils.ts not found at {TS_SQL_UTILS_PATH}")
        return ''

    content = TS_SQL_UTILS_PATH.read_text()

    md = "## TypeScript Datatable API (windmill-client)\n\n"
    md += "Import: `import * as wmill from 'windmill-client'`\n\n"

    # Extract exported type/interface/function definitions from sqlUtils.ts
    # We use extract_balanced to handle nested braces correctly

    # 1. Extract SqlStatement<T> type
    match = re.search(r'(\/\*\*(?:[^*]|\*(?!\/))*\*\/\s*)?export\s+type\s+SqlStatement<T>\s*=\s*', content)
    if match:
        jsdoc_raw = match.group(1)
        brace_start = content.index('{', match.end() - 1)
        body, end = extract_balanced(content, brace_start, '{', '}')
        if end != -1:
            if jsdoc_raw:
                md += clean_jsdoc(jsdoc_raw) + "\n"
            md += "```typescript\n"
            md += f"type SqlStatement<T> = {{\n{_indent_body(body)}\n}};\n"
            md += "```\n\n"

    # 2. Extract DatatableSqlTemplateFunction interface
    match = re.search(
        r'(\/\*\*(?:[^*]|\*(?!\/))*\*\/\s*)?export\s+interface\s+DatatableSqlTemplateFunction\s+extends\s+SqlTemplateFunction\s*',
        content
    )
    if match:
        brace_start = content.index('{', match.end() - 1)
        body, end = extract_balanced(content, brace_start, '{', '}')
        if end != -1:
            md += "```typescript\n"
            md += "// Template tag function: sql`SELECT * FROM table WHERE id = ${id}`.fetch()\n"
            md += f"interface DatatableSqlTemplateFunction {{\n"
            md += f"  // Tagged template usage:\n"
            md += f"  <T = any>(strings: TemplateStringsArray, ...values: any[]): SqlStatement<T>;\n"
            md += f"{_indent_body(body)}\n"
            md += "};\n"
            md += "```\n\n"

    # 3. Extract datatable() function
    match = re.search(
        r'(\/\*\*(?:[^*]|\*(?!\/))*\*\/\s*)?export\s+function\s+datatable\s*\(([^)]*)\)\s*:\s*(\S+)',
        content
    )
    if match:
        jsdoc_raw, params, return_type = match.groups()
        if jsdoc_raw:
            md += clean_jsdoc(jsdoc_raw) + "\n"
        md += "```typescript\n"
        md += f"function datatable({params.strip()}): {return_type}\n"
        md += "```\n"

    return md


def extract_datatable_py_sdk(py_content: str) -> str:
    """Extract datatable-specific class/function definitions from Python SDK.

    Uses Python AST to extract:
    - datatable() function
    - DataTableClient class with query() method
    - SqlQuery class with fetch(), fetch_one(), fetch_one_scalar(), execute() methods
    """
    if not py_content:
        return ''

    try:
        tree = ast.parse(py_content)
    except SyntaxError as e:
        print(f"  Warning: Could not parse Python SDK for datatable extraction: {e}")
        return ''

    md = "## Python Datatable API (wmill)\n\n"
    md += "Import: `import wmill`\n\n"

    # Target classes and the top-level datatable function
    target_classes = {'DataTableClient', 'SqlQuery'}

    # 1. Extract datatable() top-level function
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == 'datatable':
            docstring = ast.get_docstring(node) or ''
            params = _format_py_params(node)
            return_ann = f" -> {ast.unparse(node.returns)}" if node.returns else ''
            if docstring:
                for line in docstring.split('\n'):
                    md += f"# {line}\n"
            md += f"def datatable({params}){return_ann}\n\n"
            break

    # 2. Extract target classes with their public methods
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name in target_classes:
            class_doc = ast.get_docstring(node) or ''
            if class_doc:
                for line in class_doc.split('\n'):
                    md += f"# {line}\n"
            md += f"class {node.name}:\n"

            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if item.name.startswith('_') and item.name != '__init__':
                        continue
                    docstring = ast.get_docstring(item) or ''
                    params = _format_py_params(item, skip_self=True)
                    return_ann = f" -> {ast.unparse(item.returns)}" if item.returns else ''
                    async_prefix = 'async ' if isinstance(item, ast.AsyncFunctionDef) else ''
                    if docstring:
                        for line in docstring.split('\n'):
                            md += f"    # {line}\n"
                    md += f"    {async_prefix}def {item.name}({params}){return_ann}\n\n"

            md += "\n"

    return md


def _format_py_params(node: ast.FunctionDef, skip_self: bool = False) -> str:
    """Format function parameters from AST node."""
    params = []
    args = node.args
    num_defaults = len(args.defaults)
    num_args = len(args.args)

    for i, arg in enumerate(args.args):
        if skip_self and arg.arg == 'self':
            continue
        param_str = arg.arg
        if arg.annotation:
            param_str += f": {ast.unparse(arg.annotation)}"
        default_idx = i - (num_args - num_defaults)
        if default_idx >= 0:
            default = args.defaults[default_idx]
            param_str += f" = {ast.unparse(default)}"
        params.append(param_str)

    if args.vararg:
        vararg_str = f"*{args.vararg.arg}"
        if args.vararg.annotation:
            vararg_str += f": {ast.unparse(args.vararg.annotation)}"
        params.append(vararg_str)

    for i, arg in enumerate(args.kwonlyargs):
        param_str = arg.arg
        if arg.annotation:
            param_str += f": {ast.unparse(arg.annotation)}"
        if args.kw_defaults[i]:
            param_str += f" = {ast.unparse(args.kw_defaults[i])}"
        params.append(param_str)

    if args.kwarg:
        kwarg_str = f"**{args.kwarg.arg}"
        if args.kwarg.annotation:
            kwarg_str += f": {ast.unparse(args.kwarg.annotation)}"
        params.append(kwarg_str)

    return ', '.join(params)


def _indent_body(body: str) -> str:
    """Clean and re-indent a type body for readable output."""
    lines = body.strip().split('\n')
    result = []
    for line in lines:
        stripped = line.strip()
        if stripped:
            # Keep JSDoc comments and method signatures with consistent indentation
            if not stripped.startswith('//') and not stripped.startswith('/*') and not stripped.startswith('*'):
                result.append(f"  {stripped}")
            else:
                result.append(f"  {stripped}")
        else:
            result.append('')
    return '\n'.join(result)


# =============================================================================
# Workflow-as-Code SDK Extraction
# =============================================================================


WAC_TS_FUNCTIONS = [
    'getResumeUrls',
    'task',
    'taskScript',
    'taskFlow',
    'workflow',
    'step',
    'sleep',
    'waitForApproval',
    'parallel',
]

WAC_PY_FUNCTIONS = [
    'get_resume_urls',
    'task',
    'task_script',
    'task_flow',
    'workflow',
    'step',
    'sleep',
    'wait_for_approval',
    'parallel',
]


def _extract_ts_angle_params(content: str, start_pos: int) -> tuple[str, int]:
    """Extract TypeScript generic parameters, ignoring arrow `=>` tokens."""
    if start_pos >= len(content) or content[start_pos] != '<':
        return '', start_pos

    depth = 0
    i = start_pos
    quote: str | None = None
    while i < len(content):
        char = content[i]
        prev = content[i - 1] if i > 0 else ''

        if quote:
            if char == '\\':
                i += 2
                continue
            if char == quote:
                quote = None
            i += 1
            continue

        if char in ('"', "'", '`'):
            quote = char
        elif char == '<':
            depth += 1
        elif char == '>' and prev != '=':
            depth -= 1
            if depth == 0:
                return content[start_pos:i + 1], i + 1
        i += 1

    return '', -1


def _render_ts_jsdoc(jsdoc_raw: str | None) -> str:
    if not jsdoc_raw:
        return ''

    docstring = clean_jsdoc(jsdoc_raw)
    if not docstring:
        return ''

    lines = ["/**"]
    for line in docstring.split('\n'):
        lines.append(f" * {line}" if line else " *")
    lines.append(" */")
    return '\n'.join(lines)


def _extract_ts_interface(content: str, name: str) -> str:
    pattern = re.compile(
        r'(?:(/\*\*(?:[^*]|\*(?!/))*\*/)\s*)?'
        rf'export\s+interface\s+{re.escape(name)}\s*',
        re.MULTILINE
    )
    match = pattern.search(content)
    if not match:
        return ''

    try:
        brace_start = content.index('{', match.end() - 1)
    except ValueError:
        return ''

    body, end = extract_balanced(content, brace_start, '{', '}')
    if end == -1:
        return ''

    parts = []
    jsdoc = _render_ts_jsdoc(match.group(1))
    if jsdoc:
        parts.append(jsdoc)
    parts.append(f"export interface {name} {{\n{_indent_body(body)}\n}}")
    return '\n'.join(parts)


def _extract_ts_exported_function(content: str, name: str) -> str:
    pattern = re.compile(
        r'(?:(/\*\*(?:[^*]|\*(?!/))*\*/)\s*)?'
        rf'export\s+(async\s+)?function\s+{re.escape(name)}\s*',
        re.MULTILINE
    )
    match = pattern.search(content)
    if not match:
        return ''

    jsdoc_raw, is_async = match.groups()
    pos = match.end()
    while pos < len(content) and content[pos] in ' \t\n':
        pos += 1

    generic = ''
    if pos < len(content) and content[pos] == '<':
        generic, pos = _extract_ts_angle_params(content, pos)
        if pos == -1:
            return ''
        while pos < len(content) and content[pos] in ' \t\n':
            pos += 1

    if pos >= len(content) or content[pos] != '(':
        return ''

    params, paren_end = extract_balanced(content, pos, '(', ')')
    if paren_end == -1:
        return ''

    return_type, _ = extract_return_type(content, paren_end + 1)
    async_prefix = 'async ' if is_async else ''
    signature = f"export {async_prefix}function {name}{generic}({clean_params(params)})"
    if return_type:
        signature += f": {clean_params(return_type)}"

    parts = []
    jsdoc = _render_ts_jsdoc(jsdoc_raw)
    if jsdoc:
        parts.append(jsdoc)
    parts.append(signature)
    return '\n'.join(parts)


def extract_wac_ts_sdk(ts_content: str) -> str:
    """Extract Workflow-as-Code API signatures from the TypeScript SDK."""
    if not ts_content:
        return ''

    declarations = []
    task_options = _extract_ts_interface(ts_content, 'TaskOptions')
    if task_options:
        declarations.append(task_options)

    for function_name in WAC_TS_FUNCTIONS:
        signature = _extract_ts_exported_function(ts_content, function_name)
        if signature:
            declarations.append(signature)
        else:
            print(f"  Warning: TypeScript WAC function '{function_name}' not found")

    if not declarations:
        return ''

    md = "## TypeScript Workflow-as-Code API (windmill-client)\n\n"
    md += 'Import: `import { workflow, task, taskScript, taskFlow, step, sleep, waitForApproval, getResumeUrls, parallel } from "windmill-client"`\n\n'
    md += "```typescript\n"
    md += "\n\n".join(declarations)
    md += "\n```\n"
    return md


def _format_py_params_exact(node, skip_self: bool = False) -> str:
    """Format Python parameters from AST, preserving bare * for keyword-only args."""
    params = []
    args = node.args

    positional = list(args.posonlyargs) + list(args.args)
    num_defaults = len(args.defaults)
    num_positional = len(positional)

    for i, arg in enumerate(positional):
        if skip_self and arg.arg == 'self':
            continue
        param_str = arg.arg
        if arg.annotation:
            param_str += f": {ast.unparse(arg.annotation)}"
        default_idx = i - (num_positional - num_defaults)
        if default_idx >= 0:
            param_str += f" = {ast.unparse(args.defaults[default_idx])}"
        params.append(param_str)

    if args.vararg:
        vararg_str = f"*{args.vararg.arg}"
        if args.vararg.annotation:
            vararg_str += f": {ast.unparse(args.vararg.annotation)}"
        params.append(vararg_str)
    elif args.kwonlyargs:
        params.append('*')

    for i, arg in enumerate(args.kwonlyargs):
        param_str = arg.arg
        if arg.annotation:
            param_str += f": {ast.unparse(arg.annotation)}"
        if args.kw_defaults[i] is not None:
            param_str += f" = {ast.unparse(args.kw_defaults[i])}"
        params.append(param_str)

    if args.kwarg:
        kwarg_str = f"**{args.kwarg.arg}"
        if args.kwarg.annotation:
            kwarg_str += f": {ast.unparse(args.kwarg.annotation)}"
        params.append(kwarg_str)

    return ', '.join(params)


def _render_py_docstring(docstring: str, indent: str = '') -> str:
    if not docstring:
        return ''
    return '\n'.join(f"{indent}# {line}" if line else f"{indent}#" for line in docstring.split('\n'))


def _extract_py_function_signature(tree: ast.Module, name: str) -> str:
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == name:
            docstring = ast.get_docstring(node) or ''
            params = _format_py_params_exact(node)
            return_ann = f" -> {ast.unparse(node.returns)}" if node.returns else ''
            async_prefix = 'async ' if isinstance(node, ast.AsyncFunctionDef) else ''
            parts = []
            rendered_docstring = _render_py_docstring(docstring)
            if rendered_docstring:
                parts.append(rendered_docstring)
            parts.append(f"{async_prefix}def {node.name}({params}){return_ann}")
            return '\n'.join(parts)
    return ''


def _extract_py_class_signature(tree: ast.Module, name: str) -> str:
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == name:
            parts = []
            docstring = _render_py_docstring(ast.get_docstring(node) or '')
            if docstring:
                parts.append(docstring)
            bases = f"({', '.join(ast.unparse(base) for base in node.bases)})" if node.bases else ''
            parts.append(f"class {node.name}{bases}:")
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)) and item.name == '__init__':
                    init_docstring = _render_py_docstring(ast.get_docstring(item) or '', indent='    ')
                    if init_docstring:
                        parts.append(init_docstring)
                    params = _format_py_params_exact(item)
                    parts.append(f"    def __init__({params})")
                    break
            return '\n'.join(parts)
    return ''


def extract_wac_py_sdk(py_content: str) -> str:
    """Extract Workflow-as-Code API signatures from the Python SDK."""
    if not py_content:
        return ''

    try:
        tree = ast.parse(py_content)
    except SyntaxError as e:
        print(f"  Warning: Could not parse Python SDK for WAC extraction: {e}")
        return ''

    declarations = []
    task_error = _extract_py_class_signature(tree, 'TaskError')
    if task_error:
        declarations.append(task_error)

    for function_name in WAC_PY_FUNCTIONS:
        signature = _extract_py_function_signature(tree, function_name)
        if signature:
            declarations.append(signature)
        else:
            print(f"  Warning: Python WAC function '{function_name}' not found")

    if not declarations:
        return ''

    md = "## Python Workflow-as-Code API (wmill)\n\n"
    md += "Import: `from wmill import workflow, task, task_script, task_flow, step, sleep, wait_for_approval, get_resume_urls, parallel, TaskError`\n\n"
    md += "```python\n"
    md += "\n\n".join(declarations)
    md += "\n```\n"
    return md


# =============================================================================
# Skill Generation
# =============================================================================


def generate_skill_content(
    skill_name: str,
    description: str,
    intro: str,
    content: str,
    sdk_content: str = ''
) -> str:
    """Generate a skill file with YAML frontmatter."""
    parts = [
        "---",
        f"name: {skill_name}",
        f"description: {description}",
        "---",
        "",
    ]
    if intro:
        parts.extend([intro, ""])
    parts.append(content)
    if sdk_content:
        parts.extend(["", sdk_content])
    return '\n'.join(parts)


# Skill definitions for config-driven generation
SKILL_DEFINITIONS = [
    {
        'name': 'write-flow',
        'description': 'MUST use when creating flows.',
        'content_key': 'flow',
    },
    {
        'name': 'raw-app',
        'description': 'MUST use when creating raw apps.',
        'content_key': 'raw_app',
    },
    {
        'name': 'triggers',
        'description': 'MUST use when configuring triggers.',
        'content_key': 'triggers',
        'schema_types': [
            ('HttpTrigger', 'http_trigger'),
            ('WebsocketTrigger', 'websocket_trigger'),
            ('KafkaTrigger', 'kafka_trigger'),
            ('NatsTrigger', 'nats_trigger'),
            ('PostgresTrigger', 'postgres_trigger'),
            ('MqttTrigger', 'mqtt_trigger'),
            ('SqsTrigger', 'sqs_trigger'),
            ('GcpTrigger', 'gcp_trigger'),
            ('AzureTrigger', 'azure_trigger'),
        ],
    },
    {
        'name': 'schedules',
        'description': 'MUST use when configuring schedules.',
        'content_key': 'schedules',
        'schema_types': [('Schedule', 'schedule')],
    },
    {
        'name': 'resources',
        'description': 'MUST use when managing resources.',
        'content_key': 'resources',
    },
    {
        'name': 'write-workflow-as-code',
        'description': 'MUST use when writing or modifying Windmill Workflow-as-Code scripts using workflow, task, step, sleep, approvals, taskScript, taskFlow, task_script, or task_flow.',
        'content_key': 'workflow_as_code',
        'intro_key': 'wac_cli',
        'sdk_content_key': 'wac',
    },
    {
        'name': 'cli-commands',
        'description': 'MUST use when using the CLI, including debugging job failures and inspecting run history via `wmill job`.',
        'content_key': 'cli_commands',
    },
    {
        'name': 'preview',
        'description': 'MUST use when opening the Windmill dev page / visual preview of a flow, script, or app. Triggers on words like preview, open, navigate to, visualize, see the flow/app/script, and after writing a flow/script/app for visual verification.',
        'content_key': 'preview',
    },
]


def generate_skills(
    languages: dict[str, str],
    ts_sdk_md: str,
    py_sdk_md: str,
    wac_ts_md: str,
    wac_py_md: str,
    flow_cli: str,
    flow_base: str,
    openflow_content: str,
    cli_commands: str,
    cli_schemas: dict[str, dict] | None = None
):
    """Generate individual skill files for Claude Code."""
    print("Generating skill files...")

    cli_schemas = cli_schemas or {}

    # Ensure skills directory exists
    OUTPUT_SKILLS_DIR.mkdir(parents=True, exist_ok=True)

    # Read base files for additional skills
    base_dir = SCRIPT_DIR / "base"
    base_content = {
        'flow': f"{flow_cli}\n\n{flow_base}\n\n{openflow_content}",
        'raw_app': read_markdown_file(base_dir / "raw-app.md"),
        'triggers': read_markdown_file(base_dir / "triggers.md"),
        'schedules': read_markdown_file(base_dir / "schedules.md"),
        'resources': read_markdown_file(base_dir / "resources.md"),
        'workflow_as_code': read_markdown_file(base_dir / "workflow-as-code.md"),
        'cli_commands': cli_commands,
        'preview': read_markdown_file(base_dir / "preview.md"),
    }

    # CLI intro for script skills
    script_cli_intro = """## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — generate `.script.yaml` and `.lock` files for the script you modified.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill sync push` and `wmill generate-metadata` modify workspace state or local files — only run these when the user explicitly asks; otherwise tell them which to run.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types."""

    wac_cli_intro = f"""{script_cli_intro}

Workflow-as-Code files use the normal script CLI workflow. There are no separate WAC deploy commands."""

    intro_content = {
        'wac_cli': wac_cli_intro,
    }

    extra_sdk_content = {
        'wac': "\n\n".join(filter(None, [wac_ts_md, wac_py_md])),
    }

    skills_generated = []

    # Generate script skills for each language
    for lang_key, lang_content in languages.items():
        if lang_key not in LANGUAGE_METADATA:
            print(f"  Warning: No metadata for language '{lang_key}', skipping")
            continue

        metadata = LANGUAGE_METADATA[lang_key]
        skill_name = f"write-script-{lang_key}"
        skill_dir = OUTPUT_SKILLS_DIR / skill_name
        skill_dir.mkdir(parents=True, exist_ok=True)

        # Determine which SDK to include
        language_sdk_content = ''
        if lang_key in TS_SDK_LANGUAGES:
            language_sdk_content = ts_sdk_md
        elif lang_key in PY_SDK_LANGUAGES:
            language_sdk_content = py_sdk_md

        skill_content = generate_skill_content(
            skill_name=skill_name,
            description=metadata['description'],
            intro=script_cli_intro,
            content=lang_content,
            sdk_content=language_sdk_content
        )

        (skill_dir / "SKILL.md").write_text(skill_content)
        skills_generated.append(skill_name)

    # Generate other skills from definitions
    # Note: Skills with schema_types (triggers, schedules) get base content only.
    # Schemas are stored separately and combined at CLI init time.
    for skill_def in SKILL_DEFINITIONS:
        content = base_content.get(skill_def['content_key'], '')
        if not content:
            continue

        skill_name = skill_def['name']
        skill_dir = OUTPUT_SKILLS_DIR / skill_name
        skill_dir.mkdir(parents=True, exist_ok=True)

        # Note: We no longer append schemas here. Skills with 'schema_types'
        # will have schemas combined at CLI init time from SCHEMAS export.

        skill_content = generate_skill_content(
            skill_name=skill_name,
            description=skill_def['description'],
            intro=intro_content.get(skill_def.get('intro_key', ''), ''),
            content=content,
            sdk_content=extra_sdk_content.get(skill_def.get('sdk_content_key', ''), '')
        )

        (skill_dir / "SKILL.md").write_text(skill_content)
        skills_generated.append(skill_name)

    print(f"  Generated {len(skills_generated)} skills")
    return skills_generated


def generate_skills_ts_export(skills: list[str], schema_yaml_content: dict[str, str] | None = None) -> str:
    """Generate TypeScript file that exports skill metadata for the CLI.

    Args:
        skills: List of skill names
        schema_yaml_content: Dict mapping schema keys (e.g., 'http_trigger') to YAML content
    """
    schema_yaml_content = schema_yaml_content or {}

    ts = "// Auto-generated by generate.py - DO NOT EDIT\n\n"
    ts += "export interface SkillMetadata {\n"
    ts += "  name: string;\n"
    ts += "  description: string;\n"
    ts += "  languageKey?: string;\n"
    ts += "}\n\n"

    ts += "export const SKILLS: SkillMetadata[] = [\n"

    skill_desc_map = {s['name']: s['description'] for s in SKILL_DEFINITIONS}

    for skill in skills:
        if skill.startswith('write-script-'):
            lang_key = skill.replace('write-script-', '')
            if lang_key in LANGUAGE_METADATA:
                metadata = LANGUAGE_METADATA[lang_key]
                ts += f'  {{ name: "{skill}", description: "{metadata["description"]}", languageKey: "{lang_key}" }},\n'
        elif skill in skill_desc_map:
            ts += f'  {{ name: "{skill}", description: "{skill_desc_map[skill]}" }},\n'

    ts += "];\n\n"

    # Generate the skills content inline for bundling
    ts += "// Skill content for each skill (loaded inline for bundling)\n"
    ts += "export const SKILL_CONTENT: Record<string, string> = {\n"

    # We'll read the generated files and embed them
    for skill in skills:
        skill_path = OUTPUT_SKILLS_DIR / skill / "SKILL.md"
        if skill_path.exists():
            content = skill_path.read_text()
            escaped = escape_for_ts(content)
            ts += f'  "{skill}": `{escaped}`,\n'

    ts += "};\n\n"

    # Generate SCHEMAS export (YAML content for each schema)
    ts += "// YAML schema content for triggers and schedules\n"
    ts += "export const SCHEMAS: Record<string, string> = {\n"

    for schema_key, yaml_content in sorted(schema_yaml_content.items()):
        escaped = escape_for_ts(yaml_content)
        ts += f'  "{schema_key}": `{escaped}`,\n'

    ts += "};\n\n"

    # Generate SCHEMA_MAPPINGS export (maps skill names to their schemas)
    ts += "// Maps skill names to their schema types and file patterns\n"
    ts += "export interface SchemaMapping {\n"
    ts += "  name: string;\n"
    ts += "  schemaKey: string;\n"
    ts += "  filePattern: string;\n"
    ts += "}\n\n"
    ts += "export const SCHEMA_MAPPINGS: Record<string, SchemaMapping[]> = {\n"

    for skill_name, schema_types in SCHEMA_MAPPINGS.items():
        ts += f'  "{skill_name}": [\n'
        for schema_name, file_suffix in schema_types:
            ts += f'    {{ name: "{schema_name}", schemaKey: "{file_suffix}", filePattern: "*.{file_suffix}.yaml" }},\n'
        ts += "  ],\n"

    ts += "};\n"

    return ts


def format_schema_for_markdown(schema_yaml: str, schema_name: str, file_pattern: str) -> str:
    """Format a standalone schema block for plugin skill files."""
    return f"""## {schema_name} (`{file_pattern}`)

Must be a YAML file that adheres to the following schema:

```yaml
{schema_yaml.strip()}
```"""


def render_plugin_skill_content(skill_name: str, schema_yaml_content: dict[str, str]) -> str:
    """Render plugin-ready skill content from generated base skill files."""
    skill_path = OUTPUT_SKILLS_DIR / skill_name / "SKILL.md"
    if not skill_path.exists():
        raise FileNotFoundError(f"Missing generated skill content for {skill_name}: {skill_path}")

    skill_content = skill_path.read_text()
    schema_mappings = SCHEMA_MAPPINGS.get(skill_name, [])
    if not schema_mappings:
        return skill_content

    schema_docs = []
    for schema_name, schema_key in schema_mappings:
        schema_yaml = schema_yaml_content.get(schema_key)
        if not schema_yaml:
            continue
        schema_docs.append(
            format_schema_for_markdown(
                schema_yaml=schema_yaml,
                schema_name=schema_name,
                file_pattern=f"*.{schema_key}.yaml",
            )
        )

    if not schema_docs:
        return skill_content

    return f"{skill_content}\n\n" + "\n\n".join(schema_docs)


def resolve_plugin_skills_dir(plugin_dir: Path) -> Path:
    """Resolve the plugin skills directory from a repo root, plugin root, or skills dir."""
    plugin_dir = plugin_dir.expanduser().resolve()

    repo_skills_dir = plugin_dir / "plugins" / "windmill-code-plugin" / "skills"
    repo_plugin_json = plugin_dir / "plugins" / "windmill-code-plugin" / ".claude-plugin" / "plugin.json"
    if repo_plugin_json.exists():
        return repo_skills_dir

    plugin_skills_dir = plugin_dir / "skills"
    plugin_json = plugin_dir / ".claude-plugin" / "plugin.json"
    if plugin_json.exists():
        return plugin_skills_dir

    if plugin_dir.name == "skills":
        return plugin_dir

    return plugin_skills_dir


def generate_plugin_skills(
    plugin_dir: Path,
    skills: list[str],
    schema_yaml_content: dict[str, str],
) -> Path:
    """Generate standalone skills in a Claude plugin checkout."""
    skills_dir = resolve_plugin_skills_dir(plugin_dir)
    skills_dir.mkdir(parents=True, exist_ok=True)

    expected_skills = set(skills)
    for existing in skills_dir.iterdir():
        if existing.is_dir() and existing.name not in expected_skills:
            shutil.rmtree(existing)

    for skill_name in skills:
        skill_dir = skills_dir / skill_name
        skill_dir.mkdir(parents=True, exist_ok=True)
        (skill_dir / "SKILL.md").write_text(
            render_plugin_skill_content(skill_name, schema_yaml_content)
        )

    print(f"\nGenerated for plugin:")
    print(f"  - {skills_dir} ({len(skills)} skills)")
    return skills_dir


# =============================================================================
# Main Entry Point
# =============================================================================


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description=(
            "Generate Windmill system prompts, CLI guidance, and optionally "
            "plugin-ready standalone skills."
        )
    )
    parser.add_argument(
        "--plugin-dir",
        type=Path,
        help=(
            "Optional plugin target. Accepts a windmill-claude-plugin repo root, "
            "a plugin root, or a skills directory, and refreshes standalone skills there."
        ),
    )
    return parser.parse_args()


def main():
    """Main generation function."""
    args = parse_args()

    print("Generating system prompts documentation...")

    # Ensure output directories exist
    OUTPUT_SDKS_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUT_GENERATED_DIR.mkdir(parents=True, exist_ok=True)

    # Read SDK files
    ts_content = ''
    if TS_SDK_DIR.exists():
        for ts_file in sorted(TS_SDK_DIR.glob('*.ts')):
            if not ts_file.name.endswith('.d.ts'):
                ts_content += ts_file.read_text() + '\n'
    py_content = PY_SDK_PATH.read_text() if PY_SDK_PATH.exists() else ''
    openflow_raw = OPENFLOW_SCHEMA_PATH.read_text() if OPENFLOW_SCHEMA_PATH.exists() else ''

    # Extract only components.schemas from OpenFlow and convert to minified JSON
    openflow_yaml = yaml.safe_load(openflow_raw) if openflow_raw else {}
    openflow_schemas = openflow_yaml.get('components', {}).get('schemas', {})
    openflow_schemas_json = json.dumps(openflow_schemas, separators=(',', ':'))
    openflow_content = f"## OpenFlow Schema\n\n{openflow_schemas_json}"

    # Extract TypeScript SDK info
    print("Parsing TypeScript SDK...")
    ts_functions = extract_ts_functions(ts_content)
    ts_types = extract_ts_types(ts_content)
    ts_sdk_md = generate_ts_sdk_markdown(ts_functions, ts_types)
    (OUTPUT_SDKS_DIR / "typescript.md").write_text(ts_sdk_md)
    print(f"  Found {len(ts_functions)} functions, {len(ts_types)} types")

    # Extract Python SDK info
    print("Parsing Python SDK...")
    py_functions = extract_py_functions(py_content)
    py_classes = extract_py_classes(py_content)
    py_sdk_md = generate_py_sdk_markdown(py_functions, py_classes)
    (OUTPUT_SDKS_DIR / "python.md").write_text(py_sdk_md)
    print(f"  Found {len(py_functions)} functions, {len(py_classes)} classes")

    # Extract datatable-specific SDK docs (for app mode system prompt)
    print("Extracting datatable SDK docs...")
    datatable_ts_md = extract_datatable_ts_sdk()
    datatable_py_md = extract_datatable_py_sdk(py_content)
    (OUTPUT_SDKS_DIR / "datatable-typescript.md").write_text(datatable_ts_md)
    (OUTPUT_SDKS_DIR / "datatable-python.md").write_text(datatable_py_md)

    # Extract Workflow-as-Code SDK docs (for WAC skills and prompt helpers)
    print("Extracting Workflow-as-Code SDK docs...")
    wac_ts_md = extract_wac_ts_sdk(ts_content)
    wac_py_md = extract_wac_py_sdk(py_content)
    (OUTPUT_SDKS_DIR / "wac-typescript.md").write_text(wac_ts_md)
    (OUTPUT_SDKS_DIR / "wac-python.md").write_text(wac_py_md)

    # Read base prompts
    print("Assembling complete prompts...")
    base_dir = SCRIPT_DIR / "base"
    languages_dir = SCRIPT_DIR / "languages"

    script_base = read_markdown_file(base_dir / "script-base.md")
    flow_base = read_markdown_file(base_dir / "flow-base.md")
    workflow_as_code_base = read_markdown_file(base_dir / "workflow-as-code.md")
    flow_cli = read_markdown_file(base_dir / "flow-cli.md")
    flow_chat_special_modules = read_markdown_file(base_dir / "flow-chat-special-modules.md")

    # Read language files
    languages = {}
    for lang_file in sorted(languages_dir.glob("*.md")):
        languages[lang_file.stem] = lang_file.read_text()

    # Extract and generate CLI commands documentation
    print("Extracting CLI commands...")
    cli_data = extract_cli_commands()
    cli_commands = generate_cli_commands_markdown(cli_data)
    OUTPUT_CLI_DIR.mkdir(parents=True, exist_ok=True)
    (OUTPUT_CLI_DIR / "cli-commands.md").write_text(cli_commands)
    print(f"  Found {len(cli_data['commands'])} commands, {len(cli_data['global_options'])} global options")

    # Extract schemas from backend OpenAPI for CLI format documentation
    print("Extracting backend OpenAPI schemas...")
    cli_schemas = {}
    backend_schemas = {}
    if BACKEND_OPENAPI_PATH.exists():
        backend_openapi_raw = BACKEND_OPENAPI_PATH.read_text()
        backend_openapi = yaml.safe_load(backend_openapi_raw)
        backend_schemas = backend_openapi.get('components', {}).get('schemas', {})

        # Extract and transform schemas for CLI format (removing server-managed fields)
        schema_names = [
            'Schedule', 'NewSchedule',
            'HttpTrigger', 'NewHttpTrigger',
            'WebsocketTrigger', 'NewWebsocketTrigger',
            'KafkaTrigger', 'NewKafkaTrigger',
            'NatsTrigger', 'NewNatsTrigger',
            'PostgresTrigger', 'NewPostgresTrigger',
            'MqttTrigger', 'NewMqttTrigger',
            'SqsTrigger', 'NewSqsTrigger',
            'GcpTrigger',
            'AzureTrigger',
        ]
        for schema_name in schema_names:
            if schema_name in backend_schemas:
                cli_schemas[schema_name] = extract_cli_schema(backend_schemas[schema_name], backend_schemas, openflow_schemas)

        print(f"  Extracted {len(cli_schemas)} schemas for CLI format")
    else:
        print(f"  Warning: Backend OpenAPI file not found at {BACKEND_OPENAPI_PATH}")

    # Generate standalone schema files for triggers and schedules
    schema_yaml_content = generate_schema_files(cli_schemas)
    generate_workspace_tool_zod_schemas(backend_schemas, openflow_schemas)

    # Assemble prompts for export
    prompts = {
        # Base prompts
        'SCRIPT_BASE': script_base,
        'FLOW_BASE': flow_base,
        'WORKFLOW_AS_CODE_BASE': workflow_as_code_base,
        'FLOW_CHAT_SPECIAL_MODULES': flow_chat_special_modules,

        # SDKs
        'SDK_TYPESCRIPT': ts_sdk_md,
        'SDK_PYTHON': py_sdk_md,
        'WAC_SDK_TYPESCRIPT': wac_ts_md,
        'WAC_SDK_PYTHON': wac_py_md,

        # Datatable-specific SDK docs (for app mode)
        'DATATABLE_SDK_TYPESCRIPT': datatable_ts_md,
        'DATATABLE_SDK_PYTHON': datatable_py_md,

        # Schema (raw YAML content)
        'OPENFLOW_SCHEMA': openflow_content,

        # CLI
        'CLI_COMMANDS': cli_commands,
    }

    # Add language prompts
    for lang_name, lang_content in languages.items():
        prompts[f'LANG_{lang_name.upper()}'] = lang_content

    # Generate TypeScript exports
    ts_exports = generate_ts_exports(prompts)
    (OUTPUT_GENERATED_DIR / "prompts.ts").write_text(ts_exports)

    # Generate complete script.md (all languages combined)
    script_md_parts = [script_base]
    for lang_name in sorted(languages.keys()):
        script_md_parts.append(languages[lang_name])
    script_md_parts.extend([ts_sdk_md, py_sdk_md])
    script_md = "\n\n".join(filter(None, script_md_parts))
    (OUTPUT_GENERATED_DIR / "script.md").write_text(script_md)

    # Generate complete flow.md
    flow_md_parts = [flow_base, openflow_content]
    flow_md = "\n\n".join(filter(None, flow_md_parts))
    (OUTPUT_GENERATED_DIR / "flow.md").write_text(flow_md)

    # Generate an index file
    index_content = """// Auto-generated by generate.py - DO NOT EDIT
// Re-export all prompts
export * from './prompts';

import * as prompts from './prompts';

// Languages that use the TypeScript SDK
const TS_SDK_LANGUAGES = ['bun', 'deno', 'nativets', 'bunnative'];

// Languages that use the Python SDK
const PY_SDK_LANGUAGES = ['python3'];

// Helper to combine prompts for scripts
export function getScriptPrompt(language: string): string {
  const langKey = `LANG_${language.toUpperCase()}` as keyof typeof prompts;
  const langPrompt = (prompts as Record<string, string>)[langKey] || '';

  // Determine which SDK to include based on language
  let sdkPrompt = '';
  if (TS_SDK_LANGUAGES.includes(language)) {
    sdkPrompt = prompts.SDK_TYPESCRIPT;
  } else if (PY_SDK_LANGUAGES.includes(language)) {
    sdkPrompt = prompts.SDK_PYTHON;
  }

  return [
    prompts.SCRIPT_BASE,
    langPrompt,
    sdkPrompt
  ].filter(Boolean).join('\\n\\n');
}

// Helper to combine prompts for flows
export function getFlowPrompt(): string {
  return [
    prompts.FLOW_BASE,
    prompts.OPENFLOW_SCHEMA
  ].filter(Boolean).join('\\n\\n');
}

// Helper to get datatable SDK reference for app mode
export function getDatatableSdkReference(): string {
  return [
    prompts.DATATABLE_SDK_TYPESCRIPT,
    prompts.DATATABLE_SDK_PYTHON
  ].filter(Boolean).join('\\n\\n');
}

// Helper to combine prompts for Workflow-as-Code scripts
export function getWorkflowAsCodePrompt(): string {
  return [
    prompts.WORKFLOW_AS_CODE_BASE,
    prompts.WAC_SDK_TYPESCRIPT,
    prompts.WAC_SDK_PYTHON
  ].filter(Boolean).join('\\n\\n');
}
"""
    (OUTPUT_GENERATED_DIR / "index.ts").write_text(index_content)

    # Generate skill files for Claude Code
    CLI_GUIDANCE_DIR.mkdir(parents=True, exist_ok=True)
    skills = generate_skills(
        languages=languages,
        ts_sdk_md=ts_sdk_md,
        py_sdk_md=py_sdk_md,
        wac_ts_md=wac_ts_md,
        wac_py_md=wac_py_md,
        flow_cli=flow_cli,
        flow_base=flow_base,
        cli_commands=cli_commands,
        openflow_content=openflow_content,
        cli_schemas=cli_schemas
    )

    # Generate skills TypeScript export for CLI
    skills_ts = generate_skills_ts_export(skills, schema_yaml_content)

    # Replace hardcoded path conventions with placeholders for CLI runtime resolution.
    # init.ts resolves these based on the nonDottedPaths setting in wmill.yaml.
    # (Frontend auto-generated files keep the default non-dotted conventions.)
    skills_ts = (skills_ts
        .replace("\\`__flow\\`", "\\`{{FLOW_SUFFIX}}\\`")
        .replace(
            "Inline script files should NOT include \\`.inline_script.\\`"
            " in their names (e.g. use \\`a.ts\\`, not \\`a.inline_script.ts\\`).",
            "{{INLINE_SCRIPT_NAMING}}"
        )
        .replace("my_flow__flow", "my_flow{{FLOW_SUFFIX}}")
        .replace("my_app__raw_app/", "my_app{{RAW_APP_SUFFIX}}/")
    )
    (CLI_GUIDANCE_DIR / "skills.gen.ts").write_text(skills_ts)

    print(f"\nGenerated files:")
    print(f"  - auto-generated/sdks/typescript.md")
    print(f"  - auto-generated/sdks/python.md")
    print(f"  - auto-generated/sdks/wac-typescript.md")
    print(f"  - auto-generated/sdks/wac-python.md")
    print(f"  - auto-generated/cli/cli-commands.md (auto-generated from CLI source)")
    print(f"  - auto-generated/prompts.ts")
    print(f"  - auto-generated/index.ts")
    print(f"  - auto-generated/script.md")
    print(f"  - auto-generated/flow.md")
    print(f"  - auto-generated/skills/ ({len(skills)} skills)")
    print(f"  - auto-generated/schemas/ ({len(schema_yaml_content)} schema files)")
    print(f"\nGenerated for CLI:")
    print(f"  - cli/src/guidance/skills.gen.ts")

    if args.plugin_dir:
        generate_plugin_skills(args.plugin_dir, skills, schema_yaml_content)

    print("\nDone!")


if __name__ == '__main__':
    main()
