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
"""

import ast
import os
import re
import yaml
from pathlib import Path
from typing import Any

# Paths relative to this script
SCRIPT_DIR = Path(__file__).parent
ROOT_DIR = SCRIPT_DIR.parent

TS_SDK_PATH = ROOT_DIR / "typescript-client" / "client.ts"
PY_SDK_PATH = ROOT_DIR / "python-client" / "wmill" / "wmill" / "client.py"
OPENFLOW_SCHEMA_PATH = ROOT_DIR / "openflow.openapi.yaml"

OUTPUT_SDKS_DIR = SCRIPT_DIR / "sdks"
OUTPUT_SCHEMAS_DIR = SCRIPT_DIR / "schemas"
OUTPUT_GENERATED_DIR = SCRIPT_DIR / "generated"


def extract_ts_functions(content: str) -> list[dict]:
    """Extract exported function signatures from TypeScript SDK."""
    functions = []

    # Pattern for exported async functions
    # Matches: export async function name(params): ReturnType {
    async_pattern = re.compile(
        r'export\s+async\s+function\s+(\w+)\s*'  # function name
        r'(<[^>]+>)?\s*'  # optional generic
        r'\(([^)]*)\)\s*'  # parameters
        r'(?::\s*([^{]+))?\s*\{',  # return type
        re.MULTILINE | re.DOTALL
    )

    # Pattern for exported sync functions
    sync_pattern = re.compile(
        r'export\s+function\s+(\w+)\s*'  # function name
        r'(<[^>]+>)?\s*'  # optional generic
        r'\(([^)]*)\)\s*'  # parameters
        r'(?::\s*([^{]+))?\s*\{',  # return type
        re.MULTILINE | re.DOTALL
    )

    for match in async_pattern.finditer(content):
        name, generic, params, return_type = match.groups()
        functions.append({
            'name': name,
            'generic': generic or '',
            'params': clean_params(params),
            'return_type': (return_type or 'Promise<void>').strip(),
            'async': True
        })

    for match in sync_pattern.finditer(content):
        name, generic, params, return_type = match.groups()
        # Skip if already captured as async
        if not any(f['name'] == name for f in functions):
            functions.append({
                'name': name,
                'generic': generic or '',
                'params': clean_params(params),
                'return_type': (return_type or 'void').strip(),
                'async': False
            })

    return functions


def clean_params(params: str) -> str:
    """Clean up parameter string."""
    if not params:
        return ''
    # Remove excessive whitespace and newlines
    params = re.sub(r'\s+', ' ', params).strip()
    return params


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


def extract_py_functions(content: str) -> list[dict]:
    """Extract function signatures from Python SDK using AST."""
    functions = []

    try:
        tree = ast.parse(content)
    except SyntaxError as e:
        print(f"Warning: Could not parse Python SDK: {e}")
        return functions

    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef) or isinstance(node, ast.AsyncFunctionDef):
            # Skip private functions and class methods (handled separately)
            if node.name.startswith('_') and not node.name.startswith('__'):
                continue

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

            functions.append({
                'name': node.name,
                'params': ', '.join(params),
                'return_type': return_type,
                'docstring': docstring.split('\n')[0] if docstring else '',  # First line only
                'async': isinstance(node, ast.AsyncFunctionDef)
            })

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
                            'docstring': docstring.split('\n')[0] if docstring else ''
                        })

            classes.append({
                'name': node.name,
                'docstring': ast.get_docstring(node) or '',
                'methods': methods
            })

    return classes


def parse_openflow_schema(content: str) -> dict:
    """Parse OpenFlow YAML schema."""
    return yaml.safe_load(content)


def generate_ts_sdk_markdown(functions: list[dict], types: list[dict]) -> str:
    """Generate markdown documentation for TypeScript SDK."""
    md = "# TypeScript SDK (windmill-client)\n\n"
    md += "Import: `import * as wmill from 'windmill-client'`\n\n"

    # Group functions by category
    categories = {
        'Scripts & Flows': ['runScript', 'runScriptAsync', 'runScriptByPath', 'runScriptByHash',
                          'runFlow', 'runFlowAsync', 'waitJob', 'getResult', 'getJobStatus'],
        'Variables': ['getVariable', 'setVariable'],
        'Resources': ['getResource', 'setResource', 'getResourceValue'],
        'State': ['getState', 'setState', 'setFlowUserState', 'getFlowUserState'],
        'S3 Operations': ['loadS3File', 'loadS3FileStream', 'writeS3File'],
        'Progress & Logging': ['setProgress', 'getProgress', 'appendLog'],
        'Approval & Resume': ['getResumeUrls', 'resume', 'cancel'],
        'Utilities': ['getWorkspace', 'getUser', 'getRootJobId']
    }

    md += "## Functions\n\n"

    categorized = set()
    for category, names in categories.items():
        category_funcs = [f for f in functions if any(n.lower() in f['name'].lower() for n in names)]
        if category_funcs:
            md += f"### {category}\n\n"
            for func in category_funcs:
                categorized.add(func['name'])
                async_prefix = 'async ' if func['async'] else ''
                md += f"```typescript\n{async_prefix}function {func['name']}{func['generic']}({func['params']}): {func['return_type']}\n```\n\n"

    # Uncategorized functions
    uncategorized = [f for f in functions if f['name'] not in categorized]
    if uncategorized:
        md += "### Other\n\n"
        for func in uncategorized:
            async_prefix = 'async ' if func['async'] else ''
            md += f"```typescript\n{async_prefix}function {func['name']}{func['generic']}({func['params']}): {func['return_type']}\n```\n\n"

    # Types
    if types:
        md += "## Types\n\n"
        for t in types[:10]:  # Limit to important types
            if t['kind'] == 'type':
                md += f"```typescript\ntype {t['name']} = {t['definition']}\n```\n\n"
            else:
                md += f"```typescript\ninterface {t['name']} {{\n  {t['definition']}\n}}\n```\n\n"

    return md


def generate_py_sdk_markdown(functions: list[dict], classes: list[dict]) -> str:
    """Generate markdown documentation for Python SDK."""
    md = "# Python SDK (wmill)\n\n"
    md += "Import: `import wmill`\n\n"

    # Module-level functions
    md += "## Module Functions\n\n"
    md += "These functions use a global client instance automatically initialized from environment variables.\n\n"

    # Key functions to document
    key_functions = [
        'get_resource', 'set_resource', 'list_resources',
        'get_variable', 'set_variable',
        'get_state', 'set_state',
        'run_script_async', 'run_script_by_path', 'run_script_by_hash',
        'run_flow_async', 'wait_job', 'get_result', 'get_job_status',
        'load_s3_file', 'load_s3_file_reader', 'write_s3_file',
        'set_progress', 'get_progress',
        'set_flow_user_state', 'get_flow_user_state',
        'get_resume_urls', 'cancel_job',
        'whoami', 'username_to_email',
        'duckdb_connection_settings', 'polars_connection_settings', 'boto3_connection_settings'
    ]

    for func in functions:
        if func['name'] in key_functions:
            async_prefix = 'async ' if func['async'] else ''
            return_annotation = f" -> {func['return_type']}" if func['return_type'] else ''
            md += f"```python\n{async_prefix}def {func['name']}({func['params']}){return_annotation}\n```\n"
            if func['docstring']:
                md += f"{func['docstring']}\n"
            md += "\n"

    # Windmill class
    md += "## Windmill Class\n\n"
    md += "For more control, instantiate the Windmill class directly:\n\n"
    md += "```python\nfrom wmill import Windmill\n\nclient = Windmill()\n```\n\n"

    for cls in classes:
        if cls['name'] == 'Windmill':
            md += "### Key Methods\n\n"
            for method in cls['methods'][:20]:  # Limit methods shown
                md += f"- `{method['name']}()` - {method['docstring']}\n"

    # S3Object type
    md += "\n## Types\n\n"
    md += "```python\nfrom wmill import S3Object\n\n# S3Object is a TypedDict with 's3' key for the path\ns3obj = S3Object(s3=\"path/to/file.txt\")\n```\n\n"

    return md


def generate_openflow_markdown(schema: dict) -> str:
    """Generate markdown documentation for OpenFlow schema."""
    md = "# OpenFlow Schema\n\n"
    md += "The OpenFlow schema defines the structure of Windmill flows.\n\n"

    components = schema.get('components', {}).get('schemas', {})

    # Document key schemas in order
    key_schemas = [
        'OpenFlow', 'FlowValue', 'FlowModule', 'FlowModuleValue',
        'InputTransform', 'StaticTransform', 'JavascriptTransform',
        'RawScript', 'PathScript', 'PathFlow',
        'ForloopFlow', 'WhileloopFlow', 'BranchOne', 'BranchAll',
        'Identity', 'Retry', 'StopAfterIf'
    ]

    for schema_name in key_schemas:
        if schema_name in components:
            schema_def = components[schema_name]
            md += f"## {schema_name}\n\n"

            if 'description' in schema_def:
                md += f"{schema_def['description']}\n\n"

            if schema_def.get('type') == 'object' and 'properties' in schema_def:
                md += "**Properties:**\n\n"
                for prop_name, prop_def in schema_def['properties'].items():
                    prop_type = prop_def.get('type', '')
                    if '$ref' in prop_def:
                        prop_type = prop_def['$ref'].split('/')[-1]
                    desc = prop_def.get('description', '')
                    required = prop_name in schema_def.get('required', [])
                    req_marker = ' (required)' if required else ''
                    md += f"- `{prop_name}`: {prop_type}{req_marker}"
                    if desc:
                        md += f" - {desc}"
                    md += "\n"
                md += "\n"

            if 'oneOf' in schema_def:
                md += "**One of:**\n\n"
                for option in schema_def['oneOf']:
                    if '$ref' in option:
                        ref_name = option['$ref'].split('/')[-1]
                        md += f"- {ref_name}\n"
                md += "\n"

            if 'enum' in schema_def:
                md += f"**Values:** {', '.join(f'`{v}`' for v in schema_def['enum'])}\n\n"

    return md


def escape_for_ts(content: str) -> str:
    """Escape content for TypeScript template literal."""
    return content.replace('\\', '\\\\').replace('`', '\\`').replace('${', '\\${')


def generate_ts_exports(prompts: dict[str, str]) -> str:
    """Generate TypeScript file that exports all prompts."""
    ts = "// Auto-generated by generate.py - DO NOT EDIT\n\n"

    for name, content in prompts.items():
        escaped = escape_for_ts(content)
        ts += f"export const {name} = `{escaped}`;\n\n"

    return ts


def read_markdown_file(path: Path) -> str:
    """Read a markdown file and return its content."""
    if path.exists():
        return path.read_text()
    return ''


def main():
    """Main generation function."""
    print("Generating system prompts documentation...")

    # Ensure output directories exist
    OUTPUT_SDKS_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUT_SCHEMAS_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUT_GENERATED_DIR.mkdir(parents=True, exist_ok=True)

    # Read SDK files
    ts_content = TS_SDK_PATH.read_text() if TS_SDK_PATH.exists() else ''
    py_content = PY_SDK_PATH.read_text() if PY_SDK_PATH.exists() else ''
    openflow_content = OPENFLOW_SCHEMA_PATH.read_text() if OPENFLOW_SCHEMA_PATH.exists() else ''

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

    # Parse OpenFlow schema
    print("Parsing OpenFlow schema...")
    if openflow_content:
        openflow_schema = parse_openflow_schema(openflow_content)
        openflow_md = generate_openflow_markdown(openflow_schema)
        (OUTPUT_SCHEMAS_DIR / "openflow.md").write_text(openflow_md)
        num_schemas = len(openflow_schema.get('components', {}).get('schemas', {}))
        print(f"  Found {num_schemas} schema definitions")

    # Read base prompts
    print("Assembling complete prompts...")
    base_dir = SCRIPT_DIR / "base"
    languages_dir = SCRIPT_DIR / "languages"
    resources_dir = SCRIPT_DIR / "resources"

    script_base = read_markdown_file(base_dir / "script-base.md")
    flow_base = read_markdown_file(base_dir / "flow-base.md")

    # Read language files
    languages = {}
    for lang_file in languages_dir.glob("*.md"):
        languages[lang_file.stem] = lang_file.read_text()

    # Read resource files
    resource_types = read_markdown_file(resources_dir / "resource-types.md")
    s3_objects = read_markdown_file(resources_dir / "s3-objects.md")

    # Assemble prompts for export
    prompts = {
        # Base prompts
        'SCRIPT_BASE': script_base,
        'FLOW_BASE': flow_base,

        # SDKs
        'SDK_TYPESCRIPT': ts_sdk_md,
        'SDK_PYTHON': py_sdk_md,

        # Schema
        'OPENFLOW_SCHEMA': openflow_md if openflow_content else '',

        # Resources
        'RESOURCE_TYPES': resource_types,
        'S3_OBJECTS': s3_objects,
    }

    # Add language prompts
    for lang_name, lang_content in languages.items():
        prompts[f'LANG_{lang_name.upper()}'] = lang_content

    # Generate TypeScript exports
    ts_exports = generate_ts_exports(prompts)
    (OUTPUT_GENERATED_DIR / "prompts.ts").write_text(ts_exports)

    # Generate an index file
    index_content = """// Auto-generated by generate.py - DO NOT EDIT
