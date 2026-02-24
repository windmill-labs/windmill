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
	description?: string
	collapsed_by_default?: boolean
	module_ids: Array<string>
	color?: string
}

/**
 * Utility class for editing flow groups via direct flowStore mutations.
 * Follows the same pattern as NoteEditor.
 */
export class GroupEditor {
	private flowStore: StateStore<ExtendedOpenFlow>

	// Runtime collapsed state (not persisted in flow YAML)
	private _runtimeCollapsedIds = $state<Set<string>>(new Set())
	private _runtimeInitialized = $state(false)

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.flowStore = flowStore
	}

	/** Initialize runtime state from collapsed_by_default. Safe to call from event handlers. */
	private ensureRuntimeInitialized(): void {
		if (this._runtimeInitialized) return
		const groups = this.getGroups()
		this._runtimeCollapsedIds = new Set(
			groups.filter((g) => g.collapsed_by_default).map((g) => g.id)
		)
		this._runtimeInitialized = true
	}

	/** Check if a group is currently collapsed (runtime). Safe to call from $derived. */
	isRuntimeCollapsed(groupId: string): boolean {
		if (!this._runtimeInitialized) {
			return this.getGroups().find((g) => g.id === groupId)?.collapsed_by_default ?? false
		}
		return this._runtimeCollapsedIds.has(groupId)
	}

	/** Toggle runtime collapse (Minimize2 button) */
	toggleRuntimeCollapse(groupId: string): void {
		this.ensureRuntimeInitialized()
		const next = new Set(this._runtimeCollapsedIds)
		if (next.has(groupId)) next.delete(groupId)
		else next.add(groupId)
		this._runtimeCollapsedIds = next
	}

	/** Expand a group at runtime (CollapsedGroupNode click) */
	expandGroup(groupId: string): void {
		this.ensureRuntimeInitialized()
		const next = new Set(this._runtimeCollapsedIds)
		next.delete(groupId)
		this._runtimeCollapsedIds = next
	}

	/** Get currently collapsed groups for graph builder. Safe to call from $derived. */
	getCollapsedGroups(): FlowGroup[] {
		if (!this._runtimeInitialized) {
			return this.getGroups().filter((g) => g.collapsed_by_default)
		}
		return this.getGroups().filter((g) => this._runtimeCollapsedIds.has(g.id))
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
		const color = usedColors.size > 0 ? getNextAvailableColor(usedColors) : DEFAULT_GROUP_NOTE_COLOR

		const newGroup: FlowGroup = {
			id: generateId(),
			description: '',
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

	updateDescription(groupId: string, description: string): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, description } : g)))
	}

	updateCollapsedDefault(groupId: string, collapsed_by_default: boolean): void {
		const groups = this.getGroups()
		this.setGroups(
			groups.map((g) => (g.id === groupId ? { ...g, collapsed_by_default } : g))
		)
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

/** Extra vertical space pushed above the topmost node of each group for the header card */
export const GROUP_HEADER_HEIGHT = 40

const GROUP_TOP_MARGIN = 16

/**
 * Compute adjusted node positions that account for group label spacing.
 * Follows the same push-down pattern as computeNoteNodes in noteUtils.
 */
export function computeGroupSpacing(
	groups: FlowGroup[],
	nodes: Array<{ id: string; position: { x: number; y: number } }>
): Record<string, { x: number; y: number }> {
	if (groups.length === 0) {
		return Object.fromEntries(nodes.map((n) => [n.id, { ...n.position }]))
	}

	// Build yPosMap: Y position → spacing needed
	const yPosMap: Record<number, number> = {}

	for (const group of groups) {
		if (group.module_ids.length === 0) continue

		// Find topmost node Y position in this group
		let topY = Infinity
		for (const node of nodes) {
			if (group.module_ids.includes(node.id) && node.position.y < topY) {
				topY = node.position.y
			}
		}

		if (topY < Infinity) {
			yPosMap[topY] = Math.max(yPosMap[topY] || 0, GROUP_HEADER_HEIGHT) + GROUP_TOP_MARGIN
		}
	}

	// Sort nodes by Y and apply cumulative offset
	const sortedNodes = nodes
		.map((n) => ({ id: n.id, position: { ...n.position } }))
		.sort((a, b) => a.position.y - b.position.y)

	let currentYOffset = 0
	let prevYPos = NaN

	for (const node of sortedNodes) {
		if (node.position.y !== prevYPos) {
			if (yPosMap[node.position.y]) {
				currentYOffset += yPosMap[node.position.y]
			}
			prevYPos = node.position.y
		}
		node.position.y += currentYOffset
	}

	return Object.fromEntries(sortedNodes.map((n) => [n.id, n.position]))
}
