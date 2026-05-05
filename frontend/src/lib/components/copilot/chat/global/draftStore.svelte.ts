export type GlobalWorkspaceItemType = 'script' | 'flow' | 'app'

export type GlobalDraftStatus = 'new' | 'modified'

export type GlobalDraftBase = {
	source: 'workspace'
	version?: string | number
	editedAt?: string
	value: unknown
}

export type GlobalDraftItem = {
	type: GlobalWorkspaceItemType
	path: string
	status: GlobalDraftStatus
	base?: GlobalDraftBase
	draft: unknown
	updatedAt: string
}

export function getGlobalDraftKey(type: GlobalWorkspaceItemType, path: string): string {
	return `${type}:${path}`
}

function cloneValue<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T
}

class GlobalDraftStore {
	private drafts = $state<Record<string, GlobalDraftItem>>({})

	listDrafts(): GlobalDraftItem[] {
		return Object.values(this.drafts).map((draft) => cloneValue(draft))
	}

	getDraft(type: GlobalWorkspaceItemType, path: string): GlobalDraftItem | undefined {
		const draft = this.drafts[getGlobalDraftKey(type, path)]
		return draft ? cloneValue(draft) : undefined
	}

	setNewDraft(
		type: GlobalWorkspaceItemType,
		path: string,
		draft: unknown,
		overwrite = false
	): GlobalDraftItem {
		const key = getGlobalDraftKey(type, path)
		if (this.drafts[key] && !overwrite) {
			throw new Error(`A draft already exists for ${type} "${path}".`)
		}

		const item: GlobalDraftItem = {
			type,
			path,
			status: 'new',
			draft: cloneValue(draft),
			updatedAt: new Date().toISOString()
		}
		this.drafts[key] = item
		return cloneValue(item)
	}

	setModifiedDraft(
		type: GlobalWorkspaceItemType,
		path: string,
		base: GlobalDraftBase,
		draft: unknown
	): GlobalDraftItem {
		const key = getGlobalDraftKey(type, path)
		const item: GlobalDraftItem = {
			type,
			path,
			status: 'modified',
			base: cloneValue(base),
			draft: cloneValue(draft),
			updatedAt: new Date().toISOString()
		}
		this.drafts[key] = item
		return cloneValue(item)
	}

	deleteDraft(type: GlobalWorkspaceItemType, path: string): void {
		delete this.drafts[getGlobalDraftKey(type, path)]
	}

	clearDrafts(): void {
		this.drafts = {}
	}

	getScriptDraft(path: string): GlobalDraftItem | undefined {
		return this.getDraft('script', path)
	}

	getFlowDraft(path: string): GlobalDraftItem | undefined {
		return this.getDraft('flow', path)
	}

	getAppDraft(path: string): GlobalDraftItem | undefined {
		return this.getDraft('app', path)
	}
}

export const globalDraftStore = new GlobalDraftStore()
