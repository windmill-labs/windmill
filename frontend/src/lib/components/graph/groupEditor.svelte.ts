import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import type { NoteColor } from './noteColors'
import { DEFAULT_GROUP_NOTE_COLOR, getNextAvailableColor } from './noteColors'
import { generateId } from './util'
import { getContext, setContext } from 'svelte'

/**
 * Type for a flow group (matches the generated type from OpenAPI)
 */
export type FlowGroup = {
	id: string
	summary?: string
	collapsed?: boolean
	module_ids: Array<string>
	color?: string
}

/**
 * Utility class for editing flow groups via direct flowStore mutations.
 * Follows the same pattern as NoteEditor.
 */
export class GroupEditor {
	private flowStore: StateStore<ExtendedOpenFlow>

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.flowStore = flowStore
	}

	getGroups(): FlowGroup[] {
		return this.flowStore.val.value?.groups || []
	}

	private setGroups(groups: FlowGroup[]): void {
		if (this.flowStore.val.value) {
			this.flowStore.val.value.groups = groups
		}
	}

	/**
	 * Create a new group containing the specified module IDs.
	 * Returns the generated group ID.
	 */
	createGroup(moduleIds: string[]): string {
		// Filter subflow node IDs (same logic as NoteEditor.createGroupNote)
		let filteredIds = [...moduleIds]
		const subflowIds: string[] = []
		for (const id of moduleIds) {
			if (id.startsWith('subflow:')) {
				const match = id.match(/^subflow:([^:]+)/)
				if (match) {
					subflowIds.push(match[1])
				}
			}
		}
		if (subflowIds.length > 0) {
			filteredIds = filteredIds.filter((id) => !subflowIds.includes(id))
			filteredIds = [...filteredIds, ...subflowIds]
		}

		const groups = this.getGroups()
		const usedColors = new Set<NoteColor>()
		for (const group of groups) {
			if (group.color) {
				usedColors.add(group.color as NoteColor)
			}
		}
		const color =
			usedColors.size > 0 ? getNextAvailableColor(usedColors) : DEFAULT_GROUP_NOTE_COLOR

		const newGroup: FlowGroup = {
			id: generateId(),
			module_ids: filteredIds,
			color
		}
		this.setGroups([...groups, newGroup])
		return newGroup.id
	}

	deleteGroup(groupId: string): void {
		const groups = this.getGroups()
		this.setGroups(groups.filter((g) => g.id !== groupId))
	}

	updateColor(groupId: string, color: NoteColor): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, color } : g)))
	}

	updateSummary(groupId: string, summary: string): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, summary } : g)))
	}

	updateCollapsedDefault(groupId: string, collapsed: boolean): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, collapsed } : g)))
	}

	/**
	 * Returns the smallest group (by module_ids.length) that contains the given module ID.
	 */
	getClosestGroup(moduleId: string): FlowGroup | undefined {
		const groups = this.getGroups()
		let closest: FlowGroup | undefined = undefined
		for (const group of groups) {
			if (group.module_ids.includes(moduleId)) {
				if (!closest || group.module_ids.length < closest.module_ids.length) {
					closest = group
				}
			}
		}
		return closest
	}

	isAvailable(): boolean {
		return !!this.flowStore.val.value
	}
}

export type GroupEditorContext = {
	groupEditor: GroupEditor
}

const CONTEXT_KEY = 'GroupEditorContext'

export function setGroupEditorContext(groupEditor: GroupEditor): void {
	setContext<GroupEditorContext>(CONTEXT_KEY, { groupEditor })
}

export function getGroupEditorContext(): GroupEditorContext | undefined {
	return getContext<GroupEditorContext | undefined>(CONTEXT_KEY)
}
