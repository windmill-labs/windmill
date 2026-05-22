/**
 * Core guidance content for the AGENTS files Windmill writes during init.
 *
 * `wmill` writes two files:
 *
 * - `AGENTS.cli.md` — managed CLI / workspace guidance, refreshed by
 *   `wmill refresh prompts` (and the implicit refresh inside `wmill init`).
 * - `AGENTS.md` — user-owned project entry point. The default skeleton
 *   references `AGENTS.cli.md` via an `@`-include so the managed content is
 *   pulled in automatically.
 */

export const AGENTS_CLI_INCLUDE_LINE = "@AGENTS.cli.md";

/**
 * Lightweight, user-owned AGENTS.md skeleton. Written only when no AGENTS.md
 * exists in the project. Everything below the `@AGENTS.cli.md` include is for
 * the user to edit; nothing in this file is refreshed by `wmill`.
 */
export function generateAgentsMdSkeleton(): string {
  return `# Project AI Agent Instructions

This file is the entry point for AI agents working in this repository. It is
**user-owned** — \`wmill\` never overwrites it. Add your project-specific
guidance below the include line.

The line below pulls in Windmill's managed CLI guidance (skills, deploy flow,
debugging jobs, etc.). Refresh it with \`wmill refresh prompts\`. Remove the
include line if you don't want the managed guidance in this project.

${AGENTS_CLI_INCLUDE_LINE}

## Project-specific instructions

<!-- Add anything specific to this repo here. Examples:
     - Deploy commands or environments unique to this project.
     - Domain glossary, naming conventions, or "ask before X" rules.
     - Overrides for the managed guidance above (be explicit that they
       supersede the managed rule). -->
`;
}

/**
 * Managed AGENTS.cli.md content. Rewritten by `wmill init` and
 * `wmill refresh prompts` every time.
 */
