import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

export type AgentScopeId = 'personal' | 'workspace' | `folder:${string}`

export type AgentContextSelection = {
	version: 2
	baseline: 'selected' | 'deselected'
	overrides: AgentScopeId[]
}

const STORAGE_KEY = 'wm_ai_editable_folders'
const DEFAULT_SELECTION: AgentContextSelection = {
	version: 2,
	baseline: 'selected',
	overrides: []
}

function storageScope(workspace: string): string | undefined {
	const email = get(userStore)?.email
	return email ? `${workspace}:${email}` : undefined
}

function readStorage(): Record<string, unknown> {
	if (typeof localStorage === 'undefined') return {}
	try {
		return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
	} catch {
		return {}
	}
}

function isScopeId(value: string): value is AgentScopeId {
	return value === 'personal' || value === 'workspace' || value.startsWith('folder:')
}

function normalizeSelection(raw: unknown): AgentContextSelection {
	if (raw && typeof raw === 'object' && (raw as any).version === 2) {
		const selection = raw as Partial<AgentContextSelection>
		return {
			version: 2,
			baseline: selection.baseline === 'deselected' ? 'deselected' : 'selected',
			overrides: Array.isArray(selection.overrides)
				? [
						...new Set(
							selection.overrides.filter(
								(value): value is AgentScopeId => typeof value === 'string' && isScopeId(value)
							)
						)
					]
				: []
		}
	}

	// The presentation prototype stored a boolean decision per scope. Preserve those
	// decisions under the original selected-by-default semantics.
	if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
		return {
			version: 2,
			baseline: 'selected',
			overrides: Object.entries(raw)
				.filter(([id, enabled]) => isScopeId(id) && enabled === false)
				.map(([id]) => id as AgentScopeId)
		}
	}

	return { ...DEFAULT_SELECTION }
}

export function getAgentContextSelection(workspace: string): AgentContextSelection {
	const scope = storageScope(workspace)
	return scope ? normalizeSelection(readStorage()[scope]) : { ...DEFAULT_SELECTION }
}

function writeSelection(workspace: string, selection: AgentContextSelection): boolean {
	const scope = storageScope(workspace)
	if (!scope) return false
	const all = readStorage()
	all[scope] = selection
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(all))
		return true
	} catch (error) {
		console.error(`Failed to persist ${STORAGE_KEY}`, error)
		return false
	}
}

export function isEditableFolderSelected(workspace: string, id: string): boolean {
	if (!isScopeId(id)) return false
	const selection = getAgentContextSelection(workspace)
	const baseline = selection.baseline === 'selected'
	return selection.overrides.includes(id) ? !baseline : baseline
}

export function setEditableFolderSelected(
	workspace: string,
	id: string,
	enabled: boolean
): boolean {
	if (!isScopeId(id)) return false
	const selection = getAgentContextSelection(workspace)
	const baseline = selection.baseline === 'selected'
	const overrides = new Set(selection.overrides)
	if (enabled === baseline) overrides.delete(id)
	else overrides.add(id)
	return writeSelection(workspace, { ...selection, overrides: [...overrides] })
}

export function setAllEditableFoldersSelected(workspace: string, enabled: boolean): boolean {
	return writeSelection(workspace, {
		version: 2,
		baseline: enabled ? 'selected' : 'deselected',
		overrides: []
	})
}
