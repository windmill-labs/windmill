# System Prompts

This directory contains the single source of truth for AI system prompts used by both the frontend copilot and CLI guidance.

## Structure

```
system_prompts/
├── base/              # Core instruction templates (manually written)
├── languages/         # Language-specific instructions (manually written)
├── resources/         # Resource and S3 instructions (manually written)
├── sdks/              # Auto-generated SDK documentation
├── schemas/           # Auto-generated OpenFlow schema
└── generated/         # Auto-generated TypeScript exports
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
3. Assemble complete prompts from markdown files
4. Generate TypeScript exports in `generated/`

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
Uses Vite path alias `$system_prompts` pointing to `generated/`:
```typescript
import { FLOW_GUIDANCE } from '$system_prompts/flow'
import { getLangContext } from '$system_prompts/languages'
```

### CLI
Uses Deno import mapping:
```typescript
import { SCRIPT_GUIDANCE } from '$system_prompts/script.ts'
```

## Editing Guidelines

- Edit markdown files in `base/`, `languages/`, `resources/`
- Never edit files in `generated/` directly
- After editing, run `generate.py` to update exports
- SDK and schema files in `sdks/` and `schemas/` are auto-generated
