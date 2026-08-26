import { get, writable } from 'svelte/store'
import { LabelService, type Label, type LabelColor } from '$lib/gen'

type LabelColorMap = Record<string, LabelColor>

interface WorkspaceLabels {
	workspace: string
	labels: Label[]
	colors: LabelColorMap
}

/**
 * Colors are workspace-scoped, but a label chip is handed a name and nothing
 * else — a run row, an inherited-labels list, a picker in a session targeting
 * another workspace. Keeping the workspace in the cached value lets every read
 * check it, so a chip rendered for a workspace we have not loaded falls back to
 * the default color instead of borrowing the navigation workspace's.
 */
const cache = writable<Record<string, WorkspaceLabels>>({})

export const labelCache = { subscribe: cache.subscribe }

/**
 * Counts the loads started per workspace so a response that lost the race is
 * dropped rather than overwriting a newer one. Without it, a slow request for
 * workspace A finishing after a fast one for A leaves the older list in place.
 */
const generation: Record<string, number> = {}

function toColorMap(labels: Label[]): LabelColorMap {
	return Object.fromEntries(
		labels.filter((l) => l.color != undefined).map((l) => [l.name, l.color as LabelColor])
	)
}

export async function loadLabels(workspace: string, force = false): Promise<Label[]> {
	const cached = get(cache)[workspace]
	if (!force && cached != undefined) {
		return cached.labels
	}
	const started = (generation[workspace] = (generation[workspace] ?? 0) + 1)
	const labels = await LabelService.listLabels({ workspace })
	if (started !== generation[workspace]) {
		// A newer load for this workspace already landed; keep what it wrote.
		return get(cache)[workspace]?.labels ?? labels
	}
	cache.update((c) => ({ ...c, [workspace]: { workspace, labels, colors: toColorMap(labels) } }))
	return labels
}

/** Drops a workspace's entry so the next read refetches. */
export function invalidateLabels(workspace: string): void {
	generation[workspace] = (generation[workspace] ?? 0) + 1
	cache.update((c) => {
		const { [workspace]: _dropped, ...rest } = c
		return rest
	})
}

/**
 * `undefined` both when the label has no color and when that workspace is not
 * loaded — the caller renders the default in either case.
 */
export function labelColorOf(
	entries: Record<string, WorkspaceLabels>,
	workspace: string | undefined,
	name: string
): LabelColor | undefined {
	if (workspace == undefined) {
		return undefined
	}
	return entries[workspace]?.colors[name]
}

/** Returns the workspace's labels as they stand after the write. */
export async function setLabelColor(
	workspace: string,
	name: string,
	color: LabelColor | undefined
): Promise<Label[]> {
	await LabelService.updateLabel({ workspace, requestBody: { name, color: color ?? null } })
	return await loadLabels(workspace, true)
}
