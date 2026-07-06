// Cosmetic display label for a dev workspace. The paired-fork machinery is unchanged; this only
// swaps the badge text and identity wording so a team can present the environment as "staging"
// instead of "dev". A null/unknown stored value renders as "dev" (the default).

export type DevWorkspaceLabelKey = 'dev' | 'staging'

/** Resolve the stored `dev_workspace_label` to a known key; anything unset/unknown is 'dev'. */
export function devLabelKey(label: string | null | undefined): DevWorkspaceLabelKey {
	return label === 'staging' ? 'staging' : 'dev'
}

/** Short badge text: 'dev' or 'stg'. */
export function devBadgeText(label: string | null | undefined): string {
	return devLabelKey(label) === 'staging' ? 'stg' : 'dev'
}

/** Capitalized word for identity wording, e.g. `${devLabelWord(l)} workspace of X`. */
export function devLabelWord(label: string | null | undefined): string {
	return devLabelKey(label) === 'staging' ? 'Staging' : 'Dev'
}

/** Lowercase noun phrase for prose, e.g. "made in its ${devLabelNoun(l)}". */
export function devLabelNoun(label: string | null | undefined): string {
	return devLabelKey(label) === 'staging' ? 'staging workspace' : 'dev workspace'
}
