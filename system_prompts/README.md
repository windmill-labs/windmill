# System Prompts

This directory contains the single source of truth for AI system prompts used by both the frontend copilot and CLI guidance.

## Structure

```
system_prompts/
├── base/              # Core instruction templates (manually written)
├── languages/         # Language-specific instructions (manually written)
└── auto-generated/    # Auto-generated files (DO NOT EDIT)
    ├── sdks/          # SDK documentation
    ├── cli/           # CLI command documentation
    ├── prompts.ts     # TypeScript exports
    └── index.ts       # Helper functions
```

## Usage

### Regenerating Prompts

When SDK methods or the OpenFlow schema change, run:

```bash
python system_prompts/generate.py
```

This will:

1. Parse TypeScript and Python SDK files to extract function signatures
2. Parse the OpenFlow YAML schema
3. Parse the CLI commands
4. Assemble complete prompts from markdown files
5. Generate TypeScript exports in `auto-generated/`

### Scope

These system prompts contain ONLY:

- How to write Windmill scripts (language syntax, conventions, SDK usage)
- How to structure Windmill flows (OpenFlow schema, module types, data flow)
- Resource type handling, S3 operations

They DO NOT contain:

- Tool usage instructions (edit_code, set_flow_json, etc.)
- IDE/editor specific commands
- Testing tool invocations

Tool instructions are added separately by the frontend and CLI.

## Integration

### Frontend

Uses Vite path alias `$system_prompts` pointing to `auto-generated/`:

```typescript
import { FLOW_GUIDANCE } from "$system_prompts/flow";
import { getLangContext } from "$system_prompts/languages";
```

### CLI

Copies the relevant prompts in /cli/src/guidance/prompts.ts

```

## Editing Guidelines

- Edit markdown files in `base/`, `languages/`
- Never edit files in `auto-generated/` directly
- After editing, run `generate.py` to update exports
