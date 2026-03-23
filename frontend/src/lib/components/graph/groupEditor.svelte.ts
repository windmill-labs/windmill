import type { FlowModule } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'

import { canFormValidGroup, computeGroupModuleIds } from './groupDetectionUtils'
import type { NoteColor } from './noteColors'
import { DEFAULT_GROUP_NOTE_COLOR, getNextAvailableColor } from './noteColors'
import { generateId } from './util'
import { getContext, setContext } from 'svelte'

/**
 * Type for a flow group (matches the generated type from OpenAPI).
 * Members are computed dynamically from all nodes on paths between start_id and end_id.
 */
export type FlowGroup = {
	id: string
	summary?: string
	note?: string
	collapsed_by_default?: boolean
	start_id: string
	end_id: string
	color?: string
}

/**
 * Display state for flow groups inside the graph.
 * Handles runtime collapse state and note height tracking.
 * Similar to NoteManager — instantiated inside FlowGraphV2.
 */
export class GroupDisplayState {
	#getGroups: () => FlowGroup[]
	#runtimeCollapsedIds = $state<Set<string>>(new Set())
	#runtimeInitialized = $state(false)
	#noteHeights = $state<Record<string, number>>({})
	renderCount = $state(0)

	constructor(getGroups: () => FlowGroup[]) {
		this.#getGroups = getGroups
	}

	/** Initialize runtime state from collapsed_by_default. Safe to call from event handlers. */
	private ensureRuntimeInitialized(): void {
		if (this.#runtimeInitialized) return
		const groups = this.#getGroups()
		this.#runtimeCollapsedIds = new Set(
			groups.filter((g) => g.collapsed_by_default).map((g) => g.id)
		)
		this.#runtimeInitialized = true
	}

	/** Check if a group is currently collapsed (runtime). Safe to call from $derived. */
	isRuntimeCollapsed(groupId: string): boolean {
		if (!this.#runtimeInitialized) {
			return this.#getGroups().find((g) => g.id === groupId)?.collapsed_by_default ?? false
		}
		return this.#runtimeCollapsedIds.has(groupId)
	}

