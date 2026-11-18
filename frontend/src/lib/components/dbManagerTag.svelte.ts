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

/** Tags set this session, shared by every caller: local storage is not reactive, so a
 * per-instance cache would leave a second drawer open on the same database deriving —
 * and running its jobs on — the tag the first one just replaced. `null` is an explicit
 * clear, absent means "not set here, read local storage". */
const overrides = $state<Record<string, string | null>>({})

export interface DbManagerTagState {
	/** Tag override for this database's jobs; undefined runs them on the language default. */
	tag: string | undefined
}

export function useDbManagerTag(
	getWorkspace: () => string | undefined,
	getInput: () => DbInput | undefined
): DbManagerTagState {
	let key = $derived(tagKey(getWorkspace(), getInput()))
	let tag = $derived.by(() => {
		if (!key) return undefined
		const set = overrides[key]
		return (set === undefined ? getLocalSetting(key) : set) ?? undefined
	})
	return {
		get tag() {
			return tag
		},
		set tag(v: string | undefined) {
			const k = key
			if (!k) return
			const value = v ? v : undefined
			overrides[k] = value ?? null
			storeLocalSetting(k, value)
		}
	}
}
