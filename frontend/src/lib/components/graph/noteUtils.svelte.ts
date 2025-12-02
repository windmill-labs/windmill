import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import { deepEqual } from 'fast-equals'
import { calculateNodesBoundsWithOffset } from './util'
import { MIN_NOTE_WIDTH, MIN_NOTE_HEIGHT } from './noteColors'
import type { NodeLayout } from './graphBuilder.svelte'
import { topologicalSort } from './graphBuilder.svelte'
import type { AssetWithAltAccessType } from '../assets/lib'
import type { NoteEditorContext } from './noteEditor.svelte'
import { StickyNote } from 'lucide-svelte'

export type NodeDep = {
	id: string
	position: { x: number; y: number }
	data?: { assets?: AssetWithAltAccessType[] }
	parentIds?: string[]
	offset?: number
	type?: string
}

export type NoteComputeResult = {
	noteNodes: (Node & NodeLayout)[]
	newNodePositions: Record<string, { x: number; y: number }>
}

export type AIToolSpacingInfo = {
	toolNodes: (Node & NodeLayout)[]
	toolEdges: any[]
	newNodePositions: Record<string, { x: number; y: number }>
}

export interface GroupNoteBounds {
	x: number
	y: number
	width: number
	height: number
}

let computeNoteNodesCache:
	| [NodeDep[], FlowNote[], Record<string, number>, NoteComputeResult]
	| undefined

/**
 * Extracts layout-affecting signature for change detection
 * Only includes properties that affect graph layout (structure, grouping)
 */
export function getLayoutSignature(notes: FlowNote[]) {
	return {
		notesCount: notes.length,
		noteIds: notes.map((n) => n.id).sort(),
		// Group memberships affect layout spacing
		groupMemberships: notes
			.filter((note) => note.type === 'group')
			.map((note) => ({
				id: note.id,
				containedIds: note.contained_node_ids?.slice().sort() || []
			}))
			.sort((a, b) => a.id.localeCompare(b.id))
	}
}

/**
 * Extracts property-only signature for change detection
 * Only includes visual/content properties that don't affect layout
 */
export function getPropertySignature(notes: FlowNote[]) {
	return notes
		.map((note) => ({
			id: note.id,
			text: note.text,
			color: note.color,
			locked: note.locked || false,
			position: { ...note.position },
			size: { ...note.size }
		}))
		.sort((a, b) => a.id.localeCompare(b.id))
}

/**
 * Calculates z-index values for all notes
 * Group notes are ordered by their topmost node's hierarchy position
 * Free notes get undefined z-index to use SvelteFlow's native behavior
 */
export function calculateAllNoteZIndexes(
	notes: FlowNote[],
	nodes: NodeDep[]
): Record<string, number | undefined> {
	const zIndexMap: Record<string, number | undefined> = {}

	// Use topological sort to get proper hierarchy order based on parentIds relationships
	const sortedNodes = topologicalSort(nodes).reverse()

	// Create a mapping from node ID to its hierarchy position (topological order)
	const nodeHierarchyMap: Record<string, number> = {}
	sortedNodes.forEach((node, index) => {
		nodeHierarchyMap[node.id] = index
	})

	// Process each note
	for (const note of notes) {
		if (note.type === 'free') {
			// Free notes use SvelteFlow's native z-index behavior (last selected on top)
			zIndexMap[note.id] = undefined
		} else if (note.type === 'group') {
			// Group notes get z-index based on topmost contained node's hierarchy
			// Since sortedNodes is in topological order, the first matching node is the topmost
			const topmostNode = sortedNodes.find((node) => note.contained_node_ids?.includes(node.id))

			if (topmostNode) {
				const hierarchyPosition = nodeHierarchyMap[topmostNode.id] ?? 0
				// Higher hierarchy position = lower z-index (appears behind)
				// Use negative values starting from -2000 to stay below other elements
				zIndexMap[note.id] = hierarchyPosition - 2000
			} else {
				// Fallback for group notes without valid contained nodes
				zIndexMap[note.id] = -2000
			}
		}
	}

	return zIndexMap
}

/**
 * Calculate extra spacing needed for asset nodes of the topmost node
 */