	/** Toggle runtime collapse (Minimize2 button) */
	toggleRuntimeCollapse(groupId: string): void {
		this.ensureRuntimeInitialized()
		const next = new Set(this.#runtimeCollapsedIds)
		if (next.has(groupId)) next.delete(groupId)
		else next.add(groupId)
		this.#runtimeCollapsedIds = next
		this.render()
	}

	/** Expand a group at runtime (CollapsedGroupNode click) */
	expandGroup(groupId: string): void {
		this.ensureRuntimeInitialized()
		const next = new Set(this.#runtimeCollapsedIds)
		next.delete(groupId)
		this.#runtimeCollapsedIds = next
		this.render()
	}

	/** Set note height for a group (used for layout spacing) */
	setNoteHeight(groupId: string, height: number): void {
		if (this.#noteHeights[groupId] !== height) {
			this.#noteHeights[groupId] = height
			this.render()
		}
	}

	/** Get all note heights */
	getNoteHeights(): Record<string, number> {
		return this.#noteHeights
	}

	/** Bump render counter to trigger re-layout */
	render(): void {
		this.renderCount++
	}

	/** Get currently collapsed groups for graph builder. Safe to call from $derived. */
	getCollapsedGroups(): FlowGroup[] {
		if (!this.#runtimeInitialized) {
			return this.#getGroups().filter((g) => g.collapsed_by_default)
		}
		return this.#getGroups().filter((g) => this.#runtimeCollapsedIds.has(g.id))
	}
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

	/** IDs that cannot be part of a group (preprocessor, failure module) */
	getExcludeIds(): Set<string> {
		const excludeIds = new Set<string>()
		const pp = this.flowStore.val.value?.preprocessor_module?.id
		if (pp) excludeIds.add(pp)
		const fm = this.flowStore.val.value?.failure_module?.id
		if (fm) excludeIds.add(fm)
		return excludeIds
	}

	/** Check whether the given selection can form a valid group */
	canCreateGroup(
		selectedIds: string[],
		flowNodes: { id: string; parentIds?: string[] }[]
	): boolean {
		return canFormValidGroup(selectedIds, flowNodes, this.getExcludeIds()).valid
	}

	/**
	 * Create a new group from selected node IDs.
	 * Uses canFormValidGroup to determine start_id and end_id.
	 * Returns the generated group ID.
	 */
	createGroup(
		moduleIds: string[],
		flowNodes: { id: string; parentIds?: string[] }[]
	): string | undefined {
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

		const result = canFormValidGroup(filteredIds, flowNodes, this.getExcludeIds())
		if (!result.valid) return undefined

		const groups = this.getGroups()
		const usedColors = new Set<NoteColor>()
		for (const group of groups) {
			if (group.color) {
				usedColors.add(group.color as NoteColor)
			}
		}
		const color = usedColors.size > 0 ? getNextAvailableColor(usedColors) : DEFAULT_GROUP_NOTE_COLOR

		const newGroup: FlowGroup = {
			id: generateId(),
			start_id: result.startId,
			end_id: result.endId,
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

	updateNote(groupId: string, note: string | undefined): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, note } : g)))
	}

	/** Add a note to a group (sets note to empty string to trigger the placeholder UI) */
	addNote(groupId: string): void {
		this.updateNote(groupId, '')
	}

	/** Remove a note from a group */
	removeNote(groupId: string): void {
		this.updateNote(groupId, undefined)
	}

	updateCollapsedDefault(groupId: string, collapsed_by_default: boolean): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, collapsed_by_default } : g)))
	}

	/**
	 * Handle a node leaving a group (deleted or moved outside).
	 * If nodeId === start_id or end_id, shift the boundary to the next adjacent module.
	 * The group is preserved even if only one (or zero) members remain.
	 * @param allModules - flattened module list captured BEFORE the node was removed/moved
	 */
	removeNode(nodeId: string, allModules: { id: string }[]): void {
		const groups = this.getGroups()
		let hasChanges = false
		const newGroups: FlowGroup[] = []

		for (const group of groups) {
			if (group.start_id !== nodeId && group.end_id !== nodeId) {
				newGroups.push(group)
				continue
			}

			const memberIds = computeGroupModuleIds(group.start_id, group.end_id, allModules)
			const remaining = memberIds.filter((id) => id !== nodeId)

			hasChanges = true

			// Drop groups with no remaining members
			if (remaining.length === 0) {
				continue
			}

			// Shift boundary to the next adjacent module
			newGroups.push({
				...group,
				start_id: group.start_id === nodeId ? remaining[0] : group.start_id,
				end_id: group.end_id === nodeId ? remaining[remaining.length - 1] : group.end_id
			})
		}

		if (hasChanges) {
			this.setGroups(newGroups)
		}
	}

	/**
	 * Pre-check: return groups that would have 0 remaining members after removing nodeIds.
	 * Used to gate delete/move actions behind a confirmation modal.
	 */
	getGroupsEmptiedBy(nodeIds: string[], allModules: { id: string }[]): FlowGroup[] {
		const nodeSet = new Set(nodeIds)
		const result: FlowGroup[] = []
		for (const group of this.getGroups()) {
			const memberIds = computeGroupModuleIds(group.start_id, group.end_id, allModules)
			const remaining = memberIds.filter((id) => !nodeSet.has(id))
			if (remaining.length < memberIds.length && remaining.length === 0) {
				result.push(group)
			}
		}
		return result
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

/** Height of the group header bar */
export const GROUP_HEADER_HEIGHT = 22

/** Extra margin between the header and the first node */
export const GROUP_TOP_MARGIN = 30

export type GraphGroup = FlowGroup & {
	moduleIds: string[]
}

export type ContainerInnerArray = {
	get: () => FlowModule[]
	set: (v: any) => void
	label?: string
}

/** Get inner arrays from a container FlowModule with direct get/set accessors. */
export function getContainerInnerArrays(mod: FlowModule): ContainerInnerArray[] {
	const val = mod.value as any
	if (val.type === 'forloopflow' || val.type === 'whileloopflow') {
		return [
			{
				get: () => val.modules,
				set: (v) => {
					val.modules = v
				}
			}
		]
	} else if (val.type === 'branchone') {
		return [
			{
				get: () => val.default,
				set: (v) => {
					val.default = v
				},
				label: 'Default'
			},
			...val.branches.map((b: any, i: number) => ({
				get: () => b.modules,
				set: (v: any) => {
					b.modules = v
				},
				label: b.summary || `Branch ${i + 1}`
			}))
		]
	} else if (val.type === 'branchall') {
		return val.branches.map((b: any, i: number) => ({
			get: () => b.modules,
			set: (v: any) => {
				b.modules = v
			},
			label: b.summary || `Branch ${i + 1}`
		}))
	}
	return []
}
