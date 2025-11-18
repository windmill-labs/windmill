import type { FlowNote } from '../../gen'
import type { NoteManager } from './noteManager.svelte'
import type { NoteEditorContext } from './noteEditor.svelte'
import { StickyNote } from 'lucide-svelte'
import type { AssetWithAltAccessType } from '../assets/lib'

type NodeDep = {
	id: string
	parentIds?: string[]
	offset?: number
	data?: { assets?: AssetWithAltAccessType[] }
}

export const GROUP_NOTE_PADDING = 20
export const INPUT_ASSET_ROW_HEIGHT = 45 // Height needed for input asset row above node

export interface GroupNoteBounds {
	x: number
	y: number
	width: number
	height: number
}

/**
 * Finds the topmost node in a group based on topological ordering
 * Uses parent-child relationships to determine hierarchy
 */
export function findTopmostNodeInGroup(groupNote: FlowNote, nodes: NodeDep[]): NodeDep | undefined {
	if (!groupNote.contained_node_ids?.length) {
		return undefined
	}

	const containedNodes = nodes.filter((node) => groupNote.contained_node_ids?.includes(node.id))

	if (containedNodes.length === 0) {
		return undefined
	}

	// Find the node with the fewest parents (or no parents) - this will be topmost in the flow
	const nodesByParentCount = containedNodes.map((node) => ({
		node,
		parentCount: node.parentIds?.length || 0
	}))

	// Sort by parent count, then by appearance in nodes array to ensure deterministic results
	nodesByParentCount.sort((a, b) => {
		if (a.parentCount !== b.parentCount) {
			return a.parentCount - b.parentCount
		}
		// If same parent count, use original node order as tiebreaker
		const aIndex = nodes.findIndex((n) => n.id === a.node.id)
		const bIndex = nodes.findIndex((n) => n.id === b.node.id)
		return aIndex - bIndex
	})

	return nodesByParentCount[0]?.node
}

/**
 * Gets the extra spacing needed for a specific node due to group notes
 * Returns 0 if the node doesn't need extra spacing
 */
export function getNodeGroupNoteSpacing(
	nodeId: string,
	groupNotes: FlowNote[],
	nodes: NodeDep[],
	noteTextHeights: Record<string, number>,
	noteManager: NoteManager
): number {
	for (const groupNote of groupNotes) {
		if (groupNote.contained_node_ids?.includes(nodeId)) {
			const topmostNode = findTopmostNodeInGroup(groupNote, nodes)

			// Only the topmost node gets the spacing
			if (topmostNode?.id === nodeId) {
				const textHeight = noteManager.getTextHeight(
					groupNote.id,
					groupNote.text,
					noteTextHeights,
					60
				)

				return textHeight + GROUP_NOTE_PADDING
			}
		}
	}
	return 0
}

/**
 * Builds a map of node IDs to their required extra spacing for group notes
 * This is called during the layout process to integrate with D3's nodeSize function
 */
export function buildNodeSpacingMap(
	nodes: NodeDep[],
	groupNotes: FlowNote[],
	noteTextHeights: Record<string, number>,
	noteManager: NoteManager
): Record<string, number> {
	const spacingMap: Record<string, number> = {}

	// Only process if we have group notes
	if (groupNotes.length === 0) {
		return spacingMap
	}

	// For each node, calculate if it needs extra spacing
	for (const node of nodes) {
		const extraSpacing = getNodeGroupNoteSpacing(
			node.id,
			groupNotes,
			nodes,
			noteTextHeights,
			noteManager
		)
		if (extraSpacing > 0) {
			spacingMap[node.id] = extraSpacing
		}
	}

	return spacingMap
}

/**
 * Creates a stable hash of the noteTextHeights object for cache comparison
 */
export function hashNoteTextHeights(noteTextHeights: Record<string, number>): string {
	const entries = Object.entries(noteTextHeights).sort(([a], [b]) => a.localeCompare(b)) // Sort for stable hash

	return JSON.stringify(entries)
}

/**
 * Extracts note state signature for layout cache comparison
 */
export function getNoteStateSignature(
	groupNotes: FlowNote[],
	noteTextHeights: Record<string, number>
) {
	return {
		notesCount: groupNotes.length,
		noteIds: groupNotes.map((n) => n.id).sort(),
		textHeightHash: hashNoteTextHeights(noteTextHeights)
	}
}

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
 * Calculates z-index values for all notes in a single graph traversal
 * Group notes are ordered by their topmost node's hierarchy position
 * Free notes get undefined z-index to use SvelteFlow's native behavior
 */
export function calculateAllNoteZIndexes(
	notes: FlowNote[],
	nodes: NodeDep[]
): Record<string, number | undefined> {
	const zIndexMap: Record<string, number | undefined> = {}

	// Create a mapping from node ID to its hierarchy position (topological order)
	const nodeHierarchyMap: Record<string, number> = {}
	nodes.forEach((node, index) => {
		nodeHierarchyMap[node.id] = index
	})

	// Process each note
	for (const note of notes) {
		if (note.type === 'free') {
			// Free notes use SvelteFlow's native z-index behavior (last selected on top)
			zIndexMap[note.id] = undefined
		} else if (note.type === 'group') {
			// Group notes get z-index based on topmost contained node's hierarchy
			const topmostNode = findTopmostNodeInGroup(note, nodes)
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

export function addGroupNoteContextMenuItem(
	nodeId: string,
	noteEditorContext: NoteEditorContext | undefined
) {
	return {
		id: 'add-group-note',
		label: 'Add note',
		icon: StickyNote,
		disabled: !noteEditorContext?.noteEditor,
		onClick: () => {
			if (noteEditorContext?.noteEditor) {
				noteEditorContext.noteEditor.createGroupNote([nodeId])
			}
		}
	}
}
