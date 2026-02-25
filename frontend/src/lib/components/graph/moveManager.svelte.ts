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

	return nodeIds
}

import { NODE } from './util'

export class MoveManager {
	dragging = $state<DragInfo | undefined>(undefined)
	ghostScreenX = $state(0)
	ghostScreenY = $state(0)
	nearestDropZone = $state<DropZone | undefined>(undefined)
	draggedNodeIds = $state<Set<string>>(new Set())

	/** The module ID currently being moved via legacy click-to-move */
	movingModuleId = $state<string | undefined>(undefined)

	setMoving(id: string) {
		if (this.movingModuleId === id) {
			this.movingModuleId = undefined
			this.draggedNodeIds = new Set()
		} else {
			this.movingModuleId = id
		}
	}

	/** Non-toggle setter — always sets movingModuleId (used by drag coordinator) */
	forceSetMoving(id: string) {
		this.movingModuleId = id
	}

	clearMoving() {
		this.movingModuleId = undefined
		this.draggedNodeIds = new Set()
	}

	setDraggedNodeIds(ids: Set<string>) {
		this.draggedNodeIds = ids
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

	startDrag(moduleId: string, screenX: number, screenY: number) {
		// Clear any active click-to-move so only drag mode is active
		this.movingModuleId = undefined
		this.dragging = { moduleId }
		this.ghostScreenX = screenX
		this.ghostScreenY = screenY
		this.nearestDropZone = undefined
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
		this.draggedNodeIds = new Set()
		return zone
	}

	cancelDrag() {
		this.dragging = undefined
		this.nearestDropZone = undefined
		this.draggedNodeIds = new Set()
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
