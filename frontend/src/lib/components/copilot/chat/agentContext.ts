import type { Tool } from './shared'
import type { AttachedFileStatus } from './files/attachedFiles.svelte'

/** One tool as the context panel lists it — the model-facing name and description,
 * which is exactly what the tool definition sends. */
export type ToolSummary = { name: string; description: string }

/** What the glance line counts. `instructions` is a flag rather than a count:
 * two blocks of custom instructions are not "more context" in a way a number
 * conveys, only that the assistant is steered by some. */
export type AgentContextCounts = {
	tools: number
	skills: number
	mcpServers: number
	attachments: number
	instructions: boolean
}

function plural(n: number, singular: string): string {
	return `${n} ${singular}${n === 1 ? '' : 's'}`
}

/**
 * The one-line summary shown on the panel's trigger — "64 tools · 2 skills ·
 * 1 MCP server". Empty categories are dropped rather than shown as zeroes: the
 * line is a glance, and a row of zeroes reads as a warning about something that
 * is simply not configured.
 */
export function contextGlanceLine(counts: AgentContextCounts): string {
	const parts = [
		counts.tools > 0 ? plural(counts.tools, 'tool') : undefined,
		counts.skills > 0 ? plural(counts.skills, 'skill') : undefined,
		counts.mcpServers > 0 ? plural(counts.mcpServers, 'MCP server') : undefined,
		counts.attachments > 0 ? plural(counts.attachments, 'attachment') : undefined,
		counts.instructions ? 'custom instructions' : undefined
	].filter((p): p is string => p !== undefined)
	return parts.length > 0 ? parts.join(' · ') : 'Nothing yet'
}

/** Why an attachment is not reachable, or undefined when it is. The file tools
 * operate on `readyFiles()`, so every other status is attached-but-unreadable and
 * has to say so — a row that looks like the readable ones is the one place this
 * panel could claim something the assistant cannot actually open. */
export function attachmentStatusLabel(status: AttachedFileStatus): string | undefined {
	switch (status) {
		case 'ready':
			return undefined
		case 'locked':
			return 'needs access'
		case 'unavailable':
			return 'unavailable'
		case 'indexing':
			return 'indexing…'
		case 'error':
			return 'failed'
	}
}

/** The Files & folders row's value: what is usable, and what is merely attached
 * when those differ. Collapsing to one number would count a locked folder under a
 * heading reading "Available to the assistant". */
export function attachmentValue(ready: number, total: number): string {
	if (total === 0) return 'None'
	return ready === total ? `${total}` : `${ready} of ${total}`
}

/** Tool definitions as the panel lists them: name-sorted, so a list of dozens is
 * scannable and stays put as the set changes between turns. */
export function summarizeTools(tools: readonly Tool<any>[]): ToolSummary[] {
	return tools
		.map((t) => ({
			name: t.def.function.name,
			description: t.def.function.description ?? ''
		}))
		.sort((a, b) => a.name.localeCompare(b.name))
}

/** Descriptions are matched as well as names: a user looking for "screenshot" or
 * "postgres" is describing what they want done, not recalling a tool's name. */
export function filterTools(tools: readonly ToolSummary[], query: string): ToolSummary[] {
	const needle = query.trim().toLowerCase()
	if (!needle) return [...tools]
	return tools.filter(
		(t) => t.name.toLowerCase().includes(needle) || t.description.toLowerCase().includes(needle)
	)
}
