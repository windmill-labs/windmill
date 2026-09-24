import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import type { DbTableJoin } from './dbTableJoins'

export const DB_TABLE_LAYOUT_PREFIX = 'dbTableLayout:'
// Every table opened gets a layout, so without a cap they pile up forever.
const MAX_LAYOUTS = 60
const PRUNED_PER_OVERFLOW = 10

export type DbTableLayout = {
	widths?: Record<string, number>
	pinned?: Record<string, 'left' | 'right'>
	joins?: DbTableJoin[]
	lastUsed?: number
}

export function readDbTableLayout(key: string): DbTableLayout {
	try {
		const parsed = JSON.parse(getLocalSetting(key) ?? '{}')
		return parsed && typeof parsed === 'object' ? parsed : {}
	} catch {
		return {}
	}
}

export function saveDbTableLayout(key: string, layout: DbTableLayout) {
	try {
		if (!layout.widths && !layout.pinned && !layout.joins) {
			storeLocalSetting(key, undefined)
			return
		}
		const isNew = getLocalSetting(key) == null
		storeLocalSetting(key, JSON.stringify({ ...layout, lastUsed: Date.now() }))
		if (isNew) pruneDbTableLayouts(key)
	} catch {}
}

/** Drops the least recently used layouts once there are more than the cap, never `keep`. */
function pruneDbTableLayouts(keep: string) {
	const keys = Object.keys(localStorage).filter((k) => k.startsWith(DB_TABLE_LAYOUT_PREFIX))
	if (keys.length <= MAX_LAYOUTS) return
	keys
		.filter((k) => k !== keep)
		.map((k) => ({ k, lastUsed: readDbTableLayout(k).lastUsed ?? 0 }))
		.sort((a, b) => a.lastUsed - b.lastUsed)
		.slice(0, PRUNED_PER_OVERFLOW)
		.forEach(({ k }) => localStorage.removeItem(k))
}
