import type { Tool } from './shared'

/** One tool as the settings modal lists it — the model-facing name, description and
 * argument schema, which is exactly what the tool definition sends. */
export type ToolSummary = {
	name: string
	description: string
	/** The JSON Schema of the tool's arguments. `required` is defaulted because
	 * `SchemaViewer` reads it to mark the rows and renders nothing without it, and a
	 * tool whose arguments are all optional legitimately omits it. */
	parameters: Record<string, any>
}

/** Tool definitions as the modal lists them: name-sorted, so a list of dozens is
 * scannable and stays put as the set changes between turns. */
export function summarizeTools(tools: readonly Tool<any>[]): ToolSummary[] {
	return tools
		.map((t) => ({
			name: t.def.function.name,
			description: t.def.function.description ?? '',
			parameters: { required: [], ...(t.def.function.parameters ?? {}) }
		}))
		.sort((a, b) => a.name.localeCompare(b.name))
}