// Re-export all prompts
export * from './prompts';

import * as prompts from './prompts';

// Helper to combine prompts for scripts
export function getScriptPrompt(language: string): string {
  const langKey = `LANG_${language.toUpperCase()}` as keyof typeof prompts;
  const langPrompt = (prompts as Record<string, string>)[langKey] || '';

  return [
    prompts.SCRIPT_BASE,
    langPrompt,
    prompts.RESOURCE_TYPES,
    prompts.S3_OBJECTS,
    language === 'typescript' || language === 'bun' || language === 'deno'
      ? prompts.SDK_TYPESCRIPT
      : language === 'python3'
        ? prompts.SDK_PYTHON
        : ''
  ].filter(Boolean).join('\\n\\n');
}

// Helper to combine prompts for flows
export function getFlowPrompt(): string {
  return [
    prompts.FLOW_BASE,
    prompts.OPENFLOW_SCHEMA,
    prompts.RESOURCE_TYPES
  ].filter(Boolean).join('\\n\\n');
}
"""
    (OUTPUT_GENERATED_DIR / "index.ts").write_text(index_content)

    print(f"\nGenerated files:")
    print(f"  - sdks/typescript.md")
    print(f"  - sdks/python.md")
    print(f"  - schemas/openflow.md")
    print(f"  - generated/prompts.ts")
    print(f"  - generated/index.ts")
    print("\nDone!")


if __name__ == '__main__':
    main()
