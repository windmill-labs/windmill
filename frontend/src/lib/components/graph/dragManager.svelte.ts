import type { Node, Edge } from '@xyflow/svelte'

export type DropZone = {
	edgeId: string
	sourceId: string
	targetId: string
	branch: { rootId: string; branch: number } | undefined
	index: number
	flowPosition: { x: number; y: number }
	disableMoveIds: string[]
}

type DragInfo = {
	moduleId: string
	label: string
	startScreenX: number
	startScreenY: number
}

const DROP_THRESHOLD_PX = 80

export class DragManager {
	dragging = $state<DragInfo | undefined>(undefined)
	ghostScreenX = $state(0)
	ghostScreenY = $state(0)
	nearestDropZone = $state<DropZone | undefined>(undefined)

	#screenToFlowPosition: ((pos: { x: number; y: number }) => { x: number; y: number }) | undefined

	setScreenToFlowPosition(fn: (pos: { x: number; y: number }) => { x: number; y: number }) {
		this.#screenToFlowPosition = fn
	}

	startDrag(moduleId: string, label: string, screenX: number, screenY: number) {
		this.dragging = { moduleId, label, startScreenX: screenX, startScreenY: screenY }
		this.ghostScreenX = screenX
		this.ghostScreenY = screenY
		this.nearestDropZone = undefined
	}

	updateDrag(screenX: number, screenY: number, edges: Edge[], nodes: Node[]) {
		if (!this.dragging) return

		this.ghostScreenX = screenX
		this.ghostScreenY = screenY

		if (!this.#screenToFlowPosition) return

		const flowPos = this.#screenToFlowPosition({ x: screenX, y: screenY })
		this.nearestDropZone = this.#findNearestDropZone(flowPos, edges, nodes)
	}

	endDrag(): DropZone | undefined {
		if (!this.dragging) return undefined
		const zone = this.nearestDropZone
		this.dragging = undefined
		this.nearestDropZone = undefined
		return zone
	}

	cancelDrag() {
		this.dragging = undefined
		this.nearestDropZone = undefined
	}

	#findNearestDropZone(
		flowPos: { x: number; y: number },
		edges: Edge[],
		nodes: Node[]
	): DropZone | undefined {
		if (!this.dragging) return undefined

		const draggedId = this.dragging.moduleId
		let best: DropZone | undefined = undefined
		let bestDist = DROP_THRESHOLD_PX

		for (const edge of edges) {
			if (edge.type !== 'edge') continue

			const data = edge.data as any
			if (!data?.insertable) continue

			const disableMoveIds: string[] = data.disableMoveIds ?? []
			if (disableMoveIds.includes(draggedId)) continue

			// Skip edges adjacent to the dragged node (no-op move)
			if (data.sourceId === draggedId || data.targetId === draggedId) continue

			const sourceNode = nodes.find((n) => n.id === data.sourceId)
			if (!sourceNode) continue

			// The insert button is placed at (sourceX, sourceY + 32) in the EdgeLabel.
			// sourceX/sourceY are the edge source endpoint = bottom center of source node.
			// Nodes already have yOffset applied, so node.position.y is the final y.
			// NODE height = 34, so bottom of node = position.y + 34.
			// EdgeLabel offset = +32 from sourceY.
			const insertX = sourceNode.position.x + 275 / 2
			const insertY = sourceNode.position.y + 34 + 32

			const dx = flowPos.x - insertX
			const dy = flowPos.y - insertY
			const dist = Math.sqrt(dx * dx + dy * dy)

			if (dist < bestDist) {
				bestDist = dist
				best = {
					edgeId: edge.id,
					sourceId: data.sourceId,
					targetId: data.targetId,
					branch: data.branch,
					index: data.index,
					flowPosition: { x: insertX, y: insertY },
					disableMoveIds
				}
			}
		}

		return best
	}
}
