/**
 * Core guidance content for the AGENTS files Windmill writes during init.
 *
 * `wmill` writes two files:
 *
 * - `AGENTS.wmill.md` — managed CLI / workspace guidance, refreshed by
 *   `wmill refresh prompts` (and the implicit refresh inside `wmill init`).
 * - `AGENTS.md` — user-owned project entry point. The default skeleton
 *   references `AGENTS.wmill.md` via an `@`-include so the managed content is
 *   pulled in automatically.
 *
 * The managed file used to be named `AGENTS.cli.md`; `wmill init` /
 * `wmill refresh prompts` migrate the old name to `AGENTS.wmill.md` (and
 * rewrite the `@`-include) automatically. The legacy constants below exist
 * solely for that migration.
 */

export const AGENTS_WMILL_FILENAME = "AGENTS.wmill.md";
export const AGENTS_WMILL_INCLUDE_LINE = "@AGENTS.wmill.md";

/** Legacy managed filename / include line, migrated away from on init/refresh. */
export const LEGACY_AGENTS_CLI_FILENAME = "AGENTS.cli.md";
export const LEGACY_AGENTS_CLI_INCLUDE_LINE = "@AGENTS.cli.md";

/**
 * Lightweight, user-owned AGENTS.md skeleton. Written only when no AGENTS.md
 * exists in the project. Everything below the `@AGENTS.wmill.md` include is for
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

${AGENTS_WMILL_INCLUDE_LINE}

## Project-specific instructions

<!-- Add anything specific to this repo here. Examples:
     - Deploy commands or environments unique to this project.
     - Domain glossary, naming conventions, or "ask before X" rules.
     - Overrides for the managed guidance above (be explicit that they
       supersede the managed rule). -->
`;
}

/**
 * Managed AGENTS.wmill.md content. Rewritten by `wmill init` and
 * `wmill refresh prompts` every time.
 *
 * NOTE: `system_prompts/generate.py` extracts this template by anchoring on
 * the function name `generateAgentsCliMdContent` — keep the name in sync if
 * you rename it.
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

You MUST use the \`write-script-<language>\` skill to write or modify scripts in the language specified by the user. Use bun by default. For TypeScript, always prefer bun (\`write-script-bun\`) over deno unless the script specifically requires the Deno runtime.
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

## Keeping metadata in sync

After editing a script, flow inline script, or app runnable, its generated metadata can go stale. \`wmill-lock.yaml\` stores a content hash per item, so a change that **adds or removes an import** or **changes a script's arguments** invalidates that hash and leaves the \`.lock\` (resolved dependencies) and \`.script.yaml\` (the input schema that drives the auto-generated args UI) out of date. \`wmill generate-metadata\` regenerates them and refreshes the hashes. Leaving them stale produces spurious diffs in git-sync and CI.

This only writes local files — it is **not** a deploy — but it re-resolves dependencies, so it can bump unpinned versions (the same as deploying from the UI; expected, not a bug). So by default **offer it and run it once the user agrees**, rather than running it silently after every edit. YOU run the command (never tell the user to run it); the choice is only whether to confirm first.

After running it, diff the regenerated lockfiles (e.g. \`git diff\` the \`.lock\` / \`.script.lock\` files): if any dependency versions changed, tell the user what bumped (e.g. \`requests 2.31.0 → 2.32.0\`) so they can catch an unwanted change before deploying. Do this even under \`Metadata: auto\` — it is information, not a confirmation gate. Pin a version in code to keep it fixed.

With no path argument it regenerates only the items whose metadata is actually stale (content hash drifted), workspace-wide — not everything. The set can be larger than the file you edited for two reasons: imports propagate (editing a script that others import marks every importer stale too, so their locks regenerate against the new code — by design, since a lock must reflect the imported code), and any pre-existing drift is swept in. If it touches items you didn't expect, run \`wmill generate-metadata --dry-run\` first — it lists each stale item with a reason (\`content changed\` or \`depends on <path>\`) and changes nothing, so you can see why each is in scope. To narrow it, pass a folder or file path (\`wmill generate-metadata f/foo\`); add \`--strict-folder-boundaries\` to touch only items literally inside that folder (it warns about stale importers outside the folder that it skipped — they resurface as stale on the next unscoped run).

**Save the preference so you don't ask every session.** If the user wants metadata regenerated automatically after edits (or always confirmed first), record it in the **project-specific instructions** section of \`AGENTS.md\` (user-owned — never overwritten by \`wmill refresh prompts\`), e.g. a line like \`Metadata: auto (run wmill generate-metadata after edits)\` or \`Metadata: ask first\`. Read that line first on later sessions and follow it.

If the on-disk \`.lock\` and \`.script.yaml\` are already correct and only \`wmill-lock.yaml\` needs its hashes refreshed (hash drift, or bootstrapping missing entries), use \`wmill generate-metadata rehash\` — it re-records hashes from disk with no backend round-trip and no dependency changes.

## Deploying

There are two ways local changes reach the workspace. Pick based on how the repo is wired, not habit.

### Detecting the setup

A \`git push\` deploys when either mechanism is in place: **server-handled git sync** (the workspace auto-pulls the remote, so Windmill deploys what you push) or a **CI workflow that runs \`wmill sync push\` on push**. Check for either:

- Run \`wmill gitsync-settings status\`. If it reports \`deploy_on_push\`, pushing the current branch deploys via backend auto-pull — no CI needed.
- Otherwise look for a \`.github/workflows/*.yml\` (or other CI config) that invokes \`wmill sync push\` or similar \`wmill\` deployment commands.

If either is present → **use \`git push\`** (Option A). If \`status\` reports no auto-pull and there is no CI wiring, **ask the user** how this repo deploys (a CI pipeline you didn't recognize → \`git push\`, or manual → \`wmill sync push\`) rather than assuming — then record the answer (below). Only use \`wmill sync push\` directly (Option B) once you've confirmed there is no deploy-on-push path.

**Save the preference so you don't re-detect it every session.** Once you've determined which option this repo uses (or the user tells you), record it in the **project-specific instructions** section of \`AGENTS.md\` (user-owned — never overwritten by \`wmill refresh prompts\`), e.g. a line like \`Deploy mode: git push (backend auto-pull)\`, \`Deploy mode: git push (CI runs wmill sync push)\`, or \`Deploy mode: wmill sync push (no git-push deploy)\`. On later sessions, read that line first and skip the scan. Re-detect only if the wiring visibly changed.

### Option A — \`git push\` (backend auto-pull or CI deploys on push)

Pushing the commit deploys it: the backend auto-pulls the remote, or a CI workflow runs \`wmill sync push\`. This is how deployments are intended to happen in this repo. Don't bypass it.

1. \`git add\` + \`git commit\` the local changes.
2. \`git push\` to the tracked branch (the one the backend pulls or CI runs on).
3. The deploy happens on the backend.

Only fall back to Option B if the user explicitly asks to bypass this for this change (e.g. the pipeline is broken, urgent hotfix), or if the tracked branch doesn't cover the current branch.

### Option B — \`wmill sync push\` (no CI wiring)

No CI workflow runs \`wmill sync push\` automatically, so deploy directly from the CLI:

- \`wmill sync push --dry-run\` to preview.
- \`wmill sync push\` to apply.

### In both cases

Only deploy when the user explicitly asks to deploy, publish, push, or ship — not when they say "run", "try", or "test". For testing local edits use the per-entity \`preview\` commands (\`wmill script preview\`, \`wmill flow preview\`) — they don't deploy.

## Workspace forks

A **fork** is an isolated copy of a workspace for parallel or experimental work — make changes (including to datatables, which are cloned per fork) without touching the parent, then merge back after review. Each fork is paired with a git branch named \`wm-fork/<base>/<id>\`. Forks require a git repo.

Just run \`wmill workspace fork\` — it adapts to where you are:

- **On a base branch** (e.g. \`main\`, or a branch bound to a workspace): it bases the fork on that branch and prints a \`git checkout -b wm-fork/<base>/<id>\` to start a fresh fork branch.
- **On a working branch** (e.g. you've branched and already edited a forked datatable): it offers to base the fork on that branch and rename it onto \`wm-fork/<base>/<id>\` in place, preserving its commits — asking which base branch is the parent if there's more than one.

For non-interactive runs from a working branch, pass \`--from-branch <base>\` to skip the prompts. The CLI refuses to rename a base branch.

While a \`wm-fork/<base>/<id>\` branch is checked out, every wmill command automatically targets the fork workspace (resolved from the branch name), reusing the base branch workspace's remote (from wmill.yaml) and its saved profile's auth — no \`--workspace\` flag or profile switch needed. Pass \`--workspace\` to target a different workspace explicitly.

Merge a fork back into its parent with \`wmill workspace merge\` (or the Merge UI on the fork's home page). Full reference: https://www.windmill.dev/docs/advanced/workspace_forks

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

## Windmill Documentation

For Windmill concepts not covered by the skills (triggers, schedules, workers, flows, error handling, etc.), read the official docs:

- Fetch https://www.windmill.dev/llms.txt — a curated index of every docs page with one-line descriptions — to find the right page for any concept.
- Every docs page is available as raw markdown by appending \`.md\` to its URL, e.g. https://www.windmill.dev/docs/core_concepts/scheduling.md — prefer these over the HTML pages.
- https://www.windmill.dev/llms-full.txt is the entire documentation as a single ~2.3 MB file — only for bulk indexing/RAG, do NOT load it directly into context.
`;
}
