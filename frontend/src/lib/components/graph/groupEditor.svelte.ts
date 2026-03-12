import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import { canFormValidGroup, type GroupMembership } from './groupDetectionUtils'
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
	 * Create a new group from selected node IDs.
	 * Uses canFormValidGroup to determine start_id and end_id.
	 * Returns the generated group ID.
	 */
	createGroup(
		moduleIds: string[],
		flowNodes: { id: string; parentIds?: string[] }[],
		containerDescendants: Map<string, string[]>
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

		const result = canFormValidGroup(filteredIds, flowNodes, containerDescendants)
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
		this.setNoteHeight(groupId, 0)
	}

	updateCollapsedDefault(groupId: string, collapsed_by_default: boolean): void {
		const groups = this.getGroups()
		this.setGroups(groups.map((g) => (g.id === groupId ? { ...g, collapsed_by_default } : g)))
	}

	/**
	 * Returns the smallest group (by member count) that contains the given module ID.
	 * Also matches collapsed group node IDs (collapsed-group:{groupId}).
	 * Requires pre-computed membership map.
	 */
	getClosestGroup(
		moduleId: string,
		groupMemberships: Map<string, GroupMembership>
	): FlowGroup | undefined {
		const groups = this.getGroups()

		// Check if this is a collapsed group node ID
		if (moduleId.startsWith('collapsed-group:')) {
			const groupId = moduleId.slice('collapsed-group:'.length)
			return groups.find((g) => g.id === groupId)
		}

		let closest: FlowGroup | undefined = undefined
		let closestSize = Infinity
		for (const group of groups) {
			const membership = groupMemberships.get(group.id)
			if (membership && membership.memberIds.includes(moduleId)) {
				if (membership.memberIds.length < closestSize) {
					closest = group
					closestSize = membership.memberIds.length
				}
			}
		}
		return closest
	}

	isAvailable(): boolean {
		return !!this.flowStore.val.value
	}

	/**
	 * Remove a deleted node from groups.
	 * If nodeId === start_id or end_id: delete the group (cleanupGroups on next render will
	 * catch any edge cases). If between: no-op (membership recomputes dynamically).
	 */
	removeNode(nodeId: string): void {
		const groups = this.getGroups()
		const newGroups = groups.filter((g) => g.start_id !== nodeId && g.end_id !== nodeId)
		if (newGroups.length !== groups.length) {
			this.setGroups(newGroups)
		}
	}

	/**
	 * Clean up groups: check start_id and end_id still exist, delete if not.
	 * Much simpler than the old module_ids-based cleanup.
	 */
	cleanupGroups(flowNodes: { id: string; parentIds?: string[] }[]): void {
		if (!this.isAvailable()) return

		const groups = this.getGroups()
		if (groups.length === 0) return

		const nodeSet = new Set(flowNodes.map((n) => n.id))
		const newGroups: FlowGroup[] = []
		let hasChanges = false

		for (const group of groups) {
			// Skip collapsed groups — their module nodes are replaced by a single
			// collapsed-group placeholder and won't appear in flowNodes.
			if (this.isRuntimeCollapsed(group.id)) {
				newGroups.push(group)
				continue
			}

			if (nodeSet.has(group.start_id) && nodeSet.has(group.end_id)) {
				newGroups.push(group)
			} else {
				hasChanges = true
			}
		}

		if (hasChanges) {
			this.setGroups(newGroups)
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
 * Accepts a membership map to look up computed members per group.
 */
export function computeGroupSpacing(
	groups: FlowGroup[],
	groupMemberships: Map<string, GroupMembership>,
	nodes: Array<{ id: string; position: { x: number; y: number } }>,
	noteHeights?: Record<string, number>
): Record<string, { x: number; y: number }> {
	if (groups.length === 0) {
		return Object.fromEntries(nodes.map((n) => [n.id, { ...n.position }]))
	}

	// Build yPosMap: Y position → spacing needed
	const yPosMap: Record<number, number> = {}

	for (const group of groups) {
		const memberIds = groupMemberships.get(group.id)?.memberIds ?? []
		if (memberIds.length === 0) continue

		// Find topmost node Y position in this group
		// Check both member nodes (uncollapsed) and collapsed-group node (collapsed)
		let topY = Infinity
		let isCollapsed = false
		const collapsedNodeId = `collapsed-group:${group.id}`
		for (const node of nodes) {
			if (node.id === collapsedNodeId && node.position.y < topY) {
				topY = node.position.y
				isCollapsed = true
			} else if (memberIds.includes(node.id) && node.position.y < topY) {
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
