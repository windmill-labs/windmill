# Claude Code Skill Tests

Test suite for evaluating Claude Code's skill invocation and CLAUDE.md guidance using the Claude Agent SDK.

## Prerequisites

- [Bun](https://bun.sh/) installed
- `ANTHROPIC_API_KEY` environment variable set

## Setup

```bash
cd cli/test-skills
bun install
```

## Running Tests

Run all tests:
```bash
bun test
```

Run only skill invocation tests:
```bash
bun test:skills
```

Run only CLAUDE.md guidance tests:
```bash
bun test:claude-md
```

## Test Categories

### Skill Invocation Tests (`src/skill-invocation.test.ts`)

Tests that verify Claude Code correctly invokes skills when prompted:
- `/commit` command triggers the commit skill
- `/pr` command triggers the pr skill
- Natural language requests like "create a commit" trigger appropriate skills

### CLAUDE.md Guidance Tests (`src/claude-md.test.ts`)

Tests that verify Claude Code follows CLAUDE.md instructions:
- Rust backend questions invoke the rust-backend skill
- Code reviews invoke the code-review skill
- File exploration uses appropriate tools before suggesting changes
- Database questions reference the schema

## Test Utilities

The `src/test-utils.ts` module provides:

- `runPromptAndCapture(prompt, cwd, maxTurns)` - Runs a prompt and captures tool invocations
- `wasToolUsed(result, toolName)` - Checks if a specific tool was used
- `wasSkillInvoked(result, skillName)` - Checks if a specific skill was invoked
- `getToolInputs(result, toolName)` - Gets all inputs for a specific tool

## Notes

- Tests have extended timeouts (60-180 seconds) due to API latency
- Tests run against the actual Claude API, so they consume API credits
- The tests verify behavior at the windmill repo root level
