import { getContext, setContext } from 'svelte'

const CONTEXT_KEY = 'compactBadges'

/** Render every `Badge` below the calling component in its compact density. */
export function setCompactBadges() {
	setContext(CONTEXT_KEY, true)
}

export function isCompactBadges(): boolean {
	return getContext<boolean | undefined>(CONTEXT_KEY) ?? false
}
