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
BACKEND_OPENAPI_PATH = ROOT_DIR / "backend" / "windmill-api" / "openapi.yaml"

OUTPUT_SDKS_DIR = SCRIPT_DIR / "auto-generated" / "sdks"
OUTPUT_GENERATED_DIR = SCRIPT_DIR / "auto-generated"
OUTPUT_CLI_DIR = SCRIPT_DIR / "auto-generated" / "cli"
OUTPUT_SKILLS_DIR = SCRIPT_DIR / "auto-generated" / "skills"

# Fields stripped by CLI/sync format (to_string_without_metadata equivalent)
# These are server-managed fields that don't appear in YAML/JSON files pulled via CLI
CLI_EXCLUDED_FIELDS = [
    'workspace_id', 'path', 'name', 'versions', 'id',
    'created_at', 'updated_at', 'created_by', 'updated_by',
    'edited_at', 'edited_by', 'archived', 'has_draft',
    'error', 'last_server_ping', 'server_id',
    'extra_perms', 'email', 'enabled', 'mode'
]

# CLI guidance directory (DNT can't import from outside cli/, so we copy files there)
CLI_GUIDANCE_DIR = ROOT_DIR / "cli" / "src" / "guidance"

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

    # Extract options pattern - handles multi-line options and options with config objects
    # Matches: .option("flag", "desc") and .option("flag", "desc", { ... })
    # Uses separate patterns for double and single quoted strings to handle apostrophes
    option_pattern = re.compile(
        r'\.option\(\s*"([^"]+)"\s*,\s*"([^"]+)"'  # double-quoted
        r'|'
        r"\.option\(\s*'([^']+)'\s*,\s*'([^']+)'",  # single-quoted
        re.MULTILINE | re.DOTALL
    )

    # Extract top-level options (before any .command() or .action())
    top_section_until_action = re.split(r'\.action\(', top_section)[0]
    for match in option_pattern.finditer(top_section_until_action):
        groups = match.groups()
        # Pattern has 4 groups: (dq_flag, dq_desc, sq_flag, sq_desc)
        # Either double-quoted or single-quoted pair will be non-None
        flag = groups[0] or groups[2]
        desc = groups[1] or groups[3]
        if flag and desc:
            result['options'].append({'flag': flag, 'description': desc})

    # Extract top-level arguments
    args_match = re.search(r'\.arguments\(\s*["\']([^"\']+)["\']\s*\)', top_section)
    if args_match:
        result['arguments'] = args_match.group(1)

    # Parse imports if we have a file path (for resolving imported subcommands)
    imports = parse_default_imports(content) if file_path else {}

    # Extract subcommands with their arguments, options, and descriptions
    # Split by .command( to find subcommand boundaries
    subcommand_sections = re.split(r'(?=\.command\()', block)

    for section in subcommand_sections:
        # Check if this starts a new subcommand
        # Pattern matches both: .command("name", "description") and .command("name", variableName)
        cmd_match = re.match(r'\.command\(\s*["\']([^"\']+)["\']\s*(?:,\s*([^)]+))?\s*\)', section)
        if cmd_match:
            cmd_name = cmd_match.group(1)
            second_arg = cmd_match.group(2).strip() if cmd_match.group(2) else ''

            # Check if second arg is a string (description) or a variable (imported command)
            is_string_desc = second_arg.startswith('"') or second_arg.startswith("'")

            if is_string_desc:
                # Inline string description
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
                            # Use the imported command's metadata
                            result['subcommands'].append({
                                'name': cmd_name,
                                'description': imported_cmd.get('description', ''),
                                'arguments': imported_cmd.get('arguments', ''),
                                'options': imported_cmd.get('options', [])
                            })
                            continue  # Skip the normal processing below
                        except Exception as e:
                            print(f"  Warning: Could not parse imported command {second_arg}: {e}")
                cmd_desc = ''
            else:
                cmd_desc = ''

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
                groups = opt_match.groups()
                # Pattern has 4 groups: (dq_flag, dq_desc, sq_flag, sq_desc)
                flag = groups[0] or groups[2]
                desc = groups[1] or groups[3]
                if flag and desc:
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
                cmd_data = parse_command_block(cmd_content, cmd_file)
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


