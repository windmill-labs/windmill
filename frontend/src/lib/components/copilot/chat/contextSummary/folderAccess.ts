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
	id: string
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

const folderSpecs = [
	{
		name: 'Finance',
		slug: 'finance',
		subject: 'financial records',
		scriptCount: 8,
		flowCount: 5,
		appCount: 2
	},
	{
		name: 'Customer support',
		slug: 'customer_support',
		subject: 'support tickets',
		scriptCount: 7,
		flowCount: 6,
		appCount: 2
	},
	{
		name: 'Data platform',
		slug: 'data_platform',
		subject: 'datasets',
		scriptCount: 9,
		flowCount: 5,
		appCount: 2
	},
	{
		name: 'Engineering',
		slug: 'engineering',
		subject: 'releases',
		scriptCount: 8,
		flowCount: 5,
		appCount: 2
	},
	{
		name: 'Growth',
		slug: 'growth',
		subject: 'campaigns',
		scriptCount: 6,
		flowCount: 4,
		appCount: 2
	},
	{
		name: 'Operations',
		slug: 'operations',
		subject: 'orders',
		scriptCount: 9,
		flowCount: 6,
		appCount: 3
	},
	{
		name: 'People',
		slug: 'people',
		subject: 'employee onboarding',
		scriptCount: 5,
		flowCount: 4,
		appCount: 1
	}
] as const

function entry(
	folder: string,
	kind: AccessEntryKind,
	name: string,
	path: string,
	description: string
): AccessEntry {
	return { id: `${folder}:${kind}:${path}`, kind, name, path, description }
}

function folderScope(spec: (typeof folderSpecs)[number], index: number): AccessScope {
	const { name, slug, subject, scriptCount, flowCount, appCount } = spec
	const entries: AccessEntry[] = [
		entry(
			slug,
			'instruction',
			`${name} working agreement`,
			`f/${slug}/instructions`,
			`How the agent should handle ${subject} work.`
		),
		entry(
			slug,
			'skill',
			`${name} specialist`,
			`f/${slug}/skills/specialist`,
			`Domain guidance and repeatable procedures for ${subject}.`
		)
	]

	const scriptNames = [
		`Import ${subject}`,
		`Normalize ${subject}`,
		`Sync ${subject} to warehouse`,
		`Validate ${subject}`,
		`Export ${subject} report`,
		`Archive stale ${subject}`,
		`Backfill ${subject}`,
		`Notify ${subject} owners`,
		`Reconcile ${subject}`
	]
	const flowNames = [
		`${name} intake`,
		`${name} approval`,
		`${name} daily sync`,
		`${name} escalation`,
		`${name} weekly review`,
		`${name} lifecycle`
	]
	const appNames = [`${name} dashboard`, `${name} operations console`, `${name} admin`]

	entries.push(
		...scriptNames
			.slice(0, scriptCount)
			.map((label, itemIndex) =>
				entry(
					slug,
					'script',
					label,
					`f/${slug}/scripts/${itemIndex + 1}`,
					`Processes ${subject} for the ${name.toLowerCase()} team.`
				)
			),
		...flowNames
			.slice(0, flowCount)
			.map((label, itemIndex) =>
				entry(
					slug,
					'flow',
					label,
					`f/${slug}/flows/${itemIndex + 1}`,
					`Coordinates ${subject} across the team.`
				)
			),
		...appNames
			.slice(0, appCount)
			.map((label, itemIndex) =>
				entry(
					slug,
					'app',
					label,
					`f/${slug}/apps/${itemIndex + 1}`,
					`Operational view for ${subject}.`
				)
			)
	)
	if (index % 3 !== 1) {
		entries.push(
			entry(
				slug,
				'resource',
				`${name} database`,
				`$res:f/${slug}/database`,
				`Connection configuration used by this folder.`
			)
		)
	}
	if (index % 3 === 0) {
		entries.push(
			entry(
				slug,
				'mcp',
				`${name} gateway`,
				`$res:f/${slug}/mcp_gateway`,
				`External tools scoped to ${subject}.`
			)
		)
	}
	if (index % 4 !== 2) {
		entries.push(
			entry(
				slug,
				'variable',
				`${slug.toUpperCase()}_REGION`,
				`$var:f/${slug}/region`,
				`Default region for ${subject} operations.`
			)
		)
	}

	return {
		id: `folder:${slug}`,
		kind: 'folder',
		name,
		path: `f/${slug}`,
		description: `Everything the agent can use for ${subject}.`,
		entries: entries.sort((a, b) => a.name.localeCompare(b.name))
	}
}

const virtualScopes: AccessScope[] = [
	{
		id: 'personal',
		kind: 'personal',
		name: 'Personal',
		description: 'Preferences and guidance that apply only to you.',
		entries: [
			entry(
				'personal',
				'instruction',
				'My working preferences',
				'personal/instructions',
				'Favor concise explanations and show proposed changes before applying them.'
			),
			entry(
				'personal',
				'skill',
				'My review checklist',
				'personal/skills/review_checklist',
				'A personal checklist for reviewing generated work.'
			)
		]
	},
	{
		id: 'workspace',
		kind: 'workspace',
		name: 'Workspace-wide',
		description: 'Shared context available across every workspace folder.',
		entries: [
			entry(
				'workspace',
				'instruction',
				'Workspace conventions',
				'workspace/instructions',
				'Shared naming, testing, and deployment conventions.'
			),
			entry(
				'workspace',
				'skill',
				'Windmill conventions',
				'workspace/skills/windmill',
				'Standard patterns for scripts, flows, apps, and resources.'
			),
			entry(
				'workspace',
				'mcp',
				'Documentation',
				'$res:u/admin/documentation_mcp',
				'Searches internal product and engineering documentation.'
			),
			entry(
				'workspace',
				'resource',
				'Shared object storage',
				'$res:u/admin/object_storage',
				'Workspace-wide object storage configuration.'
			)
		].sort((a, b) => a.name.localeCompare(b.name))
	}
]

export const ACCESS_SCOPES: AccessScope[] = [...virtualScopes, ...folderSpecs.map(folderScope)]

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
		parts.push(`${itemCounts.reduce((total, { count }) => total + count, 0)} items`)
	}
	return parts.join(' · ')
}
