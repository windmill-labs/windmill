import type { ScriptLang } from '$lib/gen/types.gen'

export type GlobalWorkspaceItemType = 'script' | 'flow'

export type GlobalWorkspaceItem = {
	type: GlobalWorkspaceItemType
	path: string
	summary?: string
	value: unknown
	language?: ScriptLang
	isDraft: boolean
}

export function getGlobalDraftKey(type: GlobalWorkspaceItemType, path: string): string {
	return `${type}:${path}`
}

function cloneValue<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T
}

type DraftInput = Omit<GlobalWorkspaceItem, 'isDraft'>

function toDraftItem(input: DraftInput): GlobalWorkspaceItem {
	return {
		...cloneValue(input),
		isDraft: true
	}
}

class GlobalDraftStore {
	private drafts = $state<Record<string, GlobalWorkspaceItem>>({})

	listDrafts(): GlobalWorkspaceItem[] {
		return Object.values(this.drafts).map((draft) => cloneValue(draft))
	}

	getDraft(type: GlobalWorkspaceItemType, path: string): GlobalWorkspaceItem | undefined {
		const draft = this.drafts[getGlobalDraftKey(type, path)]
		return draft ? cloneValue(draft) : undefined
	}

	setNewDraft(input: DraftInput, overwrite = false): GlobalWorkspaceItem {
		const key = getGlobalDraftKey(input.type, input.path)
		if (this.drafts[key] && !overwrite) {
			throw new Error(`A draft already exists for ${input.type} "${input.path}".`)
		}

		const item = toDraftItem(input)
		this.drafts[key] = item
		return cloneValue(item)
	}

	setModifiedDraft(input: DraftInput): GlobalWorkspaceItem {
		const item = toDraftItem(input)
		this.drafts[getGlobalDraftKey(input.type, input.path)] = item
		return cloneValue(item)
	}

	deleteDraft(type: GlobalWorkspaceItemType, path: string): void {
		delete this.drafts[getGlobalDraftKey(type, path)]
	}

	clearDrafts(): void {
		this.drafts = {}
	}

	getScriptDraft(path: string): GlobalWorkspaceItem | undefined {
		return this.getDraft('script', path)
	}

	getFlowDraft(path: string): GlobalWorkspaceItem | undefined {
		return this.getDraft('flow', path)
	}
}

export const globalDraftStore = new GlobalDraftStore()
