/**
 * The import wizard's state, and the only thing the first two steps produce.
 *
 * Nothing is created, switched or written while the wizard is being filled in:
 * steps 1 and 2 only ever describe *what should happen*, and the plan travels in
 * the URL. That is what makes the back button and the stepper safe — going back
 * is a URL change and cannot leave a half-made workspace behind, because there is
 * no state anywhere else to unwind.
 *
 * `execution.svelte.ts` is the only thing that acts on a plan, and only when the
 * user asks it to on the last step.
 */

export type ImportDestination =
	| { kind: 'new'; name: string; id: string; username?: string }
	| { kind: 'existing'; workspaceId: string }

export interface ImportPlan {
	/** The hub project being imported. */
	slug: string
	/** Undefined until step 1 and 2 have both been answered. */
	destination?: ImportDestination
	/** Folder the items land in; defaults to the project slug at execution time. */
	folder?: string
}

export type WizardStep = 1 | 2 | 3

export const FOLDER_NAME_RE = /^[a-zA-Z_0-9-]+$/
export const WORKSPACE_ID_RE = /^[a-z0-9-]+$/

/** Windmill workspace ids are lowercase letters, digits and dashes. */
export function toWorkspaceId(raw: string): string {
	return raw
		.toLowerCase()
		.replace(/[^a-z0-9-]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 50)
}

export function readPlan(url: URL): { plan: ImportPlan; step: WizardStep } {
	const params = url.searchParams
	const slug = params.get('hub') ?? ''

	const newId = params.get('new_workspace_id')
	const existing = params.get('workspace')
	const destination: ImportDestination | undefined = newId
		? {
				kind: 'new',
				id: newId,
				name: params.get('new_workspace_name') || newId,
				username: params.get('username') || undefined
			}
		: existing
			? { kind: 'existing', workspaceId: existing }
			: undefined

	const raw = Number(params.get('step') ?? 1)
	const step = (Number.isFinite(raw) ? Math.min(3, Math.max(1, raw)) : 1) as WizardStep

	return { plan: { slug, destination, folder: params.get('folder') || undefined }, step }
}

export function planToSearch(plan: ImportPlan, step: WizardStep): string {
	const params = new URLSearchParams({ hub: plan.slug })
	if (step !== 1) params.set('step', String(step))
	if (plan.destination?.kind === 'new') {
		params.set('new_workspace_id', plan.destination.id)
		params.set('new_workspace_name', plan.destination.name)
		if (plan.destination.username) params.set('username', plan.destination.username)
	} else if (plan.destination?.kind === 'existing') {
		params.set('workspace', plan.destination.workspaceId)
	}
	if (plan.folder) params.set('folder', plan.folder)
	return `?${params}`
}

/**
 * Whether the plan can be executed. Returned as a reason rather than a boolean so
 * the button can say why it is disabled instead of just being grey.
 */
export function planProblem(plan: ImportPlan): string | undefined {
	if (!plan.slug) return 'No project to import'
	const d = plan.destination
	if (!d) return 'Pick a destination first'
	if (d.kind === 'new') {
		if (!d.name.trim()) return 'The new workspace needs a name'
		if (!WORKSPACE_ID_RE.test(d.id)) return 'Workspace ID: lowercase letters, digits and dashes'
	}
	if (plan.folder && !FOLDER_NAME_RE.test(plan.folder)) {
		return 'Folder: letters, digits, dashes and underscores'
	}
	return undefined
}

/** The workspace the plan lands in, once it exists. */
export function planWorkspaceId(plan: ImportPlan): string | undefined {
	return plan.destination?.kind === 'new'
		? plan.destination.id
		: plan.destination?.workspaceId
}
