import type { AccessScope } from './folderAccess'
import type { AiSkillListItem } from '../global/core'
import type { McpServer } from '../global/mcpTools'
import { buildLiveAccessScopes, loadLiveAccessItems } from './liveFolderAccess'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import {
	isEditableFolderSelected,
	setAllEditableFoldersSelected,
	setEditableFolderSelected
} from './selectedEditableFolders'

export class FolderAccessState {
	private selectionVersion = $state(0)
	private itemEntries = $state.raw<import('./folderAccess').AccessEntry[]>([])
	private itemRefreshId = 0
	expanded = $state<Record<string, boolean>>({})

	constructor(
		private readonly workspace: () => string,
		private readonly username: () => string,
		private readonly folders: () => string[],
		private readonly skills: () => AiSkillListItem[],
		private readonly mcpServers: () => McpServer[],
		private readonly onSelectionChange: () => void
	) {}

	get scopes(): AccessScope[] {
		return buildLiveAccessScopes(
			this.username(),
			this.folders(),
			this.skills(),
			this.mcpServers(),
			this.itemEntries
		)
	}

	async refreshItems(workspace = this.workspace()) {
		const refreshId = ++this.itemRefreshId
		try {
			const entries = await loadLiveAccessItems(workspace)
			if (refreshId === this.itemRefreshId && workspace === this.workspace()) {
				this.itemEntries = entries
			}
		} catch (error) {
			console.error('Failed to load AI context folder contents', error)
			if (refreshId === this.itemRefreshId) this.itemEntries = []
		}
	}

	isSelected(id: string): boolean {
		this.selectionVersion
		return isEditableFolderSelected(this.workspace(), id)
	}

	get selectedScopes(): AccessScope[] {
		return this.scopes.filter((scope) => this.isSelected(scope.id))
	}

	get selectedEntries() {
		return this.selectedScopes.flatMap((scope) => scope.entries)
	}

	setSelected(id: string, value: boolean) {
		if (setEditableFolderSelected(this.workspace(), id, value)) {
			this.selectionVersion++
			const selected = this.selectedScopes.length
			logFeatureUsage('ai_session', 'context_scope_change', {
				key: selected === 0 ? 'none' : selected === this.scopes.length ? 'all' : 'partial',
				workspace: this.workspace()
			})
			this.onSelectionChange()
		}
	}

	setAll(value: boolean) {
		if (setAllEditableFoldersSelected(this.workspace(), value)) {
			this.selectionVersion++
			logFeatureUsage('ai_session', 'context_scope_change', {
				key: value ? 'all' : 'none',
				workspace: this.workspace()
			})
			this.onSelectionChange()
		}
	}

	toggleExpanded(id: string) {
		this.expanded[id] = !this.expanded[id]
	}
}
