export type AccessEntryKind =
	| 'instruction'
	| 'skill'
	| 'mcp'
	| 'script'
	| 'flow'
	| 'app'
	| 'resource'
	| 'variable'

export type AccessEntry = {
	id: string
	kind: AccessEntryKind
	name: string
	path: string
	description: string
}

export type AccessScope = {
	id: 'personal' | 'workspace' | `folder:${string}`
	kind: 'personal' | 'workspace' | 'folder'
	name: string
	path?: string
	description: string
	entries: AccessEntry[]
}

export const ACCESS_ENTRY_KIND_ORDER: AccessEntryKind[] = [
	'mcp',
	'instruction',
	'skill',
	'script',
	'flow',
	'app',
	'resource',
	'variable'
]

export const ACCESS_ENTRY_LABELS: Record<AccessEntryKind, string> = {
	instruction: 'Instruction',
	skill: 'Skill',
	mcp: 'MCP server',
	script: 'Script',
	flow: 'Flow',
	app: 'App',
	resource: 'Resource',
	variable: 'Variable'
}

export const WORKSPACE_ITEM_KINDS = new Set<AccessEntryKind>([
	'script',
	'flow',
	'app',
	'resource',
	'variable'
])

export function scopeRecap(scope: AccessScope): string {
	const counts = ACCESS_ENTRY_KIND_ORDER.map((kind) => ({
		kind,
		count: scope.entries.filter((entry) => entry.kind === kind).length
	})).filter(({ count }) => count > 0)
	const itemCounts = counts.filter(({ kind }) => WORKSPACE_ITEM_KINDS.has(kind))
	const collapseItems = itemCounts.length > 2
	const parts = counts
		.filter(({ kind }) => !collapseItems || !WORKSPACE_ITEM_KINDS.has(kind))
		.map(({ kind, count }) => {
			const label = kind === 'mcp' ? 'MCP' : ACCESS_ENTRY_LABELS[kind].toLowerCase()
			return `${count} ${label}${count === 1 ? '' : 's'}`
		})
	if (collapseItems) {
		parts.push(`${itemCounts.reduce((sum, { count }) => sum + count, 0)} items`)
	}
	return parts.join(' · ')
}
