import type { FlowNote } from '$lib/gen'
import type { Node } from '@xyflow/svelte'
import { calculateNodesBounds } from './util'

export type NodePosition = {
	id: string
	position: { x: number; y: number }
}

export type TextHeightCacheEntry = {
	content: string
	height: number
}

/**
 * Utility class for managing flow notes including regular and group notes
 * This is now a stateless utility that operates on passed note data
 */
export class NoteManager {
	#cache: Record<string, TextHeightCacheEntry> = $state({})
	renderCount = $state(0)

	constructor() {}

	/**
	 * Triggers a re-render of the graph by incrementing the render count
	 */
	render(): void {
		this.renderCount++
	}

	getCache(): Record<string, TextHeightCacheEntry> {
		return this.#cache
	}

	/**
	 * Calculate position and size for group notes based on contained nodes
	 */
	calculateGroupNoteLayout(
		note: FlowNote,
		nodes: Node[],
		textHeight: number = 60
	): { position: { x: number; y: number }; size: { width: number; height: number } } {
		if (note.type !== 'group' || !note.contained_node_ids?.length) {
			return { position: note.position, size: note.size }
		}

		const containedNodes = nodes.filter((node) => note.contained_node_ids?.includes(node.id))

		if (containedNodes.length === 0) {
			return { position: note.position, size: note.size }
		}

		// Find bounds of all contained nodes
		const bounds = calculateNodesBounds(containedNodes)

		const padding = 16

		return {
			position: {
				x: bounds.minX - padding,
				y: bounds.minY - textHeight - padding
			},
			size: {
				width: bounds.maxX - bounds.minX + 2 * padding,
				height: bounds.maxY - bounds.minY + textHeight + 2 * padding
			}
		}
	}

	/**
	 * Create common data object for note nodes (rendering-only version)
	 */
	private createNoteData(
		note: FlowNote,
		onTextHeightChange: (noteId: string, height: number) => void,
		isGroupNote: boolean
	) {
		return {
			noteId: note.id,
			text: note.text,
			color: note.color,
			locked: note.locked || false,
			isGroupNote,
			...(isGroupNote && { containedNodeIds: note.contained_node_ids || [] }),
			// Note: Edit callbacks will be added by NoteNode when NoteEditor context is available
			onTextHeightChange: (textHeight: number) => {
				onTextHeightChange(note.id, textHeight)
				// Cache the text height for improved performance
				this.cacheTextHeight(note.id, note.text, textHeight)
			}
		}
	}

	/**
	 * Convert notes to SvelteFlow nodes (rendering-only)
	 */
	convertToNodes(
		notes: FlowNote[],
		currentNodes: Node[],
		textHeights: Record<string, number>,
		onTextHeightChange: (noteId: string, height: number) => void
	): Node[] {
		return notes.map((note) => {
			const isGroupNote = note.type === 'group'

			// Calculate position and size based on note type
			const { position, size } = isGroupNote
				? this.calculateGroupNoteLayout(note, currentNodes, textHeights[note.id] || 60)
				: { position: note.position, size: note.size }

			return {
				id: note.id,
				type: 'note',
				position,
				data: this.createNoteData(note, onTextHeightChange, isGroupNote),
				style: `width: ${size.width}px; height: ${size.height}px;`,
				width: size.width,
				height: size.height,
				zIndex: -2000,
				draggable: isGroupNote ? false : !note.locked,
				selectable: true
			}
		})
	}

	/**
	 * Caches text height measurements based on content hash
	 */
	cacheTextHeight(noteId: string, content: string, height: number): void {
		this.#cache[noteId] = { content, height }
		this.render()
	}

	/**
	 * Gets cached text height if content hasn't changed, otherwise returns undefined
	 */
	getCachedTextHeight(noteId: string, content: string): number | undefined {
		const cached = this.#cache[noteId]
		if (cached && cached.content === content) {
			return cached.height
		}
		return undefined
	}

	/**
	 * Gets the text height for a note, using cache if available or fallback
	 */
	getTextHeight(
		noteId: string,
		content: string,
		runtimeHeights: Record<string, number>,
		defaultHeight: number = 60
	): number {
		// Try cached height first
		const cachedHeight = this.getCachedTextHeight(noteId, content)
		if (cachedHeight !== undefined) {
			return cachedHeight
		}

		// Fall back to runtime heights
		const runtimeHeight = runtimeHeights[noteId]
		if (runtimeHeight !== undefined) {
			// Update cache for future use
			this.cacheTextHeight(noteId, content, runtimeHeight)
			return runtimeHeight
		}

		// Default fallback
		return defaultHeight
	}

	/**
	 * Clears the entire text height cache (useful for testing or when needed)
	 */
	clearTextHeightCache(): void {
		this.#cache = {}
		this.render()
	}

	/**
	 * Removes a specific note from the cache
	 */
	removeTextHeight(noteId: string): void {
		delete this.#cache[noteId]
		this.render()
	}
}
