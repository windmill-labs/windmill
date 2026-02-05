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

## Raw App Development

You MUST use the \`raw-app\` skill to create or modify raw apps.
Whenever a new app needs to be created you MUST ask the user to run \`wmill app new\` in its terminal first.

## Triggers

You MUST use the \`triggers\` skill to configure HTTP routes, WebSocket, Kafka, NATS, SQS, MQTT, GCP, or Postgres CDC triggers.

## Schedules

You MUST use the \`schedules\` skill to configure cron schedules.

## Resources

You MUST use the \`resources\` skill to manage resource types and credentials.

## CLI Reference

You MUST use the \`cli-commands\` skill to use the CLI.

## Skills

For specific guidance, ALWAYS use the skills listed below.

${skillsReference}
`;
}
