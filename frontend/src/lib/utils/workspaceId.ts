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

/** `validate_workspace_name` (windmill-common/src/workspaces.rs:246) refuses a longer name. */
export const WORKSPACE_NAME_MAX_LENGTH = 50

/** `check_w_id_conflict` (windmill-api-workspaces/src/workspaces.rs:5111) rejects this id. */
const RESERVED_WORKSPACE_ID = 'global'

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
	// `check_w_id_conflict` refuses it outright, and `existsWorkspace` reports it free —
	// so without this the wizard walks the user to the last step before the create fails.
	// Only the effective id, which is what the backend receives: it defaults to the raw one,
	// so a plain `global` is still caught, while a fork named `global` — submitted as
	// `wm-fork-global`, which the backend accepts — is not.
	if (effectiveId === RESERVED_WORKSPACE_ID) {
		return `'${RESERVED_WORKSPACE_ID}' is not allowed as a workspace ID`
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
