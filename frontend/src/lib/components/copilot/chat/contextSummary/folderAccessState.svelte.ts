import { ACCESS_SCOPES, type AccessScope } from './folderAccess'
import { isEditableFolderSelected, setEditableFolderSelected } from './selectedEditableFolders'

export class FolderAccessState {
	private selectionVersion = $state(0)
	expanded = $state<Record<string, boolean>>({})

	constructor(private readonly workspace: () => string) {}

	get scopes(): AccessScope[] {
		return ACCESS_SCOPES
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
		if (setEditableFolderSelected(this.workspace(), id, value)) this.selectionVersion++
	}

	setAll(value: boolean) {
		let changed = false
		for (const scope of this.scopes) {
			changed = setEditableFolderSelected(this.workspace(), scope.id, value) || changed
		}
		if (changed) this.selectionVersion++
	}

	toggleExpanded(id: string) {
		this.expanded[id] = !this.expanded[id]
	}
}
