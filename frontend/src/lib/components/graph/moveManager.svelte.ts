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
	label: string
	startScreenX: number
	startScreenY: number
	isSubflow: boolean
}

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
		} else {
			this.movingModuleId = id
		}
	}

	clearMoving() {
		this.movingModuleId = undefined
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

	startDrag(moduleId: string, label: string, screenX: number, screenY: number, isSubflow = false) {
		this.dragging = { moduleId, label, startScreenX: screenX, startScreenY: screenY, isSubflow }
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
		const halfW = 275 / 2 // NODE.width / 2
		const halfH = 62 / 2 // NODE.gap.vertical / 2

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