function calculateExtraAssetSpacing(topmostNodeId: string, nodes: NodeDep[]): number {
	// Find the topmost node position
	const topmostNode = nodes.find((n) => n.id === topmostNodeId)
	if (!topmostNode) {
		return 0
	}

	// Find actual asset nodes for the topmost node: {topmostNodeId}-asset-in, type 'asset'
	const assetNodes = nodes.filter((n) => n.id.startsWith(`${topmostNodeId}-asset-in-`))

	if (assetNodes.length === 0) {
		return 0
	}

	// Calculate the spacing based on actual asset node positions
	const assetSpacing = Math.max(
		...assetNodes.map((assetNode) => {
			// Calculate how much space the asset node takes above the main node
			return Math.max(0, -assetNode.position.y)
		})
	)

	return assetSpacing
}

/**
 * Calculate extra spacing needed for AI tool nodes of the topmost node
 */
function calculateExtraAIToolSpacing(topmostNodeId: string, nodes: NodeDep[]): number {
	// Find the topmost node position
	const topmostNode = nodes.find((n) => n.id === topmostNodeId)
	if (!topmostNode) {
		return 0
	}

	// Find actual AI tool nodes for the topmost node: {topmostNodeId}-tool-, type 'aiTool'
	const toolNodes = nodes.filter((n) => n.id.startsWith(`${topmostNodeId}-tool-`))

	if (toolNodes.length === 0) {
		return 0
	}

	// Calculate the spacing based on actual AI tool node positions
	const toolSpacing = Math.max(
		...toolNodes.map((toolNode) => {
			// Calculate how much space the tool node takes above/below the main node
			return Math.max(0, -toolNode.position.y)
		})
	)

	return toolSpacing
}

/**
 * Calculate position and size for group notes based on contained nodes
 */
function calculateGroupNoteLayout(
	note: FlowNote,
	nodes: NodeDep[],
	textHeight: number = 60,
	topMostNodeId: string
): { position: { x: number; y: number }; size: { width: number; height: number } } {
	if (note.type !== 'group' || !note.contained_node_ids?.length) {
		return {
			position: note.position ?? { x: 0, y: 0 },
			size: note.size ?? { width: MIN_NOTE_WIDTH, height: MIN_NOTE_HEIGHT }
		}
	}

	const containedNodes = nodes.filter((node) => note.contained_node_ids?.includes(node.id))

	if (containedNodes.length === 0) {
		return {
			position: note.position ?? { x: 0, y: 0 },
			size: note.size ?? { width: MIN_NOTE_WIDTH, height: MIN_NOTE_HEIGHT }
		}
	}

	const bounds = calculateNodesBoundsWithOffset(
		note.contained_node_ids || [],
		nodes.map((n) => ({
			id: n.id,
			position: n.position,
			data: { offset: n.offset ?? 0 },
			type: n.type ?? ''
		}))
	)

	const padding = 16

	// Calculate extra spacing for asset nodes and AI tool nodes of the topmost node
	const extraAssetSpacing = topMostNodeId ? calculateExtraAssetSpacing(topMostNodeId, nodes) : 0

	const extraAIToolSpacing = topMostNodeId ? calculateExtraAIToolSpacing(topMostNodeId, nodes) : 0

	const totalTextHeight = textHeight + extraAssetSpacing + extraAIToolSpacing

	return {
		position: {
			x: bounds.minX - padding,
			y: bounds.minY - totalTextHeight - padding
		},
		size: {
			width: bounds.maxX - bounds.minX + 2 * padding,
			height: bounds.maxY - bounds.minY + totalTextHeight + 2 * padding
		}
	}
}

/**
 * Create common data object for note nodes
 */
function createNoteData(
	note: FlowNote,
	onTextHeightChange: (noteId: string, height: number) => void,
	isGroupNote: boolean,
	editMode: boolean
) {
	return {
		noteId: note.id,
		text: note.text,
		color: note.color,
		locked: note.locked || false,
		isGroupNote,
		editMode,
		...(isGroupNote && { containedNodeIds: note.contained_node_ids || [] }),
		onTextHeightChange: (textHeight: number) => {
			onTextHeightChange(note.id, textHeight)
		}
	}
}

/**
 * Main function to compute note nodes and adjust nodes position based on group notes
 */
