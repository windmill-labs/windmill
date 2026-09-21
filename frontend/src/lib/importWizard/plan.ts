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

import { validateUsername } from '$lib/utils'
import { validateWorkspaceId, WORKSPACE_NAME_MAX_LENGTH } from '$lib/utils/workspaceId'

/**
 * `existing` carries no workspace until one is picked: step 1 answers *which kind*
 * of destination and step 2 answers *which one*, and the plan has to be able to
 * hold the state in between. Encoding that gap as an absent destination would make
 * "chose an existing workspace, has not picked it" and "answered nothing yet" the
 * same URL.
 */
export type ImportDestination =
	| { kind: 'new'; name: string; id: string; username?: string }
	| { kind: 'existing'; workspaceId?: string }

export interface ImportPlan {
	/** The hub project being imported. */
	slug: string
	/** Undefined until step 1 has been answered. */
	destination?: ImportDestination
	/** Folder the items land in; defaults to the project slug at execution time. */
	folder?: string
}

/** 4 is the optional setup step, reached only when the import leaves work to do. */
export type WizardStep = 1 | 2 | 3 | 4

export const FOLDER_NAME_RE = /^[a-zA-Z_0-9-]+$/

export function readPlan(url: URL): { plan: ImportPlan; step: WizardStep } {
	const params = url.searchParams
	const slug = params.get('hub') ?? ''

	const newId = params.get('new_workspace_id')
	const existing = params.get('workspace')
	// `destination` records the step 1 answer on its own; the older links that only
	// carried `workspace` or `new_workspace_id` still read as the kind they imply.
	const kind = params.get('destination')
	const destination: ImportDestination | undefined =
		newId || kind === 'new'
			? {
					kind: 'new',
					id: newId ?? '',
					name: params.get('new_workspace_name') || newId || '',
					username: params.get('username') || undefined
				}
			: existing || kind === 'existing'
				? { kind: 'existing', workspaceId: existing || undefined }
				: undefined

	// Rounded as well as clamped: the steps are compared with `>` and `===`, so a
	// fractional `?step=2.5` would clamp to 2.5 and match neither.
	const raw = Number(params.get('step') ?? 1)
	const step = (Number.isFinite(raw) ? Math.min(4, Math.max(1, Math.round(raw))) : 1) as WizardStep

	return { plan: { slug, destination, folder: params.get('folder') || undefined }, step }
}

export function planToSearch(plan: ImportPlan, step: WizardStep): string {
	const params = new URLSearchParams({ hub: plan.slug })
	if (step !== 1) params.set('step', String(step))
	if (plan.destination) params.set('destination', plan.destination.kind)
	if (plan.destination?.kind === 'new') {
		params.set('new_workspace_id', plan.destination.id)
		params.set('new_workspace_name', plan.destination.name)
		if (plan.destination.username) params.set('username', plan.destination.username)
	} else if (plan.destination?.workspaceId) {
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
		// The backend refuses a longer one (`validate_workspace_name`), and only at
		// creation — by then the wizard has already walked the user through two more steps.
		if (d.name.trim().length > WORKSPACE_NAME_MAX_LENGTH) {
			return `The name is too long (${d.name.trim().length} chars). Maximum is ${WORKSPACE_NAME_MAX_LENGTH}.`
		}
		const idProblem = validateWorkspaceId(d.id)
		if (idProblem) return idProblem
		// Only asked for when the instance does not derive it. `create_workspace` takes
		// whatever it is given here — `Some("")` passes its only check — so a blank or
		// malformed username is written to `usr.username` verbatim rather than refused.
		// The sibling creator validates it; this is the same check.
		if (d.username !== undefined) {
			if (!d.username.trim()) return 'The new workspace needs a username'
			const bad = validateUsername(d.username.trim())
			if (bad) return bad
		}
	} else if (!d.workspaceId) {
		return 'Pick the workspace to import into'
	} else if (validateWorkspaceId(d.workspaceId)) {
		// The id arrives from the URL exactly as the new-workspace one does, so it gets
		// the same check. Downstream it is interpolated into a credentialed same-origin
		// API path and pushed into `workspaceStore`; an id that cannot name a workspace
		// has no business reaching either.
		return 'That is not a valid workspace id'
	}
	if (plan.folder && !FOLDER_NAME_RE.test(plan.folder)) {
		return 'Folder: letters, digits, dashes and underscores'
	}
	return undefined
}

/** The workspace the plan lands in, once one has been named or picked. */
export function planWorkspaceId(plan: ImportPlan): string | undefined {
	const d = plan.destination
	return (d?.kind === 'new' ? d.id : d?.workspaceId) || undefined
}
