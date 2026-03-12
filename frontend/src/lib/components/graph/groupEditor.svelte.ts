import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import { completeAndSplitGroup } from './groupDetectionUtils'
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
	note?: string
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

	// Note height tracking (shared between GroupOverlay and CollapsedGroupNode)
	private _noteHeights = $state<Record<string, number>>({})

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

	/** Set note height for a group (used for layout spacing) */
	setNoteHeight(groupId: string, height: number): void {
		if (this._noteHeights[groupId] !== height) {
			this._noteHeights[groupId] = height
		}
	}

	/** Get all note heights */
	getNoteHeights(): Record<string, number> {
		return this._noteHeights
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
		this.setNoteHeight(groupId, 0)
	}

	updateCollapsedDefault(groupId: string, collapsed_by_default: boolean): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, collapsed_by_default } : g)))
	}

	/**
	 * Returns the smallest group (by module_ids.length) that contains the given module ID.
	 * Also matches collapsed group node IDs (collapsed-group:{groupId}).
	 */
	getClosestGroup(moduleId: string): FlowGroup | undefined {
		const groups = this.getGroups()

		// Check if this is a collapsed group node ID
		if (moduleId.startsWith('collapsed-group:')) {
			const groupId = moduleId.slice('collapsed-group:'.length)
			return groups.find((g) => g.id === groupId)
		}

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

	/** Remove a deleted node from all groups. Removes empty groups. */
	removeNode(nodeId: string): void {
		const groups = this.getGroups()
		let changed = false
		for (const group of groups) {
			const idx = group.module_ids.indexOf(nodeId)
			if (idx !== -1) {
				group.module_ids.splice(idx, 1)
				changed = true
			}
		}
		if (changed) {
			this.setGroups(groups.filter((g) => g.module_ids.length > 0))
		}
	}

	/** Clean up groups: remove stale node IDs, complete paths, and split disconnected components. */
	cleanupGroups(flowNodes: { id: string; parentIds?: string[] }[]): void {
		if (!this.isAvailable()) return

		const groups = this.getGroups()
		if (groups.length === 0) return

		let hasChanges = false
		const nodeSet = new Set(flowNodes.map((n) => n.id))
		const newGroups: FlowGroup[] = []

		for (const group of groups) {
			// Skip collapsed groups — their module nodes are replaced by a single
			// collapsed-group placeholder and won't appear in flowNodes.
			if (this.isRuntimeCollapsed(group.id)) {
				newGroups.push(group)
				continue
			}

			// Step 1: Remove stale module_ids
			const validIds = group.module_ids.filter((id) => nodeSet.has(id))
			if (validIds.length !== group.module_ids.length) {
				group.module_ids = validIds
				hasChanges = true
			}

			if (group.module_ids.length === 0) {
				hasChanges = true
				continue
			}

			// Step 2: Complete paths and split disconnected components
			const components = completeAndSplitGroup(group.module_ids, flowNodes)

			if (components.length <= 1) {
				const completed = components.length > 0 ? components[0] : []
				const sortedCompleted = [...completed].sort()
				const sortedOriginal = [...group.module_ids].sort()

				if (
					sortedCompleted.length !== sortedOriginal.length ||
					!sortedCompleted.every((id, i) => id === sortedOriginal[i])
				) {
					group.module_ids = completed
					hasChanges = true
				}
				if (group.module_ids.length > 0) {
					newGroups.push(group)
				} else {
					hasChanges = true
				}
			} else {
				// Split into multiple groups
				hasChanges = true
				for (const component of components) {
					if (component.length === 0) continue
					newGroups.push({
						...group,
						id: generateId(),
						module_ids: component,
						summary: group.summary ? `${group.summary}` : undefined
					})
				}
			}
		}

		if (hasChanges) {
			this.setGroups(newGroups)
		}
	}

	/** Add a newly inserted node to the group that contains both its neighbors. */
	addInsertedNode(newNodeId: string, sourceId?: string, targetId?: string): void {
		if (!sourceId || !targetId) return
		const groups = this.getGroups()
		for (const group of groups) {
			if (group.module_ids.includes(sourceId) && group.module_ids.includes(targetId)) {
				group.module_ids.push(newNodeId)
				this.setGroups(groups)
				return
			}
		}
	}

	/** Handle a node that was moved to a new position in the flow. */
	handleNodeMoved(movedId: string, sourceId?: string, targetId?: string): void {
		const groups = this.getGroups()
		const currentGroup = groups.find((g) => g.module_ids.includes(movedId))

		if (currentGroup) {
			// Was in a group — only keep if BOTH neighbors are in the same group
			const sourceInGroup = sourceId ? currentGroup.module_ids.includes(sourceId) : false
			const targetInGroup = targetId ? currentGroup.module_ids.includes(targetId) : false
			if (!(sourceInGroup && targetInGroup)) {
				// Moved to boundary or outside the group — remove
				currentGroup.module_ids = currentGroup.module_ids.filter((id) => id !== movedId)
				this.setGroups(groups.filter((g) => g.module_ids.length > 0))
			}
		} else {
			// Wasn't in a group — check if moved into one
			this.addInsertedNode(movedId, sourceId, targetId)
		}
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
export const GROUP_TOP_MARGIN = 20

/**
 * Compute adjusted node positions for collapsed groups whose note is visible.
 * Pushes nodes below the collapsed group node down by the note height.
 */
export function computeCollapsedGroupNoteSpacing(
	collapsedGroups: FlowGroup[],
	nodes: Array<{ id: string; position: { x: number; y: number } }>,
	noteHeights: Record<string, number>
): Record<string, { x: number; y: number }> {
	const collapsedNoteHeights: Record<string, number> = {}
	for (const group of collapsedGroups) {
		const h = noteHeights[group.id] ?? 0
		if (h > 0) {
			collapsedNoteHeights[`collapsed-group:${group.id}`] = h
		}
	}

	if (Object.keys(collapsedNoteHeights).length === 0) {
		return Object.fromEntries(nodes.map((n) => [n.id, { ...n.position }]))
	}

	const sortedNodes = nodes
		.map((n) => ({ id: n.id, position: { ...n.position } }))
		.sort((a, b) => a.position.y - b.position.y)

	let cumulativeOffset = 0
	let prevYPos = NaN
	let pendingOffset = 0

	for (const node of sortedNodes) {
		if (node.position.y !== prevYPos) {
			cumulativeOffset += pendingOffset
			pendingOffset = 0
			prevYPos = node.position.y
		}
		node.position.y += cumulativeOffset

		if (collapsedNoteHeights[node.id]) {
			pendingOffset = Math.max(pendingOffset, collapsedNoteHeights[node.id])
		}
	}

	return Object.fromEntries(sortedNodes.map((n) => [n.id, n.position]))
}

/**
 * Compute adjusted node positions that account for group label spacing.
 * Follows the same push-down pattern as computeNoteNodes in noteUtils.
 */
export function computeGroupSpacing(
	groups: FlowGroup[],
	nodes: Array<{ id: string; position: { x: number; y: number } }>,
	noteHeights?: Record<string, number>
): Record<string, { x: number; y: number }> {
	if (groups.length === 0) {
		return Object.fromEntries(nodes.map((n) => [n.id, { ...n.position }]))
	}

	// Build yPosMap: Y position → spacing needed
	const yPosMap: Record<number, number> = {}

	for (const group of groups) {
		if (group.module_ids.length === 0) continue

		// Find topmost node Y position in this group
		// Check both member nodes (uncollapsed) and collapsed-group node (collapsed)
		let topY = Infinity
		let isCollapsed = false
		const collapsedNodeId = `collapsed-group:${group.id}`
		for (const node of nodes) {
			if (node.id === collapsedNodeId && node.position.y < topY) {
				topY = node.position.y
				isCollapsed = true
			} else if (group.module_ids.includes(node.id) && node.position.y < topY) {
				topY = node.position.y
				isCollapsed = false
			}
		}

		if (topY < Infinity) {
			const noteHeight = noteHeights?.[group.id] ?? 0
			// Collapsed groups only need header height (no top padding gap)
			const spacing = isCollapsed
				? GROUP_HEADER_HEIGHT + noteHeight
				: GROUP_HEADER_HEIGHT + noteHeight + GROUP_TOP_MARGIN
			yPosMap[topY] = Math.max(yPosMap[topY] || 0, spacing)
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
