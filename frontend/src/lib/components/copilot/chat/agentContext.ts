import type { Tool } from './shared'

/** One tool as the settings modal lists it — the model-facing name and description,
 * which is exactly what the tool definition sends. */
export type ToolSummary = { name: string; description: string }

/** Tool definitions as the modal lists them: name-sorted, so a list of dozens is
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
