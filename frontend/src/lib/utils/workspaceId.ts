/**
 * What the backend accepts as a workspace id, in one place. Every screen that lets
 * someone name a workspace validates against this — a laxer copy elsewhere only
 * moves the rejection from the form to the create call, after the user has
 * finished the whole flow.
 */

/** Letters, digits and underscores in dash-separated groups: no leading, trailing or doubled dash. */
export const WORKSPACE_ID_RE = /^\w+(-\w+)*$/
/** The DB column and the git branch name derived from it both stop here. */
export const WORKSPACE_ID_MAX_LENGTH = 50

/**
 * The reason `id` is not a usable workspace id, or undefined when it is.
 *
 * `effectiveId` is what actually reaches the backend: a fork's id is submitted
 * with a `wm-fork-` prefix, so the length limit applies to the prefixed form while
 * the character rule still applies to what the user typed.
 */
export function validateWorkspaceId(id: string, effectiveId: string = id): string | undefined {
	if (!WORKSPACE_ID_RE.test(id)) {
		return 'ID can only contain letters, numbers and dashes and must not finish by a dash'
	}
	if (effectiveId.length > WORKSPACE_ID_MAX_LENGTH) {
		return `ID '${effectiveId}' is too long (${effectiveId.length} chars). Maximum is ${WORKSPACE_ID_MAX_LENGTH}.`
	}
	return undefined
}

/** Slugifies free text into something `validateWorkspaceId` accepts, for a prefill. */
export function toWorkspaceId(raw: string): string {
	return raw
		.toLowerCase()
		.replace(/[^a-z0-9-]+/g, '-')
		.replace(/-{2,}/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, WORKSPACE_ID_MAX_LENGTH)
		.replace(/-+$/, '')
}