export function computeNoteNodes(
	nodes: NodeDep[],
	notes: FlowNote[],
	noteTextHeights: Record<string, number>,
	onTextHeightChange: (noteId: string, height: number) => void,
	editMode: boolean = false,
	noteEditorContext: NoteEditorContext | undefined
): NoteComputeResult {
	// Check cache first
	if (
		computeNoteNodesCache &&
		deepEqual(nodes, computeNoteNodesCache[0]) &&
		deepEqual(notes, computeNoteNodesCache[1]) &&
		deepEqual(noteTextHeights, computeNoteNodesCache[2])
	) {
		return computeNoteNodesCache[3]
	}

	if (editMode) {
		if (noteEditorContext?.noteEditor?.isAvailable()) {
			noteEditorContext.noteEditor.cleanupGroupNotes(nodes)
		}
	}

	const allNoteNodes: (Node & NodeLayout)[] = []

	// Build a map of Y positions that need extra spacing for group notes
	const yPosMap: Record<number, number> = {} // Y position -> spacing needed

	// Group notes that need spacing
	const groupNotes = notes.filter((n) => n.type === 'group')

	const topMostNodesMap: Record<string, string> = {}

	const sortedNodes = topologicalSort(nodes).reverse()

	for (const groupNote of groupNotes) {
		if (groupNote.contained_node_ids?.length) {
			const topmostNodeId = sortedNodes.find((node) =>
				groupNote.contained_node_ids?.includes(node.id)
			)?.id
			const topmostNode = nodes.find((node) => node.id === topmostNodeId)
			if (topmostNode) {
				const textHeight = noteTextHeights[groupNote.id] || 60
				const spacing = textHeight + 16 // padding
				// Mark this Y position as needing spacing
				yPosMap[topmostNode.position.y] = Math.max(yPosMap[topmostNode.position.y] || 0, spacing)
				topMostNodesMap[groupNote.id] = topmostNode.id
			}
		}
	}

	// Calculate new positions for nodes (offset by group notes)
	const sortedNewNodes = nodes
		.map((n) => ({ position: { ...n.position }, id: n.id }))
		.sort((a, b) => a.position.y - b.position.y)

	let currentYOffset = 0
	let prevYPos = NaN

	for (const node of sortedNewNodes) {
		if (node.position.y !== prevYPos) {
			// Add spacing for group notes at this Y level
			if (yPosMap[node.position.y]) {
				currentYOffset += yPosMap[node.position.y]
			}
			prevYPos = node.position.y
		}
		node.position.y += currentYOffset
	}

	// Create note nodes AFTER calculating adjusted node positions
	// For group notes, we need to use the adjusted node positions
	const adjustedNodes = sortedNewNodes.map((n) => {
		const origNode = nodes.find((orig) => orig.id === n.id)
		return {
			...n,
			data: origNode?.data,
			offset: origNode?.offset,
			type: origNode?.type
		}
	})

	// Calculate all z-indexes at once using hierarchy information
	const noteZIndexes = calculateAllNoteZIndexes(notes, nodes)

	for (const note of notes) {
		const isGroupNote = note.type === 'group'
		const zIndex = noteZIndexes[note.id]

		// Calculate position and size using adjusted node positions for group notes
		const { position, size } = isGroupNote
			? calculateGroupNoteLayout(
					note,
					adjustedNodes,
					noteTextHeights[note.id] || 60,
					topMostNodesMap[note.id]
				)
			: {
					position: note.position ?? { x: 0, y: 0 },
					size: note.size ?? { width: MIN_NOTE_WIDTH, height: MIN_NOTE_HEIGHT }
				}

		// Create the note node
		const noteNode: Node & NodeLayout = {
			id: note.id,
			type: 'note' as any, // Note nodes are handled specially
			position,
			width: size.width,
			height: size.height,
			zIndex,
			draggable: isGroupNote ? false : editMode && !note.locked,
			selectable: false,
			data: createNoteData(note, onTextHeightChange, isGroupNote, editMode) as any
		}

		allNoteNodes.push(noteNode)
	}

	const newNodePositions: Record<string, { x: number; y: number }> = Object.fromEntries(
		sortedNewNodes.map((n) => [n.id, n.position])
	)

	const result: NoteComputeResult = {
		noteNodes: allNoteNodes,
		newNodePositions
	}

	// Cache the result
	computeNoteNodesCache = [
		structuredClone($state.snapshot(nodes)),
		structuredClone($state.snapshot(notes)),
		structuredClone($state.snapshot(noteTextHeights)),
		result
	]

	return result
}

export function addGroupNoteContextMenuItem(
	nodeId: string,
	noteEditorContext: NoteEditorContext | undefined
) {
	const isDisabled =
		!noteEditorContext?.noteEditor ||
		(noteEditorContext?.noteEditor?.isNodeOnlyMemberOfGroupNote(nodeId) ?? false)

	return {
		id: 'add-group-note',
		label: 'Add note',
		icon: StickyNote,
		disabled: isDisabled,
		onClick: () => {
			if (noteEditorContext?.noteEditor && !isDisabled) {
				noteEditorContext.noteEditor.createGroupNote([nodeId])
			}
		}
	}
}
