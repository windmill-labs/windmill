<script lang="ts">
	import { useSvelteFlow, type XYPosition } from '@xyflow/svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { DEFAULT_NOTE_COLOR, MIN_NOTE_WIDTH, MIN_NOTE_HEIGHT } from './noteColors'
	import { StickyNote } from 'lucide-svelte'
	import ContextMenu, { type ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'

	interface Props {
		exitNoteMode?: () => void
		yOffset: number
	}

	let { exitNoteMode, yOffset }: Props = $props()

	// Get NoteEditor context for direct note creation
	const noteEditorContext = getNoteEditorContext()

	const { screenToFlowPosition, getViewport } = useSvelteFlow()

	let isDrawing = $state(false)
	let startPosition: XYPosition | null = $state(null)
	let endPosition: XYPosition | null = $state(null)
	let rect: DOMRect | null = $state(null)
	let contextMenuPosition: XYPosition | null = $state(null)

	function onPointerDown(event: PointerEvent) {
		// Capture pointer to continue tracking outside the element
		const target = event.currentTarget as Element
		target?.setPointerCapture?.(event.pointerId)

		// Use page coordinates as reference
		rect = target.getBoundingClientRect()
		startPosition = {
			x: event.pageX - rect.left,
			y: event.pageY - rect.top
		}
		isDrawing = true
	}

	function onPointerMove(event: PointerEvent) {
		if (event.buttons !== 1) return

		// Use page coordinates as reference
		const target = event.currentTarget as Element
		rect = target.getBoundingClientRect()
		endPosition = {
			x: event.pageX - rect.left,
			y: event.pageY - rect.top
		}
	}

	function onPointerUp() {
		if (!isDrawing || !startPosition || !endPosition || !rect) return

		// We need to convert the start and end positions to absolute positions to then convert to flow positions
		const absoluteStartPosition = {
			x: startPosition.x + rect.left,
			y: startPosition.y + rect.top
		}
		const absoluteEndPosition = {
			x: endPosition.x + rect.left,
			y: endPosition.y + rect.top
		}

		const flowPosition = screenToFlowPosition({
			x: Math.min(absoluteStartPosition.x, absoluteEndPosition.x),
			y: Math.min(absoluteStartPosition.y, absoluteEndPosition.y)
		})
		const position = {
			x: flowPosition.x,
			y: flowPosition.y - (yOffset || 0)
		}

		const zoom = getViewport().zoom
		const size = {
			width: Math.max(
				MIN_NOTE_WIDTH,
				Math.abs(absoluteEndPosition.x - absoluteStartPosition.x) / zoom
			),
			height: Math.max(
				MIN_NOTE_HEIGHT,
				Math.abs(absoluteEndPosition.y - absoluteStartPosition.y) / zoom
			)
		}

		// Create the actual note using NoteEditor context
		if (noteEditorContext?.noteEditor) {
			noteEditorContext.noteEditor.addNote({
				text: '### Free note\nDouble click to edit me',
				position,
				size,
				color: DEFAULT_NOTE_COLOR,
				type: 'free',
				locked: false
			})
		}

		// Exit note mode after creating note
		exitNoteMode?.()

		// Reset state
		isDrawing = false
		startPosition = null
	}

	function handleAddStickyNote() {
		if (!noteEditorContext?.noteEditor || !contextMenuPosition) return

		noteEditorContext.noteEditor.addNote({
			text: '### Free note\nDouble click to edit me',
			position: contextMenuPosition,
			size: { width: 300, height: 200 },
			color: DEFAULT_NOTE_COLOR,
			type: 'free',
			locked: false
		})

		// Clear the position after use
		contextMenuPosition = null

		exitNoteMode?.()
	}

	function handleItemClick(item: ContextMenuItem) {
		if (item.id === 'add-sticky-note') {
			handleAddStickyNote()
		}
	}

	const contextMenuItems: ContextMenuItem[] = [
		{
			id: 'add-sticky-note',
			label: 'Add sticky note',
			icon: StickyNote,
			onClick: handleAddStickyNote
		}
	]

	const previewNote = $derived(
		startPosition && endPosition
			? {
					position: {
						x: Math.min(startPosition['x'], endPosition['x']),
						y: Math.min(startPosition['y'], endPosition['y'])
					},
					size: {
						width: Math.abs(endPosition['x'] - startPosition['x']),
						height: Math.abs(endPosition['y'] - startPosition['y'])
					}
				}
			: null
	)
</script>

<ContextMenu items={contextMenuItems} onItemClick={handleItemClick}>
	<div
		class="tool-overlay"
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		oncontextmenu={(e) => {
			// Capture the position when context menu is triggered
			const flowPosition = screenToFlowPosition({
				x: e.clientX,
				y: e.clientY
			})
			contextMenuPosition = {
				x: flowPosition.x,
				y: flowPosition.y - yOffset
			}
		}}
		role="button"
		tabindex="0"
		aria-label="Click and drag to create a note, or right-click to add a sticky note"
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				if (isDrawing) {
					// Cancel current drawing
					isDrawing = false
					startPosition = null
				} else {
					// Exit note mode
					exitNoteMode?.()
				}
			}
		}}
	>
		<!-- Preview note while drawing -->
		{#if previewNote}
			<div
				class="absolute border-2 border-dashed border-lime-400 bg-lime-100 bg-opacity-50 rounded-md pointer-events-none"
				style="
          width: {previewNote.size.width}px;
          height: {previewNote.size.height}px;
          transform: translate({previewNote.position.x}px, {previewNote.position.y}px);
        "
			>
			</div>
		{/if}
	</div>
</ContextMenu>

<style>
	.tool-overlay {
		pointer-events: auto;
		position: absolute;
		top: 0;
		left: 0;
		z-index: 4;
		height: 100%;
		width: 100%;
		transform-origin: top left;
		cursor: crosshair;
		touch-action: none;
	}
</style>
