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
import json
import re
from pathlib import Path

import yaml

# Paths relative to this script
SCRIPT_DIR = Path(__file__).parent
ROOT_DIR = SCRIPT_DIR.parent

TS_SDK_DIR = ROOT_DIR / "typescript-client"
PY_SDK_PATH = ROOT_DIR / "python-client" / "wmill" / "wmill" / "client.py"
OPENFLOW_SCHEMA_PATH = ROOT_DIR / "openflow.openapi.yaml"

OUTPUT_SDKS_DIR = SCRIPT_DIR / "sdks"
OUTPUT_GENERATED_DIR = SCRIPT_DIR / "generated"

# CLI guidance directory (DNT can't import from outside cli/, so we copy files there)
CLI_GUIDANCE_DIR = ROOT_DIR / "cli" / "src" / "guidance"


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


def extract_ts_functions(content: str) -> list[dict]:
    """Extract exported function signatures from TypeScript SDK."""
    functions = []

    # Pattern for JSDoc comment followed by exported function
    # Captures: /** full docstring */ export [async] function name(params): ReturnType {
    jsdoc_pattern = re.compile(
        r'(/\*\*[\s\S]*?\*/)\s*'  # Capture entire JSDoc comment
        r'export\s+(async\s+)?function\s+(\w+)\s*'  # export [async] function name
        r'(<[^>]+>)?\s*'  # optional generic
        r'\(([^)]*)\)\s*'  # parameters
        r'(?::\s*([^{]+))?\s*\{',  # return type
        re.MULTILINE
    )

    # Pattern for exported functions without JSDoc
    func_pattern = re.compile(
        r'export\s+(async\s+)?function\s+(\w+)\s*'  # export [async] function name
        r'(<[^>]+>)?\s*'  # optional generic
        r'\(([^)]*)\)\s*'  # parameters
        r'(?::\s*([^{]+))?\s*\{',  # return type
        re.MULTILINE | re.DOTALL
    )

    # First, find all functions with JSDoc
    for match in jsdoc_pattern.finditer(content):
        jsdoc_raw, is_async, name, generic, params, return_type = match.groups()
        docstring = clean_jsdoc(jsdoc_raw)
        functions.append({
            'name': name,
            'generic': generic or '',
            'params': clean_params(params),
            'return_type': (return_type or ('Promise<void>' if is_async else 'void')).strip(),
            'async': bool(is_async),
            'docstring': docstring
        })

    # Then find functions without JSDoc (that weren't already captured)
    for match in func_pattern.finditer(content):
        is_async, name, generic, params, return_type = match.groups()
        if not any(f['name'] == name for f in functions):
            functions.append({
                'name': name,
                'generic': generic or '',
                'params': clean_params(params),
                'return_type': (return_type or ('Promise<void>' if is_async else 'void')).strip(),
                'async': bool(is_async),
                'docstring': ''
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


def generate_ts_sdk_markdown(functions: list[dict], types: list[dict]) -> str:
    """Generate compact documentation for TypeScript SDK."""
    md = "# TypeScript SDK (windmill-client)\n\n"
    md += "Import: import * as wmill from 'windmill-client'\n\n"

    for i, func in enumerate(functions):
        if func.get('docstring'):
            # Format multi-line docstrings with // prefix on each line
            docstring_lines = func['docstring'].split('\n')
            for line in docstring_lines:
                md += f"// {line}\n"
        async_prefix = 'async ' if func['async'] else ''
        md += f"{async_prefix}{func['name']}{func['generic']}({func['params']}): {func['return_type']}"
        md += "\n"
        if i < len(functions) - 1:
            md += "\n"

    return md


def generate_py_sdk_markdown(functions: list[dict], classes: list[dict]) -> str:
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
    OUTPUT_GENERATED_DIR.mkdir(parents=True, exist_ok=True)

    # Read SDK files
    ts_content = ''
    if TS_SDK_DIR.exists():
        for ts_file in TS_SDK_DIR.glob('*.ts'):
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

    # Read base prompts
    print("Assembling complete prompts...")
    base_dir = SCRIPT_DIR / "base"
    languages_dir = SCRIPT_DIR / "languages"

    script_base = read_markdown_file(base_dir / "script-base.md")
    flow_base = read_markdown_file(base_dir / "flow-base.md")

    # Read language files
    languages = {}
    for lang_file in languages_dir.glob("*.md"):
        languages[lang_file.stem] = lang_file.read_text()

    # Read CLI files
    cli_dir = SCRIPT_DIR / "cli"
    cli_commands = read_markdown_file(cli_dir / "cli-commands.md")

    # Assemble prompts for export
    prompts = {
        # Base prompts
        'SCRIPT_BASE': script_base,
        'FLOW_BASE': flow_base,

        # SDKs
        'SDK_TYPESCRIPT': ts_sdk_md,
        'SDK_PYTHON': py_sdk_md,

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
"""
    (OUTPUT_GENERATED_DIR / "index.ts").write_text(index_content)

    # Generate CLI-specific prompts.ts with only SCRIPT_PROMPT and FLOW_PROMPT
    print("Generating CLI prompts...")
    CLI_GUIDANCE_DIR.mkdir(parents=True, exist_ok=True)
    cli_prompts = {
        'SCRIPT_PROMPT': script_md,
        'FLOW_PROMPT': flow_md,
        'CLI_COMMANDS': cli_commands,
    }
    cli_prompts_ts = generate_ts_exports(cli_prompts)
    (CLI_GUIDANCE_DIR / "prompts.ts").write_text(cli_prompts_ts)

    print(f"\nGenerated files:")
    print(f"  - sdks/typescript.md")
    print(f"  - sdks/python.md")
    print(f"  - generated/prompts.ts")
    print(f"  - generated/index.ts")
    print(f"  - generated/script.md")
    print(f"  - generated/flow.md")
    print(f"\nGenerated for CLI:")
    print(f"  - cli/src/guidance/prompts.ts")
    print("\nDone!")


if __name__ == '__main__':
    main()
