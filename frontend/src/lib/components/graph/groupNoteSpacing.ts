import type { FlowNote } from '../../gen'
import { deepEqual } from 'fast-equals'
import { clone } from '../../utils'

export type NodePosition = {
	id: string
	position: { x: number; y: number }
}

export type GroupNoteSpacingResult = {
	// Nodes need to be offset on the y axis to make space for group notes
	newNodePositions: Record<string, { x: number; y: number }>
}

export type TextHeightCache = {
	content: string
	height: number
}

// Cache for expensive group note spacing calculations
let computeGroupNoteSpacingCache:
	| [NodePosition[], FlowNote[], Record<string, number>, GroupNoteSpacingResult]
	| undefined

// Text height cache to avoid remeasuring unchanged note text
let textHeightCache: Record<string, TextHeightCache> = {}

const GROUP_NOTE_PADDING = 20

/**
 * Caches text height measurements based on content hash
 */
export function cacheTextHeight(noteId: string, content: string, height: number): void {
	textHeightCache[noteId] = { content, height }
}

/**
 * Gets cached text height if content hasn't changed, otherwise returns undefined
 */
export function getCachedTextHeight(noteId: string, content: string): number | undefined {
	const cached = textHeightCache[noteId]
	if (cached && cached.content === content) {
		return cached.height
	}
	return undefined
}

/**
 * Gets the text height for a note, using cache if available or fallback
 */
function getTextHeight(note: FlowNote, noteTextHeights: Record<string, number>): number {
	// Try cached height first
	const cachedHeight = getCachedTextHeight(note.id, note.text)
	if (cachedHeight !== undefined) {
		return cachedHeight
	}

	// Fall back to runtime heights
	const runtimeHeight = noteTextHeights[note.id]
	if (runtimeHeight !== undefined) {
		// Update cache for future use
		cacheTextHeight(note.id, note.text, runtimeHeight)
		return runtimeHeight
	}

	// Default fallback
	return 60
}

/**
 * Finds the topmost node in a group by Y position
 */
function findTopmostNodeInGroup(
	groupNote: FlowNote,
	nodes: NodePosition[]
): NodePosition | undefined {
	if (!groupNote.contained_node_ids?.length) {
		return undefined
	}

	const containedNodes = nodes.filter((node) => groupNote.contained_node_ids?.includes(node.id))

	if (containedNodes.length === 0) {
		return undefined
	}

	return containedNodes.reduce((topMost, node) =>
		node.position.y < topMost.position.y ? node : topMost
	)
}

/**
 * Computes vertical spacing adjustments for nodes to accommodate group notes.
 */
export function computeGroupNoteSpacing(
	nodes: NodePosition[],
	notes: FlowNote[],
	noteTextHeights: Record<string, number>
): GroupNoteSpacingResult {
	// Check cache first
	if (
		computeGroupNoteSpacingCache &&
		deepEqual(nodes, computeGroupNoteSpacingCache[0]) &&
		deepEqual(notes, computeGroupNoteSpacingCache[1]) &&
		deepEqual(noteTextHeights, computeGroupNoteSpacingCache[2])
	) {
		return computeGroupNoteSpacingCache[3]
	}

	// Filter to only group notes
	const groupNotes = notes.filter((note) => note.type === 'group')

	if (groupNotes.length === 0) {
		const result: GroupNoteSpacingResult = {
			newNodePositions: Object.fromEntries(nodes.map((n) => [n.id, n.position]))
		}
		computeGroupNoteSpacingCache = [clone(nodes), clone(notes), clone(noteTextHeights), result]
		return result
	}

	// Map Y positions to required spacing
	const ySpacingMap = new Map<number, number>()

	// For each group note, determine the spacing needed at the topmost node's Y position
	for (const groupNote of groupNotes) {
		const topmostNode = findTopmostNodeInGroup(groupNote, nodes)

		if (topmostNode) {
			const textHeight = getTextHeight(groupNote, noteTextHeights)
			const requiredSpacing = textHeight + GROUP_NOTE_PADDING

			// If multiple group notes affect the same Y position, take the maximum spacing needed
			const currentSpacing = ySpacingMap.get(topmostNode.position.y) || 0
			ySpacingMap.set(topmostNode.position.y, Math.max(currentSpacing, requiredSpacing))
		}
	}

	// Sort nodes by Y position to apply cumulative spacing
	const sortedNodes = [...nodes].sort((a, b) => a.position.y - b.position.y)

	// Apply cumulative spacing
	let cumulativeOffset = 0
	let lastYPosition = -Infinity
	const adjustedNodes = sortedNodes.map((node) => {
		// When we encounter a new Y position, check if it needs additional spacing
		if (node.position.y > lastYPosition) {
			const spacingAtThisY = ySpacingMap.get(node.position.y)
			if (spacingAtThisY !== undefined) {
				cumulativeOffset += spacingAtThisY
			}
			lastYPosition = node.position.y
		}

		return {
			...node,
			position: {
				...node.position,
				y: node.position.y + cumulativeOffset
			}
		}
	})

	const result: GroupNoteSpacingResult = {
		newNodePositions: Object.fromEntries(adjustedNodes.map((n) => [n.id, n.position]))
	}

	// Cache the result
	computeGroupNoteSpacingCache = [clone(nodes), clone(notes), clone(noteTextHeights), result]
	return result
}

/**
 * Clears the text height cache (useful for testing or when needed)
 */
export function clearTextHeightCache(): void {
	textHeightCache = {}
}

/**
 * Clears the group note spacing cache (useful when nodes structure changes dramatically)
 */
export function clearGroupNoteSpacingCache(): void {
	computeGroupNoteSpacingCache = undefined
}
