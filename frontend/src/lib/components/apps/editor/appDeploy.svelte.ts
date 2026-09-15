import { base } from '$lib/base'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

export function computeSecretUrl(secretUrl: string) {
	return `${window.location.origin}${base}/public/${get(workspaceStore)}/${secretUrl}`
}

/**
 * The version a just-finished deploy wrote, read back from the history it lands in.
 * The deploy and this read are two requests, so someone else's deploy in between is the
 * newest entry too; taking it would make their version the fork base of content it never
 * contained, and the next deploy would find base === head and overwrite them with no
 * warning. A head this caller did not write therefore yields `undefined`: an unknown
 * base, which the timestamps still cover, rather than a wrong one.
 *
 * Author is all the history can be matched on (`deployment_msg` is written later, by the
 * dependency job), so a second deploy by the same user inside that window still reads as
 * this one's.
 */
export function versionThisDeployWrote(
	history: { version: number; created_by?: string }[] | undefined,
	deployedBy: string | undefined
): number | undefined {
	const head = history?.[0]
	if (!head || !deployedBy || head.created_by !== deployedBy) return undefined
	return head.version
}
