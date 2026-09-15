import { base } from '$lib/base'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

export function computeSecretUrl(secretUrl: string) {
	return `${window.location.origin}${base}/public/${get(workspaceStore)}/${secretUrl}`
}

/**
 * The version a just-finished deploy wrote: this caller's newest history entry, which the
 * awaited deploy guarantees is the one it wrote. Taking the head instead would pin a
 * deploy that landed in between as the base of content it never contained, and the next
 * deploy would then find base === head and overwrite it with no warning.
 */
export function versionThisDeployWrote(
	history: { version: number; created_by?: string }[] | undefined,
	deployedBy: string | undefined
): number | undefined {
	if (!deployedBy) return undefined
	return history?.find((h) => h.created_by === deployedBy)?.version
}
