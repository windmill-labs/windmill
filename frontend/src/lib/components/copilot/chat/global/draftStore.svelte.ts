import type { FlowValue, ScriptLang } from '$lib/gen/types.gen'

export type WorkspaceItemType = 'script' | 'flow'

export type WorkspaceItem = {
	type: WorkspaceItemType
	path: string
	summary?: string
	language?: ScriptLang
	value?: string | FlowValue
	isDraft: boolean
}

export function getWorkspaceItemKey(type: WorkspaceItemType, path: string): string {
	return `${type}:${path}`
}

function clone<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T
}

class GlobalDraftStore {
	private drafts = $state<Record<string, WorkspaceItem>>({})

	listDrafts(): WorkspaceItem[] {
		return Object.values(this.drafts).map(clone)
	}

	getDraft(type: WorkspaceItemType, path: string): WorkspaceItem | undefined {
		const draft = this.drafts[getWorkspaceItemKey(type, path)]
		return draft ? clone(draft) : undefined
	}

	setDraft(item: WorkspaceItem): WorkspaceItem {
		const stored: WorkspaceItem = { ...clone(item), isDraft: true }
		this.drafts[getWorkspaceItemKey(item.type, item.path)] = stored
		return clone(stored)
	}

	deleteDraft(type: WorkspaceItemType, path: string): void {
		delete this.drafts[getWorkspaceItemKey(type, path)]
	}

	clearDrafts(): void {
		this.drafts = {}
	}

	getScriptDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('script', path)
	}

	getFlowDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('flow', path)
	}
}

export const globalDraftStore = new GlobalDraftStore()
