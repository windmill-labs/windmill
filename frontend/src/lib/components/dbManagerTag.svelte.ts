import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import type { DbInput } from './dbTypes'

/**
 * Every database manager operation runs as a job on the language's native tag
 * (`postgresql`, `duckdb`, …). A database reachable only from one worker group
 * (private network, VPC) needs those jobs on that group's tag instead, and it
 * needs it on every visit — hence local storage rather than component state.
 */
function tagKey(workspace: string | undefined, input: DbInput | undefined): string | undefined {
	if (!workspace || !input) return undefined
	const path = input.type === 'ducklake' ? `ducklake://${input.ducklake}` : input.resourcePath
	return `dbManagerWorkerTag:${workspace}:${path}`
}

export interface DbManagerTagState {
	/** Tag override for this database's jobs; undefined runs them on the language default. */
	tag: string | undefined
}

export function useDbManagerTag(
	getWorkspace: () => string | undefined,
	getInput: () => DbInput | undefined
): DbManagerTagState {
	let key = $derived(tagKey(getWorkspace(), getInput()))
	// Local storage is not reactive, so a tag set here is remembered separately to
	// re-run the readers that depend on it.
	let written = $state<Record<string, string | undefined>>({})
	let tag = $derived.by(() => {
		if (!key) return undefined
		return (key in written ? written[key] : getLocalSetting(key)) ?? undefined
	})
	return {
		get tag() {
			return tag
		},
		set tag(v: string | undefined) {
			const k = key
			if (!k) return
			const value = v ? v : undefined
			written = { ...written, [k]: value }
			storeLocalSetting(k, value)
		}
	}
}
