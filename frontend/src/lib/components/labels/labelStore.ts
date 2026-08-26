import { derived, get, writable } from 'svelte/store'
import { LabelService, type Label, type LabelColor } from '$lib/gen'

type LabelColorMap = Record<string, LabelColor>

/**
 * Colors are workspace-scoped, but a label chip is handed a name and nothing
 * else — a run row, an inherited-labels list, a picker in a session targeting
 * another workspace. Keeping the workspace in the cached value lets every read
 * check it, so a chip rendered for a workspace we have not loaded falls back to
 * the default color instead of borrowing the navigation workspace's.
 */
const cache = writable<{ workspace: string; colors: LabelColorMap } | undefined>(undefined)

export const labelColorCache = { subscribe: cache.subscribe }

/** All labels of a workspace, colored or not, as `labels/list` returns them. */
export const labelList = writable<{ workspace: string; labels: Label[] } | undefined>(undefined)

export const labelNames = derived(labelList, (l) => l?.labels.map((label) => label.name) ?? [])

function toColorMap(labels: Label[]): LabelColorMap {
	return Object.fromEntries(
		labels.filter((l) => l.color != undefined).map((l) => [l.name, l.color as LabelColor])
	)
}

export async function loadLabels(workspace: string, force = false): Promise<Label[]> {
	const cached = get(labelList)
	if (!force && cached?.workspace === workspace) {
		return cached.labels
	}
	const labels = await LabelService.listLabels({ workspace })
	labelList.set({ workspace, labels })
	cache.set({ workspace, colors: toColorMap(labels) })
	return labels
}

/**
 * `undefined` both when the label has no color and when the cache holds another
 * workspace — the caller renders the default in either case.
 */
export function labelColorOf(
	entry: { workspace: string; colors: LabelColorMap } | undefined,
	workspace: string | undefined,
	name: string
): LabelColor | undefined {
	if (entry == undefined || workspace == undefined || entry.workspace !== workspace) {
		return undefined
	}
	return entry.colors[name]
}

export async function setLabelColor(
	workspace: string,
	name: string,
	color: LabelColor | undefined
): Promise<void> {
	await LabelService.updateLabel({ workspace, requestBody: { name, color: color ?? null } })
	await loadLabels(workspace, true)
}
