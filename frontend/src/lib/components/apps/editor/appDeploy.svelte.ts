import { base } from '$lib/base'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

export function computeSecretUrl(secretUrl: string) {
	return `${window.location.origin}${base}/public/${get(workspaceStore)}/${secretUrl}`
}

/**
 * The version a just-finished deploy wrote: this caller's newest entry, and only while it
 * sits directly on `headBefore`, the head read just before the write. That is what the
 * write appended, so anything else in between belongs to a deploy this cannot tell from
 * its own, and an unknown base beats one naming content the draft never forked from.
 */
export function versionThisDeployWrote(
	history: { version: number; created_by?: string }[] | undefined,
	deployedBy: string | undefined,
	headBefore: number | undefined
): number | undefined {
	if (!deployedBy || !history) return undefined
	const i = history.findIndex((h) => h.created_by === deployedBy)
	if (i < 0) return undefined
	return history[i + 1]?.version === headBefore ? history[i].version : undefined
}
