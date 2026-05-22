# System Prompts

This directory contains the single source of truth for AI system prompts used by both the frontend copilot and CLI guidance.

## Structure

```
system_prompts/
├── base/              # Core instruction templates (manually written)
│   ├── flow-base.md   # Shared OpenFlow structure guidance
│   └── flow-cli.md    # CLI/local-agent workflow guidance for write-flow skill
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

To also refresh the standalone skills in a Claude plugin checkout:

```bash
python system_prompts/generate.py --plugin-dir ~/windmill-claude-plugin
```

`--plugin-dir` accepts:
- the `windmill-claude-plugin` repo root
- a plugin root such as `plugins/windmill`
- a direct `skills/` directory

To regenerate the public docs repo (consumed by context7):

```bash
python system_prompts/generate.py --context7-dir ~/windmill-cli-docs
```

`--context7-dir` writes a fully-rendered snapshot (`AGENTS.md`,
`cli-commands.md`, `skills/<name>/SKILL.md`, `README.md`, `manifest.json`
with the Windmill `version`) with all template placeholders resolved —
suitable for ingestion by docs aggregators. In CI this runs from
`.github/workflows/publish-cli-docs.yml` on every release tag. The
generator refuses to wipe the target directory unless it's empty or has
a context7 marker (`context7.json`, `manifest.json`, or a
`windmill-cli-docs` git remote), so a typo can't delete unrelated files.

This will:

1. Parse TypeScript and Python SDK files to extract function signatures
2. Parse the OpenFlow YAML schema
3. Parse the CLI commands
4. Assemble complete prompts from markdown files
5. Generate TypeScript exports in `auto-generated/`
6. Optionally refresh plugin-ready standalone `SKILL.md` files in the target directory

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

CLI-only workflow instructions live in `base/flow-cli.md` and are included in the
generated `write-flow` skill for `wmill init`. They are intentionally excluded
from the frontend flow chat prompt.

## Integration

### Frontend

Uses Vite path alias `$system_prompts` pointing to `auto-generated/`:

```typescript
import { FLOW_GUIDANCE } from "$system_prompts/flow";
import { getLangContext } from "$system_prompts/languages";
```

### CLI

Generates `/cli/src/guidance/skills.gen.ts` with embedded skill content for `wmill init`.

## Editing Guidelines

- Edit markdown files in `base/`, `languages/`
- Never edit files in `auto-generated/` directly
- After editing, run `generate.py` to update exports
