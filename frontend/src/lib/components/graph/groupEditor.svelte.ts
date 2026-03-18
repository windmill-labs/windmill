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

		const result = canFormValidGroup(filteredIds, flowNodes)
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

export type GroupedModule =
	| FlowModule
	| {
			type: 'group'
			group: FlowGroup
			collapsed: boolean
			modules: GroupedModule[]
			moduleIds: string[]
	  }

export type GraphGroup = FlowGroup & {
	collapsed: boolean
	moduleIds: string[]
}

export function buildGroupedModules(modules: FlowModule[], groups: GraphGroup[]): GroupedModule[] {
	const { items, consumed } = buildGroupedModulesRecurse(modules, groups)
	const unconsumed = groups.filter((g) => !consumed.has(g.id))
	if (unconsumed.length > 0) {
		console.warn(
			'Groups with no matching modules (ill-formed):',
			unconsumed.map((g) => g.id)
		)
	}
	return items
}

/** Get inner arrays from a container FlowModule with direct get/set accessors.
 *  Safe to mutate — callers deep-copy modules before calling buildGroupedModules. */
function getContainerInnerArrays(
	mod: FlowModule
): { get: () => FlowModule[]; set: (v: any) => void }[] {
	const val = mod.value as any
	if (val.type === 'forloopflow' || val.type === 'whileloopflow') {
		return [{ get: () => val.modules, set: (v) => (val.modules = v) }]
	} else if (val.type === 'branchone') {
		return [
			{ get: () => val.default, set: (v) => (val.default = v) },
			...val.branches.map((b: any) => ({
				get: () => b.modules,
				set: (v: any) => (b.modules = v)
			}))
		]
	} else if (val.type === 'branchall') {
		return val.branches.map((b: any) => ({
			get: () => b.modules,
			set: (v: any) => (b.modules = v)
		}))
	}
	return []
}

function buildGroupedModulesRecurse(
	modules: FlowModule[],
	groups: GraphGroup[]
): { items: GroupedModule[]; consumed: Set<string> } {
	const indexMap = new Map<string, number>()
	for (let i = 0; i < modules.length; i++) {
		indexMap.set(modules[i].id, i)
	}

	// Partition: groups for this level vs rest
	const levelGroups: GraphGroup[] = []
	const otherGroups: GraphGroup[] = []
	for (const g of groups) {
		if (indexMap.has(g.start_id) && indexMap.has(g.end_id)) {
			const s = indexMap.get(g.start_id)!
			const e = indexMap.get(g.end_id)!
			if (s > e) {
				throw new Error(
					`Group '${g.id}' has inverted range: start_id='${g.start_id}' (index ${s}) > end_id='${g.end_id}' (index ${e})`
				)
			}
			levelGroups.push(g)
		} else {
			otherGroups.push(g)
		}
	}

	// Validate no partial overlaps among level groups
	for (let i = 0; i < levelGroups.length; i++) {
		for (let j = i + 1; j < levelGroups.length; j++) {
			const a = levelGroups[i]
			const b = levelGroups[j]
			const aStart = indexMap.get(a.start_id)!
			const aEnd = indexMap.get(a.end_id)!
			const bStart = indexMap.get(b.start_id)!
			const bEnd = indexMap.get(b.end_id)!

			if (aEnd < bStart || bEnd < aStart) continue
			if (aStart <= bStart && bEnd <= aEnd) continue
			if (bStart <= aStart && aEnd <= bEnd) continue

			throw new Error(`Groups '${a.id}' and '${b.id}' overlap without nesting`)
		}
	}

	// Build grouped structure for this level
	function build(startIdx: number, endIdx: number, availableGroups: GraphGroup[]): GroupedModule[] {
		const result: GroupedModule[] = []
		let i = startIdx
		while (i <= endIdx) {
			const candidates = availableGroups.filter((g) => {
				const gStart = indexMap.get(g.start_id)!
				const gEnd = indexMap.get(g.end_id)!
				return gStart === i && gEnd <= endIdx
			})
			candidates.sort((a, b) => {
				const spanA = indexMap.get(a.end_id)! - indexMap.get(a.start_id)!
				const spanB = indexMap.get(b.end_id)! - indexMap.get(b.start_id)!
				return spanB - spanA
			})

			const group = candidates[0]
			if (group) {
				const gEnd = indexMap.get(group.end_id)!
				const remaining = availableGroups.filter((g) => g.id !== group.id)
				const innerModules = build(i, gEnd, remaining)

				const moduleIds: string[] = []
				for (let k = i; k <= gEnd; k++) {
					moduleIds.push(modules[k].id)
				}

				result.push({
					type: 'group',
					group: {
						id: group.id,
						summary: group.summary,
						note: group.note,
						color: group.color,
						collapsed_by_default: group.collapsed_by_default,
						start_id: group.start_id,
						end_id: group.end_id
					},
					collapsed: group.collapsed,
					modules: innerModules,
					moduleIds
				})
				i = gEnd + 1
			} else {
				result.push(modules[i])
				i++
			}
		}
		return result
	}

	const result = build(0, modules.length - 1, levelGroups)

	// Recurse into containers and groups with remaining unconsumed groups
	const consumed = new Set(levelGroups.map((g) => g.id))
	let remaining = otherGroups

	function recurseIntoContainers(items: GroupedModule[]): void {
		for (const item of items) {
			if ('type' in item && item.type === 'group') {
				recurseIntoContainers((item as any).modules)
				continue
			}
			const mod = item as FlowModule
			for (const { get, set } of getContainerInnerArrays(mod)) {
				const inner = buildGroupedModulesRecurse(get(), remaining)
				set(inner.items as any)
				for (const id of inner.consumed) consumed.add(id)
				remaining = remaining.filter((g) => !inner.consumed.has(g.id))
			}
		}
	}
	recurseIntoContainers(result)

	return { items: result, consumed }
}
