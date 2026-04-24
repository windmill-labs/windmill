/**
 * Core guidance content for AGENTS.md
 *
 * This module exports the template for the AGENTS.md file that provides
 * AI agent instructions for working with Windmill projects.
 */

/**
 * Generate the AGENTS.md content with the given skills reference.
 * @param skillsReference - A formatted list of skills to include in the document
 * @returns The complete AGENTS.md content
 */
export function generateAgentsMdContent(skillsReference: string): string {
  return `# Windmill AI Agent Instructions

You are a helpful assistant that can help with Windmill scripts, flows, apps, and resources management.

## Important Notes
- Every new entity MUST be created using the skills listed below.
- Every modification of an entity MUST be done using the skills listed below.
- User MUST be asked where to create the entity. It can be in its user folder, under u/{user_name} folder, or in a new folder, /f/{folder_name}/. folder_name and user_name must be provided by the user.

## Script Writing Guide

You MUST use the \`write-script-<language>\` skill to write or modify scripts in the language specified by the user. Use bun by default.

## Flow Writing Guide

You MUST use the \`write-flow\` skill to create or modify flows.
When a new flow needs to be created, YOU run \`wmill flow new <path>\` yourself (with \`--summary\` and optional \`--description\`) to scaffold the folder and \`flow.yaml\`, then edit \`flow.yaml\` to fill in modules and schema. Do NOT scaffold the folder + yaml by hand and do NOT tell the user to run \`wmill flow new\`. If path or summary are missing from the user's request, ask via \`AskUserQuestion\` (one call, all missing fields) — never invent them. See the \`write-flow\` skill for the procedure.

## Raw App Development

You MUST use the \`raw-app\` skill to create or modify raw apps.
When a new app needs to be created, YOU run \`wmill app new\` yourself with \`--summary\`, \`--path\`, and \`--framework\` flags (and any other relevant flags). Do NOT ask the user to run it. If you don't have the values for those flags, ask the user via \`AskUserQuestion\` (one call, all missing fields) — never invent them. See the \`raw-app\` skill for the full procedure.

## Triggers

You MUST use the \`triggers\` skill to configure HTTP routes, WebSocket, Kafka, NATS, SQS, MQTT, GCP, or Postgres CDC triggers.

## Schedules

You MUST use the \`schedules\` skill to configure cron schedules.

## Resources

You MUST use the \`resources\` skill to manage resource types and credentials.

## Visual Preview

You MUST use the \`preview\` skill any time the user wants to see/open/visualize/preview a flow, script, or app in the dev page — and after writing one, when offering visual verification. The skill picks between an MCP-embedded proxy (one named \`launch.json\` entry per target) and direct mode (URL handed to the user) based on what tools you have.

## CLI Reference

You MUST use the \`cli-commands\` skill to use the CLI.

## Debugging Jobs

When the user reports a script or flow failure, is investigating unexpected output, or asks why something ran the way it did, use the CLI to fetch job details before speculating. See the \`cli-commands\` skill for all flags.

- \`wmill job list --script-path <path>\` — recent runs of a specific script or flow
- \`wmill job list --failed --limit 20\` — recent failures across the workspace
- \`wmill job get <id>\` — status, timing, and (for flows) the step tree with sub-job IDs
- \`wmill job logs <id>\` — stdout/stderr; for flows, aggregates every step's logs
- \`wmill job result <id>\` — JSON result of a completed job
- \`wmill job cancel <id>\` — stop a running or queued job

For flow failures, start with \`wmill job get <id>\` to identify the failing step and its sub-job ID, then \`wmill job logs <sub-job-id>\` to drill in.

## Skills

For specific guidance, ALWAYS use the skills listed below.

${skillsReference}
`;
}
