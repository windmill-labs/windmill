import { NODE } from './util'

export type DropZone = {
	edgeId: string
	sourceId: string
	targetId: string
	branch: { rootId: string; branch: number } | undefined
	index: number
	flowPosition: { x: number; y: number }
	disableMoveIds: string[]
}

export type DropZoneRegistration = {
	sourceId: string
	targetId: string
	branch: { rootId: string; branch: number } | undefined
	index: number
	disableMoveIds: string[]
	centerX: number
	centerY: number
}

type DragInfo = {
	moduleId: string
	selectedIds?: string[]
}

/**
 * Compute the set of node IDs belonging to a subflow (or a single step).
 * Includes the module itself, all nested children (id starts with `moduleId-`),
 * and structural edge endpoints whose `disableMoveIds` reference this module.
 */
export function getSubflowNodeIds(
	moduleId: string,
	allNodes: { id: string }[],
	allEdges: { source: string; target: string; data?: any }[]
): Set<string> {
	const nodeIdPrefix = moduleId + '-'
	const nodeIds = new Set<string>()

	for (const n of allNodes) {
		if (n.id === moduleId || n.id.startsWith(nodeIdPrefix)) {
			nodeIds.add(n.id)
		}
	}

	for (const e of allEdges) {
		const disableIds: string[] = e.data?.disableMoveIds ?? []
		if (disableIds.includes(moduleId)) {
			nodeIds.add(e.source)
			nodeIds.add(e.target)
		}
	}

	// Include child nodes (e.g. asset/AI tool nodes) of nodes added via edges.
	// Nodes found through disableMoveIds (like inner module "b") may have children
	// ("b-asset-in-...") that weren't caught by the initial prefix match on moduleId.
	const edgeMatchedIds = [...nodeIds]
	for (const n of allNodes) {
		if (!nodeIds.has(n.id)) {
			for (const id of edgeMatchedIds) {
				if (n.id.startsWith(id + '-')) {
					nodeIds.add(n.id)
					break
				}
			}
		}
	}

	return nodeIds
}

export class MoveManager {
	dragging = $state<DragInfo | undefined>(undefined)
	ghostScreenX = $state(0)
	ghostScreenY = $state(0)
	nearestDropZone = $state<DropZone | undefined>(undefined)
	draggedNodeIds = $state<Set<string>>(new Set())

	/** The module ID currently being moved via legacy click-to-move */
	movingModuleId = $state<string | undefined>(undefined)

	/** Multiple module IDs being moved together (multi-select move) */
	movingIds = $state<string[] | undefined>(undefined)

	toggleMoving(id: string) {
		if (this.movingModuleId === id) {
			this.movingModuleId = undefined
			this.#updateDraggedNodeIds(undefined)
		} else {
			this.movingModuleId = id
			this.#updateDraggedNodeIds([id])
		}
	}

	toggleMovingMultiple(ids: string[]) {
		if (
			this.movingIds &&
			this.movingIds.length === ids.length &&
			this.movingIds.every((id, i) => id === ids[i])
		) {
			this.movingModuleId = undefined
			this.movingIds = undefined
			this.#updateDraggedNodeIds(undefined)
		} else {
			this.movingModuleId = ids[0]
			this.movingIds = ids
			this.#updateDraggedNodeIds(ids)
		}
	}

	setMoving(id: string) {
		this.movingModuleId = id
		this.#updateDraggedNodeIds([id])
	}

	clearMoving() {
		this.movingModuleId = undefined
		this.movingIds = undefined
		this.#updateDraggedNodeIds(undefined)
	}

	#computeDraggedNodeIds: ((moduleIds: string[]) => Set<string>) | undefined

	setComputeDraggedNodeIds(fn: (moduleIds: string[]) => Set<string>) {
		this.#computeDraggedNodeIds = fn
	}

	#updateDraggedNodeIds(moduleIds: string[] | undefined) {
		this.draggedNodeIds =
			moduleIds && moduleIds.length > 0 && this.#computeDraggedNodeIds
				? this.#computeDraggedNodeIds(moduleIds)
				: new Set()
	}

	#screenToFlowPosition: ((pos: { x: number; y: number }) => { x: number; y: number }) | undefined
	#registeredDropZones = new Map<string, DropZoneRegistration>()

	setScreenToFlowPosition(fn: (pos: { x: number; y: number }) => { x: number; y: number }) {
		this.#screenToFlowPosition = fn
	}

	registerDropZone(edgeId: string, zone: DropZoneRegistration) {
		this.#registeredDropZones.set(edgeId, zone)
	}

	unregisterDropZone(edgeId: string) {
		this.#registeredDropZones.delete(edgeId)
	}

	startDrag(moduleId: string, screenX: number, screenY: number, selectedIds?: string[]) {
		// Clear any active click-to-move so only drag mode is active
		this.movingModuleId = undefined
		this.dragging = { moduleId, selectedIds }
		this.ghostScreenX = screenX
		this.ghostScreenY = screenY
		this.nearestDropZone = undefined
		// Compute dragged node IDs for the primary module plus any additional selected modules
		const allIds =
			selectedIds && selectedIds.length > 0
				? [moduleId, ...selectedIds.filter((id) => id !== moduleId)]
				: [moduleId]
		this.#updateDraggedNodeIds(allIds)
	}

	updateDrag(screenX: number, screenY: number) {
		if (!this.dragging) return

		this.ghostScreenX = screenX
		this.ghostScreenY = screenY

		if (!this.#screenToFlowPosition) return

		const flowPos = this.#screenToFlowPosition({ x: screenX, y: screenY })
		this.nearestDropZone = this.#findNearestDropZone(flowPos)
	}

	endDrag(): DropZone | undefined {
		if (!this.dragging) return undefined
		const zone = this.nearestDropZone
		this.dragging = undefined
		this.nearestDropZone = undefined
		this.#updateDraggedNodeIds(undefined)
		return zone
	}

	cancelDrag() {
		this.dragging = undefined
		this.nearestDropZone = undefined
		this.#updateDraggedNodeIds(undefined)
	}

	#findNearestDropZone(flowPos: { x: number; y: number }): DropZone | undefined {
		if (!this.dragging) return undefined

		const draggedId = this.dragging.moduleId
		let best: DropZone | undefined = undefined
		let bestDist = Infinity

		// Box half-dimensions matching the visual drop zone
		const halfW = NODE.width / 2
		const halfH = NODE.gap.vertical / 2

		for (const [edgeId, zone] of this.#registeredDropZones) {
			if (zone.disableMoveIds.includes(draggedId)) continue

			// Skip edges adjacent to the dragged node (no-op move)
			if (zone.sourceId === draggedId || zone.targetId === draggedId) continue

			const dx = Math.abs(flowPos.x - zone.centerX)
			const dy = Math.abs(flowPos.y - zone.centerY)

			// Axis-aligned bounding box test
			if (dx <= halfW && dy <= halfH) {
				// Manhattan-like distance for ranking overlapping boxes
				const dist = dx + dy
				if (dist < bestDist) {
					bestDist = dist
					best = {
						edgeId,
						sourceId: zone.sourceId,
						targetId: zone.targetId,
						branch: zone.branch,
						index: zone.index,
						flowPosition: { x: zone.centerX, y: zone.centerY },
						disableMoveIds: zone.disableMoveIds
					}
				}
			}
		}

		return best
	}
}
