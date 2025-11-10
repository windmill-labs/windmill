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
 * Computes the bounding box that wraps all contained nodes with padding
 * Uses the same calculation logic as SelectionBoundingBox for consistency
 */
export function computeGroupNoteBounds(
	containedNodeIds: string[],
	nodes: Node[],
	textHeight: number = 60
): GroupNoteBounds {
	const containedNodes = nodes.filter(node => containedNodeIds.includes(node.id))

	if (containedNodes.length === 0) {
		throw new Error('No nodes contained in group note')
	}

	// Calculate flow coordinates bounds using same logic as SelectionBoundingBox
	let minX = Infinity
	let maxX = -Infinity
	let minY = Infinity
	let maxY = -Infinity

	containedNodes.forEach(node => {
		minX = Math.min(minX, node.position.x)
		maxX = Math.max(maxX, node.position.x + NODE.width)
		minY = Math.min(minY, node.position.y)
		maxY = Math.max(maxY, node.position.y + NODE.height)
	})

	// Add padding in flow coordinates (same as SelectionBoundingBox)
	const padding = 10

	return {
		x: minX - padding,
		y: minY - padding - textHeight, // Position text above the nodes
		width: maxX - minX + (padding * 2),
		height: maxY - minY + (padding * 2) + textHeight
	}
}

/**
 * Gets the topmost node from a list of contained nodes
 */
export function getTopMostNode(containedNodeIds: string[], nodes: Node[]): Node | null {
	const containedNodes = nodes.filter(node => containedNodeIds.includes(node.id))

	if (containedNodes.length === 0) {
		return null
	}

	return containedNodes.reduce((topMost, node) =>
		node.position.y < topMost.position.y ? node : topMost
	)
}

/**
 * Validates that all contained nodes exist in the provided nodes array
 */
export function validateGroupNote(note: FlowNote, nodes: Node[]): FlowNote {
	if (note.type !== 'group' || !note.contained_node_ids) {
		return note
	}

	const validNodeIds = nodes.map(node => node.id)
	const validContainedNodeIds = note.contained_node_ids.filter(id => validNodeIds.includes(id))

	return {
		...note,
		contained_node_ids: validContainedNodeIds
	}
}