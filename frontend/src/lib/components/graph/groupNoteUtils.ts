import type { Node } from '@xyflow/svelte'
import type { FlowNote } from '$lib/gen'
import { NODE } from './util'

export interface GroupNoteBounds {
	x: number
	y: number
	width: number
	height: number
}

// Extended types for group notes
export interface RegularNote extends FlowNote {
	type: 'regular'
}

export interface GroupNote extends Omit<FlowNote, 'position' | 'size'> {
	type: 'group'
	containedNodeIds: string[]
	// position and size will be calculated dynamically
}

export type ExtendedFlowNote = RegularNote | GroupNote

/**
 * Type guard to check if a note is a group note
 */
export function isGroupNote(note: ExtendedFlowNote): note is GroupNote {
	return note.type === 'group'
}

/**
 * Type guard to check if a note is a regular note
 */
export function isRegularNote(note: ExtendedFlowNote): note is RegularNote {
	return note.type === 'regular'
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
 * Calculates the dynamic bounds for a group note
 */
export function calculateGroupNoteBounds(
	groupNote: GroupNote,
	nodes: Node[],
	textHeight: number = 60
): { position: { x: number; y: number }; size: { width: number; height: number } } {
	const bounds = computeGroupNoteBounds(groupNote.containedNodeIds, nodes, textHeight)

	return {
		position: {
			x: bounds.x,
			y: bounds.y
		},
		size: {
			width: bounds.width,
			height: bounds.height
		}
	}
}

/**
 * Creates a new group note with contained node IDs instead of fixed bounds
 */
export function createGroupNote(
	selectedNodeIds: string[],
	color: string = 'yellow'
): GroupNote {
	if (selectedNodeIds.length === 0) {
		throw new Error('No nodes selected for group note')
	}

	return {
		id: `group-note-${Date.now()}`,
		type: 'group',
		text: `Group note for ${selectedNodeIds.length} nodes`,
		containedNodeIds: [...selectedNodeIds], // Copy array to avoid mutation
		color
	}
}

/**
 * Adds a node to an existing group note
 */
export function addNodeToGroup(groupNote: GroupNote, nodeId: string): GroupNote {
	if (groupNote.containedNodeIds.includes(nodeId)) {
		return groupNote // Node already in group
	}

	return {
		...groupNote,
		containedNodeIds: [...groupNote.containedNodeIds, nodeId],
		text: `Group note for ${groupNote.containedNodeIds.length + 1} nodes`
	}
}

/**
 * Removes a node from an existing group note
 */
export function removeNodeFromGroup(groupNote: GroupNote, nodeId: string): GroupNote | null {
	const updatedNodeIds = groupNote.containedNodeIds.filter(id => id !== nodeId)

	if (updatedNodeIds.length === 0) {
		return null // Group note should be deleted if no nodes remain
	}

	return {
		...groupNote,
		containedNodeIds: updatedNodeIds,
		text: `Group note for ${updatedNodeIds.length} nodes`
	}
}

/**
 * Validates that all contained nodes exist in the provided nodes array
 */
export function validateGroupNote(groupNote: GroupNote, nodes: Node[]): GroupNote {
	const validNodeIds = nodes.map(node => node.id)
	const validContainedNodeIds = groupNote.containedNodeIds.filter(id => validNodeIds.includes(id))

	return {
		...groupNote,
		containedNodeIds: validContainedNodeIds,
		text: `Group note for ${validContainedNodeIds.length} nodes`
	}
}

/**
 * Converts a legacy FlowNote to the new ExtendedFlowNote format
 */
export function convertToExtendedNote(note: FlowNote & { isGroupNote?: boolean; containedNodeIds?: string[] }): ExtendedFlowNote {
	if (note.isGroupNote && note.containedNodeIds) {
		// Convert legacy group note
		return {
			id: note.id,
			type: 'group',
			text: note.text,
			color: note.color,
			containedNodeIds: note.containedNodeIds
		}
	} else {
		// Convert regular note
		return {
			id: note.id,
			type: 'regular',
			text: note.text,
			position: note.position,
			size: note.size,
			color: note.color
		}
	}
}