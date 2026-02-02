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
    CLI_GUIDANCE_DIR,
    CLI_MAIN,
    CLI_COMMANDS_DIR,
    # Language metadata
    LANGUAGE_METADATA,
    TS_SDK_LANGUAGES,
    PY_SDK_LANGUAGES,
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
        cmd_match = re.match(r'\.command\(\s*["\']([^"\']+)["\']\s*(?:,\s*([^)]+))?\s*\)', section)
        if not cmd_match:
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
        'name': 'cli-commands',
        'description': 'MUST use when using the CLI.',
        'content_key': 'cli_commands',
    },
]


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
    base_content = {
        'flow': f"{flow_base}\n\n{openflow_content}",
        'raw_app': read_markdown_file(base_dir / "raw-app.md"),
        'triggers': read_markdown_file(base_dir / "triggers.md"),
        'schedules': read_markdown_file(base_dir / "schedules.md"),
        'resources': read_markdown_file(base_dir / "resources.md"),
        'cli_commands': cli_commands,
    }

    # CLI intro for script skills
    script_cli_intro = """## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types."""

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
        sdk_content = ''
        if lang_key in TS_SDK_LANGUAGES:
            sdk_content = ts_sdk_md
        elif lang_key in PY_SDK_LANGUAGES:
            sdk_content = py_sdk_md

        skill_content = generate_skill_content(
            skill_name=skill_name,
            description=metadata['description'],
            intro=script_cli_intro,
            content=lang_content,
            sdk_content=sdk_content
        )

        (skill_dir / "SKILL.md").write_text(skill_content)
        skills_generated.append(skill_name)

    # Generate other skills from definitions
    for skill_def in SKILL_DEFINITIONS:
        content = base_content.get(skill_def['content_key'], '')
        if not content:
            continue

        skill_name = skill_def['name']
        skill_dir = OUTPUT_SKILLS_DIR / skill_name
        skill_dir.mkdir(parents=True, exist_ok=True)

        # Append schemas if defined
        if 'schema_types' in skill_def:
            schema_docs = []
            for schema_name, file_suffix in skill_def['schema_types']:
                if schema_name in cli_schemas:
                    schema_doc = format_schema_for_markdown(
                        cli_schemas[schema_name],
                        f"{schema_name} (`*.{file_suffix}.yaml`)",
                        as_json_schema=True
                    )
                    if schema_doc:
                        schema_docs.append(schema_doc)
            if schema_docs:
                content = content + "\n\n" + "\n\n".join(schema_docs)

        skill_content = generate_skill_content(
            skill_name=skill_name,
            description=skill_def['description'],
            intro="",
            content=content
        )

        (skill_dir / "SKILL.md").write_text(skill_content)
        skills_generated.append(skill_name)

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


# =============================================================================
# Main Entry Point
# =============================================================================


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

    # Generate skill files for Claude Code
    CLI_GUIDANCE_DIR.mkdir(parents=True, exist_ok=True)
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
    print(f"  - cli/src/guidance/skills.ts")
    print("\nDone!")


if __name__ == '__main__':
    main()
