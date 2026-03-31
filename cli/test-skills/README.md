# Windmill CLI Prompt Tests

Test suite for verifying how Claude Code behaves with Windmill auto-generated
skills. It currently contains both skill-invocation smoke tests and the first
artifact-evaluation benchmark.

## Overview

This framework sends prompts through the Claude Agent SDK using the repo's
generated Windmill skills.

It currently supports:

- skill-invocation smoke tests
- CLI artifact evaluation in an isolated temp workspace

## Prerequisites

- [Bun](https://bun.sh/) installed
- `ANTHROPIC_API_KEY` environment variable set
- Auto-generated Windmill skills available under `system_prompts/auto-generated/skills/`

## User Setup

1. Generate the latest system prompts and CLI skills in the repo:

```bash
python3 system_prompts/generate.py
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

Run the first artifact-evaluation benchmark:
```bash
bun test:artifact
```

## Test Utilities

The `src/test-utils.ts` module provides:

- `runPromptAndCapture(prompt, cwd?, maxTurns)` - Runs a prompt and captures tool invocations
- `wasToolUsed(result, toolName)` - Checks if a specific tool was used
- `wasSkillInvoked(result, skillName)` - Checks if a specific skill was invoked
- `getToolInputs(result, toolName)` - Gets all inputs for a specific tool
- `getTestSkillsDir()` - Returns the test-skills directory path

The `src/artifact-eval.ts` module provides:

- temp-workspace creation with generated skills
- prompt rendering with workspace-root placeholders
- file-based artifact scoring for benchmark cases

## Notes

- Tests have extended timeouts (120 seconds) due to API latency
- Tests run against the actual Claude API, so they consume API credits
- Skill-invocation smoke tests still use `test-folder/`
- Artifact evals use isolated temp workspaces under `/tmp`
- The first artifact benchmark uses an explicit absolute target path so file
  outputs are scoreable even when Claude executes inside a skill context
