export type ContextSectionId = 'tools' | 'skills' | 'instructions' | 'mcp'

export type ContextTool = { name: string; description: string; readOnly: boolean }
export type ContextSkill = { path: string; name: string; description: string; enabled: boolean }
export type ContextMcpServer = {
	path: string
	name: string
	description?: string
	enabled: boolean
}
/** What a session's agent is given before the first message. `tools` is every tool
 * definition the chat loop sends, the MCP bridge tools included — a server's own tools
 * are discovered at call time and are not part of the prompt. */
export type AgentContext = {
	workspace: string
	tools: ContextTool[]
	skills: ContextSkill[]
	mcpServers: ContextMcpServer[]
	instructions: { workspace?: string; user?: string }
}

export type ContextSummaryActions = {
	onToggleSkill: (path: string, enabled: boolean) => void
	onToggleMcp: (path: string, enabled: boolean) => void
	/** Opens the assistant settings modal on that section. */
	onManage: (section: ContextSectionId) => void
}

export type ContextSectionSummary = {
	id: ContextSectionId
	label: string
	/** Undefined for Instructions, which has no count. */
	count: string | undefined
	/** Names of what reaches the agent, in display order. */
	names: string[]
	empty: boolean
	/** Shown when the section is empty, or when everything in it is turned off. */
	emptyText: string
	/** Label of the action that opens the settings modal on an empty section. */
	emptyAction?: string
}

export const CONTEXT_SECTION_ORDER: ContextSectionId[] = [
	'tools',
	'skills',
	'instructions',
	'mcp'
]

function partCount(on: number, total: number): string {
	return on === total ? `${total}` : `${on} of ${total}`
}

export function summarizeContextSection(
	ctx: AgentContext,
	id: ContextSectionId
): ContextSectionSummary {
	switch (id) {
		case 'tools': {
			const read = ctx.tools.filter((t) => t.readOnly).length
			return {
				id,
				label: 'Tools',
				count: `${ctx.tools.length}`,
				// The split rather than the names: which tools can change something is the one
				// thing worth reading off a collapsed row.
				names: ctx.tools.length ? [`${read} read`, `${ctx.tools.length - read} write`] : [],
				empty: ctx.tools.length === 0,
				emptyText: 'No tools'
			}
		}
		case 'skills': {
			const on = ctx.skills.filter((s) => s.enabled)
			return {
				id,
				label: 'Skills',
				count: ctx.skills.length ? partCount(on.length, ctx.skills.length) : '0',
				names: on.map((s) => s.name),
				empty: ctx.skills.length === 0,
				emptyText: ctx.skills.length ? 'All turned off' : 'No skills',
				emptyAction: 'Add skill'
			}
		}
		case 'instructions': {
			const names = [
				ctx.instructions.workspace?.trim() ? 'workspace' : undefined,
				ctx.instructions.user?.trim() ? 'personal' : undefined
			].filter((n) => n !== undefined)
			return {
				id,
				label: 'Instructions',
				count: undefined,
				names,
				empty: names.length === 0,
				emptyText: 'No instructions',
				emptyAction: 'Write instructions'
			}
		}
		case 'mcp': {
			const on = ctx.mcpServers.filter((s) => s.enabled)
			return {
				id,
				label: 'MCP',
				count: ctx.mcpServers.length ? partCount(on.length, ctx.mcpServers.length) : '0',
				names: on.map((s) => s.name),
				empty: ctx.mcpServers.length === 0,
				emptyText: ctx.mcpServers.length ? 'All turned off' : 'No servers',
				emptyAction: 'Connect'
			}
		}
	}
}

export function summarizeContext(ctx: AgentContext): ContextSectionSummary[] {
	return CONTEXT_SECTION_ORDER.map((id) => summarizeContextSection(ctx, id))
}
