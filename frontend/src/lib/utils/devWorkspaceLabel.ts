// Environment label of a dev workspace: its badge text, the wording used to name it, and the git
// branch it deploys to. 'dev' and 'staging' are what the UI offers; any other valid label is a
// custom environment name, which is what lets a chain hold more than two dev workspaces (each must
// carry a distinct label). A null/empty stored value reads as 'dev'.

export type DevWorkspaceLabelKey = string

/** The labels offered by default, in offer order. Any valid label is accepted besides these. */
export const DEV_WORKSPACE_LABELS: DevWorkspaceLabelKey[] = ['dev', 'staging']

/** Resolve the stored `dev_workspace_label` to its effective value; unset/empty is 'dev'. */
export function devLabelKey(label: string | null | undefined): DevWorkspaceLabelKey {
	return label && label !== '' ? label : 'dev'
}

/**
 * Short badge text: 'staging' abbreviates to 'stg', anything else shows verbatim — so this is only
 * bounded by the label's own 30-char limit, not by the 3 characters `dev`/`stg` used to take. Where
 * the badge shares a row with the workspace name, cap it on a child of `<Badge>`: the badge's own
 * element is a flex container, where `text-overflow` never applies and the text would clip
 * mid-glyph instead of ellipsizing.
 */
export function devBadgeText(label: string | null | undefined): string {
	const key = devLabelKey(label)
	return key === 'staging' ? 'stg' : key
}

/** Capitalized word for identity wording, e.g. `${devLabelWord(l)} workspace of X`. */
export function devLabelWord(label: string | null | undefined): string {
	const key = devLabelKey(label)
	return key.charAt(0).toUpperCase() + key.slice(1)
}

/** Lowercase noun phrase for prose, e.g. "made in its ${devLabelNoun(l)}". */
export function devLabelNoun(label: string | null | undefined): string {
	return `${devLabelKey(label)} workspace`
}

/**
 * Roots of the `wm-fork/**` and `wm_deploy/**` branch namespaces. Git cannot hold a branch and a
 * directory of branches under one name, so a label equal to either would break every deploy.
 */
const RESERVED_BRANCH_NAMESPACES = ['wm-fork', 'wm_deploy']

/**
 * Mirrors `normalize_dev_workspace_label` in the backend: the label is used verbatim as a git
 * branch name, so it must be a valid single-segment ref. Returns an error message, or undefined
 * when the label is accepted.
 */
export function devLabelError(label: string): string | undefined {
	const trimmed = label.trim()
	if (trimmed === '') return 'Label cannot be empty'
	if (RESERVED_BRANCH_NAMESPACES.includes(trimmed))
		return `'${trimmed}' is reserved: '${trimmed}/...' branches hold this repository's fork and promotion deploys`
	if (trimmed.length > 30) return 'Label cannot exceed 30 characters'
	if (!/^[a-z0-9][a-z0-9._-]*$/.test(trimmed))
		return "Use lowercase letters, digits, '-', '_' or '.', starting with a letter or digit"
	if (trimmed.includes('..') || trimmed.endsWith('.') || trimmed.endsWith('.lock'))
		return "Not a valid branch name: cannot contain '..' or end with '.' or '.lock'"
	return undefined
}