export function generateAgentsCliMdContent(skillsReference: string): string {
  return `# Windmill CLI Agent Instructions

> Managed by \`wmill\`. This file is regenerated on \`wmill init\` and
> \`wmill refresh prompts\` — edit AGENTS.md (user-owned) for project-specific
> instructions instead.

You are a helpful assistant that can help with Windmill scripts, flows, apps, and resources management.

## Important Notes
- Every new entity MUST be created using the skills listed below.
- Every modification of an entity MUST be done using the skills listed below.
- User MUST be asked where to create the entity. It can be in its user folder, under u/{user_name} folder, or in a new folder, /f/{folder_name}/. folder_name and user_name must be provided by the user.

## Script Writing Guide

You MUST use the \`write-script-<language>\` skill to write or modify scripts in the language specified by the user. Use bun by default.
For Workflow-as-Code scripts, use the \`write-workflow-as-code\` skill.

## Flow Writing Guide

You MUST use the \`write-flow\` skill to create or modify flows.
When a new flow needs to be created, YOU run \`wmill flow new <path>\` yourself (with \`--summary\` and optional \`--description\`) to scaffold the folder and \`flow.yaml\`, then edit \`flow.yaml\` to fill in modules and schema. Do NOT scaffold the folder + yaml by hand and do NOT tell the user to run \`wmill flow new\`. If path or summary are missing from the user's request, ask via \`AskUserQuestion\` (one call, all missing fields) — never invent them. See the \`write-flow\` skill for the procedure.

## Raw App Development

You MUST use the \`raw-app\` skill to create or modify raw apps.
When a new app needs to be created, YOU run \`wmill app new\` yourself with \`--summary\`, \`--path\`, and \`--framework\` flags (and any other relevant flags). Do NOT ask the user to run it. If you don't have the values for those flags, ask the user via \`AskUserQuestion\` (one call, all missing fields) — never invent them. See the \`raw-app\` skill for the full procedure.

## Triggers

You MUST use the \`triggers\` skill to configure HTTP routes, WebSocket, Kafka, NATS, SQS, MQTT, GCP, Azure, Email, or Postgres CDC triggers.

## Schedules

You MUST use the \`schedules\` skill to configure cron schedules.

## Resources

You MUST use the \`resources\` skill to manage resource types and credentials.

## Visual Preview

You MUST use the \`preview\` skill any time the user wants to see/open/visualize/preview a flow, script, or app in the dev page — and after writing one, when offering visual verification. The skill picks between an MCP-embedded proxy (one named \`launch.json\` entry per target) and direct mode (URL handed to the user) based on what tools you have.

## CLI Reference

You MUST use the \`cli-commands\` skill to use the CLI.

## Running and previewing local changes

Local previews exist for every entity type and don't deploy:

- \`wmill script preview <path> -d '<args>'\` — run a local script.
- \`wmill flow preview <flow_path> -d '<args>'\` — run a local flow.yaml.
- \`wmill app dev\` — live-reload dev server for raw apps.

Argument shapes and per-language details live in the \`write-script-<lang>\`, \`write-flow\`, and \`raw-app\` skills.

## Deploying

There are two ways local changes reach the workspace. Pick based on how the repo is wired, not habit.

### Detecting the setup

Before deploying, check whether this repo has a **GitHub Actions (or other CI) workflow that runs \`wmill sync push\` on push**. That workflow is the signal that pushing a branch will deploy:

- Look for \`.github/workflows/*.yml\` (or other CI configs) that invoke \`wmill sync push\`, \`wmill\` deployment commands, or similar.
- Cache the result for the rest of the session — don't re-scan on every deploy.

If such a workflow exists → **use \`git push\`** (Option A). Otherwise → **use \`wmill sync push\`** directly (Option B).

### Option A — \`git push\` (CI is wired to sync)

The CI workflow will pick up the commit and run \`wmill sync push\` on the backend, which is how deployments are intended to happen in this repo. Don't bypass it.

1. \`git add\` + \`git commit\` the local changes.
2. \`git push\` to the branch the CI runs on.
3. The workflow deploys to the workspace.

Only fall back to Option B if the user explicitly asks to bypass CI for this change (e.g. CI is broken, urgent hotfix), or if the workflow doesn't cover the current branch.

### Option B — \`wmill sync push\` (no CI wiring)

No CI workflow runs \`wmill sync push\` automatically, so deploy directly from the CLI:

- \`wmill sync push --dry-run\` to preview.
- \`wmill sync push\` to apply.

### In both cases

Only deploy when the user explicitly asks to deploy, publish, push, or ship — not when they say "run", "try", or "test". For testing local edits use the per-entity \`preview\` commands (\`wmill script preview\`, \`wmill flow preview\`) — they don't deploy.

## Debugging Jobs

When the user reports a script or flow failure, is investigating unexpected output, or asks why something ran the way it did, use the CLI to fetch job details before speculating. See the \`cli-commands\` skill for all flags.

- \`wmill job list --script-path <path>\` — recent runs of a specific script or flow
- \`wmill job list --failed --limit 20\` — recent failures across the workspace
- \`wmill job get <id>\` — status, timing, and (for flows) the step tree with sub-job IDs
- \`wmill job logs <id>\` — stdout/stderr; for flows, aggregates every step's logs
- \`wmill job result <id>\` — JSON result of a completed job
- \`wmill job cancel <id>\` — stop a running or queued job
- \`wmill job rerun <id>\` — re-run a completed job with the same args (single-job equivalent of the frontend "rerun" button)
- \`wmill job restart <id> --step <step-id> [--iteration <n>]\` — restart a completed flow at a top-level step (for nested-container restart, use the UI)

For flow failures, start with \`wmill job get <id>\` to identify the failing step and its sub-job ID, then \`wmill job logs <sub-job-id>\` to drill in.

## Skills

For specific guidance, ALWAYS use the skills listed below. Paths point at \`.agents/skills/\` — Claude Code reads identical copies under \`.claude/skills/\`.

${skillsReference}
`;
}
