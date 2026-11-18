import { AssetService, type ListWorkspaceMacrosResponse } from '$lib/gen'

export type WorkspaceMacro = ListWorkspaceMacrosResponse[number]

// Workspace macros are late-bound (the worker reads the registry per job), so
// mild staleness in editor surfaces is harmless — a short TTL keeps repeated
// editor mounts / drawer opens from refetching on every keystroke-driven
// remount while still picking up a lib deploy within seconds.
const TTL_MS = 30_000
const cache = new Map<string, { at: number; items: WorkspaceMacro[] }>()

export async function listWorkspaceMacrosCached(workspace: string): Promise<WorkspaceMacro[]> {
	const hit = cache.get(workspace)
	if (hit && Date.now() - hit.at < TTL_MS) return hit.items
	const items = await AssetService.listWorkspaceMacros({ workspace })
	cache.set(workspace, { at: Date.now(), items })
	return items
}

export function invalidateWorkspaceMacros(workspace: string) {
	cache.delete(workspace)
}

/** `name(params)` display signature, with the table-macro arrow. */
export function macroSignature(m: WorkspaceMacro): string {
	return `${m.name}(${m.params})${m.is_table ? ' → table' : ''}`
}

/** Full `CREATE` statement for the copy button / documentation preview. */
export function macroDefinitionSql(m: WorkspaceMacro): string {
	return `CREATE OR REPLACE MACRO ${m.name}(${m.params}) AS ${m.is_table ? 'TABLE ' : ''}${m.body};`
}
