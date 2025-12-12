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

OUTPUT_SDKS_DIR = SCRIPT_DIR / "auto-generated" / "sdks"
OUTPUT_GENERATED_DIR = SCRIPT_DIR / "auto-generated"
OUTPUT_CLI_DIR = SCRIPT_DIR / "auto-generated" / "cli"

# CLI guidance directory (DNT can't import from outside cli/, so we copy files there)
CLI_GUIDANCE_DIR = ROOT_DIR / "cli" / "src" / "guidance"

# CLI source paths for extracting command documentation
CLI_DIR = ROOT_DIR / "cli"
CLI_MAIN = CLI_DIR / "src" / "main.ts"
CLI_COMMANDS_DIR = CLI_DIR / "src" / "commands"


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


def extract_ts_functions(content: str) -> list[dict]:
    """Extract exported function signatures from TypeScript SDK."""
    functions = []
    seen_names = set()

    # Pattern to find JSDoc followed by export function declaration
    # Only captures up to the function name and optional generic
    jsdoc_func_pattern = re.compile(
        r'(/\*\*(?:[^*]|\*(?!/))*\*/)\s*'  # JSDoc comment
        r'export\s+(async\s+)?function\s+(\w+)\s*'  # export [async] function name
        r'(<[^>]+>)?\s*',  # optional generic
        re.MULTILINE
    )

    # Pattern to find export function without JSDoc
    func_pattern = re.compile(
        r'export\s+(async\s+)?function\s+(\w+)\s*'  # export [async] function name
        r'(<[^>]+>)?\s*',  # optional generic
        re.MULTILINE
    )

    # First, find all functions with JSDoc
    for match in jsdoc_func_pattern.finditer(content):
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

        docstring = clean_jsdoc(jsdoc_raw)
        seen_names.add(name)
        functions.append({
            'name': name,
            'generic': generic or '',
            'params': clean_params(params),
            'return_type': return_type,
            'async': bool(is_async),
            'docstring': docstring
        })

    # Then find functions without JSDoc (that weren't already captured)
    for match in func_pattern.finditer(content):
        is_async, name, generic = match.groups()

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

        seen_names.add(name)
        functions.append({
            'name': name,
            'generic': generic or '',
            'params': clean_params(params),
            'return_type': return_type,
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


def parse_command_block(content: str) -> dict:
    """
    Parse a Cliffy Command() definition block and extract metadata.
    Returns a dict with: description, options, subcommands, arguments, alias
    """
    result = {
        'description': '',
        'options': [],
        'subcommands': [],
        'arguments': '',
        'alias': ''
    }

    # Find the command block - starts with "new Command()" or "const command = new Command()"
    # and ends with "export default"
    command_match = re.search(
        r'(?:const\s+command\s*=\s*)?new\s+Command\(\)([\s\S]*?)(?=export\s+default)',
        content
    )
    if not command_match:
        return result

    block = command_match.group(1)

    # Extract main description (first one in the block, before any subcommand)
    first_subcommand_pos = block.find('.command(')
    if first_subcommand_pos == -1:
        first_subcommand_pos = len(block)
    top_section = block[:first_subcommand_pos]

    # Handle multi-line descriptions with template literals or string concatenation
    # Pattern: .description("text") or .description(\n  "text",\n)
    desc_match = re.search(r'\.description\(\s*["\']([^"\']+)["\']\s*,?\s*\)', top_section, re.DOTALL)
    if not desc_match:
        # Try matching descriptions that use template literals
        desc_match = re.search(r'\.description\(\s*`([^`]+)`\s*,?\s*\)', top_section, re.DOTALL)
    if desc_match:
        result['description'] = desc_match.group(1).strip()

    # Extract alias
    alias_match = re.search(r'\.alias\(\s*["\']([^"\']+)["\']\s*\)', top_section)
    if alias_match:
        result['alias'] = alias_match.group(1)

    # Extract options pattern
    option_pattern = re.compile(
        r'\.option\(\s*["\']([^"\']+)["\']\s*,\s*["\']([^"\']+)["\']\s*\)',
        re.MULTILINE
    )

    # Extract top-level options (before any .command() or .action())
    top_section_until_action = re.split(r'\.action\(', top_section)[0]
    for match in option_pattern.finditer(top_section_until_action):
        flag, desc = match.groups()
        result['options'].append({'flag': flag, 'description': desc})

    # Extract top-level arguments
    args_match = re.search(r'\.arguments\(\s*["\']([^"\']+)["\']\s*\)', top_section)
    if args_match:
        result['arguments'] = args_match.group(1)

    # Extract subcommands with their arguments, options, and descriptions
    # Split by .command( to find subcommand boundaries
    subcommand_sections = re.split(r'(?=\.command\()', block)

    for section in subcommand_sections:
        # Check if this starts a new subcommand
        cmd_match = re.match(r'\.command\(\s*["\']([^"\']+)["\']\s*(?:,\s*["\']([^"\']+)["\'])?\s*\)', section)
        if cmd_match:
            cmd_name = cmd_match.group(1)
            cmd_desc = cmd_match.group(2) or ''

            # Check for description in chained .description() call
            # Handle multi-line descriptions
            desc_match = re.search(r'\.description\(\s*["\']([^"\']+)["\']\s*,?\s*\)', section, re.DOTALL)
            if desc_match:
                cmd_desc = desc_match.group(1).strip()

            # Check for arguments
            args_match = re.search(r'\.arguments\(\s*["\']([^"\']+)["\']\s*\)', section)
            cmd_args = args_match.group(1) if args_match else ''

            # Check for options specific to this subcommand
            # Only get options that appear before .action()
            cmd_options = []
            section_until_action = re.split(r'\.action\(', section)[0]
            for opt_match in option_pattern.finditer(section_until_action):
                flag, desc = opt_match.groups()
                cmd_options.append({'flag': flag, 'description': desc})

            result['subcommands'].append({
                'name': cmd_name,
                'description': cmd_desc,
                'arguments': cmd_args,
                'options': cmd_options
            })

    return result


def find_command_file(cmd_name: str) -> Path | None:
    """Find the command file for a given command name.

    Convention: directory name should match main command file name.
    E.g., flow/flow.ts, app/app.ts, worker-groups/worker-groups.ts
    """
    # Standard pattern: command-name/command-name.ts
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
    # Pattern: .command("name", importedCommand) or .command("name with desc", ...)
    cmd_reg_pattern = re.compile(
        r'\.command\(\s*["\']([^"\']+)["\']\s*,\s*(\w+)\s*\)',
        re.MULTILINE
    )

    # Also handle inline commands like .command("version --version", "description")
    inline_cmd_pattern = re.compile(
        r'\.command\(\s*["\']([^"\']+)["\']\s*,\s*["\']([^"\']+)["\']\s*\)',
        re.MULTILINE
    )

    registered_commands = []

    for match in cmd_reg_pattern.finditer(main_content):
        cmd_name = match.group(1).split()[0]  # Get just the name, not flags
        registered_commands.append(cmd_name)

    # Process each registered command
    for cmd_name in registered_commands:
        cmd_file = find_command_file(cmd_name)
        if cmd_file:
            try:
                cmd_content = cmd_file.read_text()
                cmd_data = parse_command_block(cmd_content)
                cmd_data['name'] = cmd_name
                result['commands'].append(cmd_data)
            except Exception as e:
                print(f"Warning: Could not parse command file for {cmd_name}: {e}")
        else:
            # Some commands might be inline (like 'version', 'upgrade', 'completions')
            pass

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


def generate_cli_commands_markdown(cli_data: dict) -> str:
    """Generate markdown documentation from extracted CLI command data."""
    md = "# Windmill CLI Commands\n\n"
    md += "The Windmill CLI (`wmill`) provides commands for managing scripts, flows, apps, and other resources.\n\n"

    if cli_data.get('version'):
        md += f"Current version: {cli_data['version']}\n\n"

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


def generate_ts_sdk_markdown(functions: list[dict], types: list[dict]) -> str:
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

    # Extract and generate CLI commands documentation
    print("Extracting CLI commands...")
    cli_data = extract_cli_commands()
    cli_commands = generate_cli_commands_markdown(cli_data)
    OUTPUT_CLI_DIR.mkdir(parents=True, exist_ok=True)
    (OUTPUT_CLI_DIR / "cli-commands.md").write_text(cli_commands)
    print(f"  Found {len(cli_data['commands'])} commands, {len(cli_data['global_options'])} global options")

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
    print(f"  - auto-generated/sdks/typescript.md")
    print(f"  - auto-generated/sdks/python.md")
    print(f"  - auto-generated/cli/cli-commands.md (auto-generated from CLI source)")
    print(f"  - auto-generated/prompts.ts")
    print(f"  - auto-generated/index.ts")
    print(f"  - auto-generated/script.md")
    print(f"  - auto-generated/flow.md")
    print(f"\nGenerated for CLI:")
    print(f"  - cli/src/guidance/prompts.ts")
    print("\nDone!")


if __name__ == '__main__':
    main()
