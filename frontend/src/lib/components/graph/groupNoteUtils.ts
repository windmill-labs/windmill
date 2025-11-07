import type { Node } from '@xyflow/svelte'
import type { FlowNote } from '$lib/gen'
import { NODE } from './util'

export interface GroupNoteBounds {
	x: number
	y: number
	width: number
	height: number
}

/**
 * Computes the bounding box that wraps all selected nodes with padding
 * Uses the same calculation logic as SelectionBoundingBox for consistency
 */
export function computeGroupNoteBounds(
	selectedNodeIds: string[],
	nodes: Node[]
): GroupNoteBounds {
	const selectedNodes = nodes.filter(node => selectedNodeIds.includes(node.id))

	if (selectedNodes.length === 0) {
		throw new Error('No nodes selected for group note')
	}

	// Calculate flow coordinates bounds using same logic as SelectionBoundingBox
	let minX = Infinity
	let maxX = -Infinity
	let minY = Infinity
	let maxY = -Infinity

	selectedNodes.forEach(node => {
		minX = Math.min(minX, node.position.x)
		maxX = Math.max(maxX, node.position.x + NODE.width)
		minY = Math.min(minY, node.position.y)
		maxY = Math.max(maxY, node.position.y + NODE.height)
	})

	// Add padding in flow coordinates (same as SelectionBoundingBox)
	const padding = 10

	return {
		x: minX - padding,
		y: minY - padding,
		width: maxX - minX + (padding * 2),
		height: maxY - minY + (padding * 2)
	}
}

/**
 * Creates a new group note with computed position and size
 */
export function createGroupNote(
	selectedNodeIds: string[],
	nodes: Node[],
	color: string = 'yellow'
): FlowNote {
	const bounds = computeGroupNoteBounds(selectedNodeIds, nodes)

	return {
		id: `group-note-${Date.now()}`,
		text: `Group note for ${selectedNodeIds.length} nodes`,
		position: {
			x: bounds.x,
			y: bounds.y
		},
		size: {
			width: bounds.width,
			height: bounds.height
		},
		color
	}
}