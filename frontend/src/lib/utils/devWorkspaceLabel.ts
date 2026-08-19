// Environment label of a dev workspace: its badge text, the wording used to name it, and the git
// branch it deploys to. A null/empty stored value reads as 'dev'.

/**
 * The labels a dev workspace may carry, ordered dev -> prod. Mirrors `DEV_WORKSPACE_LABELS` in the
 * backend, which is authoritative. Every dev workspace in a chain must carry a distinct one, so the
 * length of this list is also the deepest promotion chain.
 */
export const DEV_WORKSPACE_LABELS = [
	'dev',
	'qa',
	'test',
	'uat',
	'staging',
	'demo',
	'sandbox',
	'preprod'
] as const

/** A label the UI may assign. The read side stays `string`: rows predating a list change persist. */
export type DevWorkspaceLabelKey = (typeof DEV_WORKSPACE_LABELS)[number]

/** Resolve the stored `dev_workspace_label` to its effective value; unset/empty is 'dev'. */
export function devLabelKey(label: string | null | undefined): string {
	return label && label !== '' ? label : 'dev'
}

/** Short badge text: 'staging' abbreviates to 'stg', anything else shows verbatim. */
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
 * How to name a child workspace in prose: its environment noun when it is a dev workspace
 * ("dev workspace", "staging workspace"), "fork" otherwise. A dev workspace is a standing
 * environment its whole team works in, so calling it a fork misreads it as throwaway.
 */
export function childWorkspaceNoun(
	workspace: { is_dev_workspace?: boolean; dev_workspace_label?: string | null } | undefined
): string {
	return workspace?.is_dev_workspace ? devLabelNoun(workspace.dev_workspace_label) : 'fork'
}