def generate_skills(
    languages: dict[str, str],
    ts_sdk_md: str,
    py_sdk_md: str,
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
    script_base = read_markdown_file(base_dir / "script-base.md")
    raw_app_content = read_markdown_file(base_dir / "raw-app.md")
    triggers_content = read_markdown_file(base_dir / "triggers.md")
    schedules_content = read_markdown_file(base_dir / "schedules.md")
    resources_content = read_markdown_file(base_dir / "resources.md")

    # CLI intro for script skills
    script_cli_intro = """## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types."""

    skills_generated = []

    # Languages that use TypeScript SDK
    ts_sdk_languages = ['bun', 'deno', 'nativets', 'bunnative']
    # Languages that use Python SDK
    py_sdk_languages = ['python3']

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
        sdk_content = ''
        if lang_key in ts_sdk_languages:
            sdk_content = ts_sdk_md
        elif lang_key in py_sdk_languages:
            sdk_content = py_sdk_md

        # Language skills only include language-specific content, not the general script-base
        skill_content = generate_skill_content(
            skill_name=skill_name,
            description=metadata['description'],
            intro=script_cli_intro,
            content=lang_content,
            sdk_content=sdk_content
        )

        (skill_dir / "SKILL.md").write_text(skill_content)
        skills_generated.append(skill_name)

    # Generate write-flow skill
    flow_skill_dir = OUTPUT_SKILLS_DIR / "write-flow"
    flow_skill_dir.mkdir(parents=True, exist_ok=True)
    flow_skill_content = generate_skill_content(
        skill_name="write-flow",
        description="MUST use when creating flows.",
        intro="",
        content=f"{flow_base}\n\n{openflow_content}"
    )
    (flow_skill_dir / "SKILL.md").write_text(flow_skill_content)
    skills_generated.append("write-flow")

    # Generate raw-app skill (if content exists)
    if raw_app_content:
        raw_app_skill_dir = OUTPUT_SKILLS_DIR / "raw-app"
        raw_app_skill_dir.mkdir(parents=True, exist_ok=True)
        raw_app_skill_content = generate_skill_content(
            skill_name="raw-app",
            description="MUST use when creating raw apps.",
            intro="",
            content=raw_app_content
        )
        (raw_app_skill_dir / "SKILL.md").write_text(raw_app_skill_content)
        skills_generated.append("raw-app")

    # Generate triggers skill (if content exists)
    if triggers_content:
        triggers_skill_dir = OUTPUT_SKILLS_DIR / "triggers"
        triggers_skill_dir.mkdir(parents=True, exist_ok=True)

        # Generate trigger schemas from OpenAPI as primary documentation
        trigger_schema_docs = []
        trigger_types = [
            ('HttpTrigger', 'http_trigger'),
            ('WebsocketTrigger', 'websocket_trigger'),
            ('KafkaTrigger', 'kafka_trigger'),
            ('NatsTrigger', 'nats_trigger'),
            ('PostgresTrigger', 'postgres_trigger'),
            ('MqttTrigger', 'mqtt_trigger'),
            ('SqsTrigger', 'sqs_trigger'),
            ('GcpTrigger', 'gcp_trigger'),
        ]
        for schema_name, file_suffix in trigger_types:
            if schema_name in cli_schemas:
                schema_doc = format_schema_for_markdown(cli_schemas[schema_name], f"{schema_name} (`*.{file_suffix}.yaml`)", as_json_schema=True)
                if schema_doc:
                    trigger_schema_docs.append(schema_doc)

        triggers_with_schemas = triggers_content
        if trigger_schema_docs:
            triggers_with_schemas = triggers_content + "\n\n" + "\n\n".join(trigger_schema_docs)

        triggers_skill_content = generate_skill_content(
            skill_name="triggers",
            description="MUST use when configuring triggers.",
            intro="",
            content=triggers_with_schemas
        )
        (triggers_skill_dir / "SKILL.md").write_text(triggers_skill_content)
        skills_generated.append("triggers")

    # Generate schedules skill (if content exists)
    if schedules_content:
        schedules_skill_dir = OUTPUT_SKILLS_DIR / "schedules"
        schedules_skill_dir.mkdir(parents=True, exist_ok=True)

        # Append schedule schema from OpenAPI as JSON schema (like triggers)
        schedules_with_schema = schedules_content
        if 'Schedule' in cli_schemas:
            schedule_schema_doc = format_schema_for_markdown(cli_schemas['Schedule'], 'Schedule (`*.schedule.yaml`)', as_json_schema=True)
            if schedule_schema_doc:
                schedules_with_schema = schedules_content + "\n\n" + schedule_schema_doc

        schedules_skill_content = generate_skill_content(
            skill_name="schedules",
            description="MUST use when configuring schedules.",
            intro="",
            content=schedules_with_schema
        )
        (schedules_skill_dir / "SKILL.md").write_text(schedules_skill_content)
        skills_generated.append("schedules")

    # Generate resources skill (if content exists)
    if resources_content:
        resources_skill_dir = OUTPUT_SKILLS_DIR / "resources"
        resources_skill_dir.mkdir(parents=True, exist_ok=True)
        resources_skill_content = generate_skill_content(
            skill_name="resources",
            description="MUST use when managing resources.",
            intro="",
            content=resources_content
        )
        (resources_skill_dir / "SKILL.md").write_text(resources_skill_content)
        skills_generated.append("resources")

    # Generate cli-commands skill
    if cli_commands:
        cli_skill_dir = OUTPUT_SKILLS_DIR / "cli-commands"
        cli_skill_dir.mkdir(parents=True, exist_ok=True)
        cli_skill_content = generate_skill_content(
            skill_name="cli-commands",
            description="MUST use when using the CLI.",
            intro="",
            content=cli_commands
        )
        (cli_skill_dir / "SKILL.md").write_text(cli_skill_content)
        skills_generated.append("cli-commands")

    print(f"  Generated {len(skills_generated)} skills")
    return skills_generated


def generate_skills_ts_export(skills: list[str]) -> str:
    """Generate TypeScript file that exports skill metadata for the CLI."""
    ts = "// Auto-generated by generate.py - DO NOT EDIT\n\n"
    ts += "export interface SkillMetadata {\n"
    ts += "  name: string;\n"
    ts += "  description: string;\n"
    ts += "  languageKey?: string;\n"
    ts += "}\n\n"

    ts += "export const SKILLS: SkillMetadata[] = [\n"

    for skill in skills:
        if skill.startswith('write-script-'):
            lang_key = skill.replace('write-script-', '')
            if lang_key in LANGUAGE_METADATA:
                metadata = LANGUAGE_METADATA[lang_key]
                ts += f'  {{ name: "{skill}", description: "{metadata["description"]}", languageKey: "{lang_key}" }},\n'
        elif skill == 'write-flow':
            ts += f'  {{ name: "{skill}", description: "MUST use when creating flows." }},\n'
        elif skill == 'cli-commands':
            ts += f'  {{ name: "{skill}", description: "MUST use when using the CLI." }},\n'
        elif skill == 'raw-app':
            ts += f'  {{ name: "{skill}", description: "MUST use when creating raw apps." }},\n'
        elif skill == 'triggers':
            ts += f'  {{ name: "{skill}", description: "MUST use when configuring triggers." }},\n'
        elif skill == 'schedules':
            ts += f'  {{ name: "{skill}", description: "MUST use when configuring schedules." }},\n'
        elif skill == 'resources':
            ts += f'  {{ name: "{skill}", description: "MUST use when managing resources." }},\n'

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

    ts += "};\n"

    return ts


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

    # Extract schemas from backend OpenAPI for CLI format documentation
    print("Extracting backend OpenAPI schemas...")
    cli_schemas = {}
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
        ]
        for schema_name in schema_names:
            if schema_name in backend_schemas:
                cli_schemas[schema_name] = extract_cli_schema(backend_schemas[schema_name], backend_schemas, openflow_schemas)

        print(f"  Extracted {len(cli_schemas)} schemas for CLI format")
    else:
        print(f"  Warning: Backend OpenAPI file not found at {BACKEND_OPENAPI_PATH}")

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

    # Generate CLI-specific prompts.ts with base prompts and full prompts
    print("Generating CLI prompts...")
    CLI_GUIDANCE_DIR.mkdir(parents=True, exist_ok=True)
    cli_prompts = {
        'SCRIPT_BASE': script_base,
        'FLOW_BASE': flow_base,
        'SCRIPT_PROMPT': script_md,
        'FLOW_PROMPT': flow_md,
        'CLI_COMMANDS': cli_commands,
    }
    cli_prompts_ts = generate_ts_exports(cli_prompts)
    (CLI_GUIDANCE_DIR / "prompts.ts").write_text(cli_prompts_ts)

    # Generate skill files for Claude Code
    skills = generate_skills(
        languages=languages,
        ts_sdk_md=ts_sdk_md,
        py_sdk_md=py_sdk_md,
        flow_base=flow_base,
        cli_commands=cli_commands,
        openflow_content=openflow_content,
        cli_schemas=cli_schemas
    )

    # Generate skills TypeScript export for CLI
    skills_ts = generate_skills_ts_export(skills)
    (CLI_GUIDANCE_DIR / "skills.ts").write_text(skills_ts)

    print(f"\nGenerated files:")
    print(f"  - auto-generated/sdks/typescript.md")
    print(f"  - auto-generated/sdks/python.md")
    print(f"  - auto-generated/cli/cli-commands.md (auto-generated from CLI source)")
    print(f"  - auto-generated/prompts.ts")
    print(f"  - auto-generated/index.ts")
    print(f"  - auto-generated/script.md")
    print(f"  - auto-generated/flow.md")
    print(f"  - auto-generated/skills/ ({len(skills)} skills)")
    print(f"\nGenerated for CLI:")
    print(f"  - cli/src/guidance/prompts.ts")
    print(f"  - cli/src/guidance/skills.ts")
    print("\nDone!")


if __name__ == '__main__':
    main()
