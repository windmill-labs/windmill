/**
 * The publish flow's item shape and the pure predicates over it. Kept apart from
 * the session store so they can be exercised without dragging in the API client
 * and the editor bundle the store pulls behind it.
 */
import type { Kind } from '$lib/utils_deployable'

export type RecStatus = 'none' | 'recorded'

export interface DeployItem {
	key: string
	path: string
	kind: Kind
	summary?: string
	rec: RecStatus
	[k: string]: unknown
}

export const canRecord = (k: Kind) => k === 'script' || k === 'flow'

// `s3_object` is a built-in file-picker format, never a resource type (it is not
// on the Hub's type list), so it is excluded even when there is no catalog to
// validate against.
export const HIDDEN_RESOURCE_TYPES = new Set(['app_theme', 'state', 'cache', 's3_object'])

/**
 * Resource types an item takes as an input, read off its schema's
 * `resource-<type>` arg formats.
 *
 * A `resource-<x>` format is not a type declaration: stale and misspelled ones
 * pass through unchanged. Publishing those to the Hub would push an empty-schema
 * type and a stub resource nothing can ever fill, so an input-derived type only
 * counts once the workspace declares it — the same bar `ArgInput` applies before
 * rendering an arg as a resource picker.
 *
 * `known` is only authoritative once it holds something: undefined (still
 * loading, or the call failed) and empty (a workspace whose type catalog was
 * never synced from the Hub) both mean "no catalog to validate against", and
 * filtering on one would drop every legitimate type instead.
 */
export function inputResourceTypes(schema: unknown, known: Set<string> | undefined): string[] {
	const validate = known !== undefined && known.size > 0
	const out = new Set<string>()
	const props = (schema as any)?.properties
	if (props && typeof props === 'object') {
		for (const key of Object.keys(props)) {
			const fmt = props[key]?.format
			if (typeof fmt !== 'string' || !fmt.startsWith('resource-')) continue
			const type = fmt.slice('resource-'.length)
			if (HIDDEN_RESOURCE_TYPES.has(type)) continue
			if (validate && !known.has(type)) continue
			out.add(type)
		}
	}
	return [...out]
}

// A raw app has no run to capture: its demo is a recorded session of someone
// using it, driven in the record drawer and replayed on the Hub page. Legacy raw
// apps live only in the `raw_app` table, and the record surface loads the app
// (bundle secret, runnables) through AppService, so it can only offer the action
// for apps stored in the `app` table.
export const canRecordSession = (it: DeployItem): boolean =>
	it.kind === 'raw_app' && it.appTable === true

// Hub rehydration carries draft membership, not where an app is stored. Copy the
// app-table origin from the loaded workspace items onto matching draft items so a
// reopened draft still knows which raw apps can be recorded. Returns the original
// array unchanged when nothing needs merging (stable reference).
export function mergeAppTableOrigin(
	draftItems: DeployItem[],
	workspaceItems: DeployItem[]
): DeployItem[] {
	if (draftItems.length === 0 || workspaceItems.length === 0) return draftItems
	const byKey = new Map(workspaceItems.map((w) => [w.key, w]))
	let changed = false
	const merged = draftItems.map((d) => {
		const w = byKey.get(d.key)
		if (!w || w.appTable === d.appTable) return d
		changed = true
		return { ...d, appTable: w.appTable }
	})
	return changed ? merged : draftItems
}
