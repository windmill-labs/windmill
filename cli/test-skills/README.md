# Windmill Skill Invocation Tests

Test suite for verifying that Claude Code correctly invokes Windmill auto-generated skills based on user prompts.

## Overview

This framework tests skill invocation behavior by sending prompts through the Claude Agent SDK and verifying that the expected skills are invoked. Users must provide their own `.claude/skills` folder containing auto-generated Windmill skills.

## Prerequisites

- [Bun](https://bun.sh/) installed
- `ANTHROPIC_API_KEY` environment variable set
- Auto-generated Windmill skills placed in `.claude/skills/`

## User Setup

1. Create a `test-folder` directory inside `cli/test-skills/` and copy your auto-generated Windmill skills into it:

```
cli/test-skills/
└── test-folder/
    └── .claude/
        └── skills/
            ├── write-flow/
            │   └── SKILL.md
            ├── write-script-python3/
            │   └── SKILL.md
            ├── write-script-bun/
            │   └── SKILL.md
            ├── schedules/
            │   └── SKILL.md
            └── triggers/
                └── SKILL.md
```

2. Set your API key:
```bash
export ANTHROPIC_API_KEY=your-key-here
```

3. Install dependencies and run tests:
```bash
cd cli/test-skills
bun install
bun test
```

## Expected Skills

The tests expect the following auto-generated skills to be present:

| Skill Name | Purpose |
|------------|---------|
| `write-flow` | Creating Windmill flows/workflows |
| `write-script-python3` | Creating Python scripts |
| `write-script-bun` | Creating TypeScript/Bun scripts |
| `schedules` | Configuring schedules and cron jobs |
| `triggers` | Setting up triggers (webhook, Kafka, etc.) |

## Test Matrix

| Prompt | Expected Skill |
|--------|----------------|
| "Create a flow to process user data" | `write-flow` |
| "Build a workflow that fetches and transforms data" | `write-flow` |
| "Write a Python script to fetch API data" | `write-script-python3` |
| "Create a Python function to process CSV files" | `write-script-python3` |
| "Write a TypeScript script using Bun" | `write-script-bun` |
| "Create a Bun script to handle webhooks" | `write-script-bun` |
| "Set up a schedule to run this daily at midnight" | `schedules` |
| "Configure a cron job to run every hour" | `schedules` |
| "Set up a webhook trigger for this flow" | `triggers` |
| "Configure a Kafka trigger" | `triggers` |

## Running Tests

Run all tests:
```bash
bun test
```

Run only skill invocation tests:
```bash
bun test:skills
```

## Test Utilities

The `src/test-utils.ts` module provides:

- `runPromptAndCapture(prompt, cwd?, maxTurns)` - Runs a prompt and captures tool invocations
- `wasToolUsed(result, toolName)` - Checks if a specific tool was used
- `wasSkillInvoked(result, skillName)` - Checks if a specific skill was invoked
- `getToolInputs(result, toolName)` - Gets all inputs for a specific tool
- `getTestSkillsDir()` - Returns the test-skills directory path

## Notes

- Tests have extended timeouts (120 seconds) due to API latency
- Tests run against the actual Claude API, so they consume API credits
- Tests verify skill invocation, not skill execution
- The working directory for tests is `test-folder/` (where `.claude/skills` should be placed)
- Tests will fail with a clear error if `test-folder/` or `test-folder/.claude/skills/` don't exist
