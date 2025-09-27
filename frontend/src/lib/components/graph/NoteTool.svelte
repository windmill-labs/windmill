<script lang="ts">
	import { useSvelteFlow, type XYPosition } from '@xyflow/svelte'

	interface Props {
		onNoteAdded?: (note: any) => void
	}

	let { onNoteAdded }: Props = $props()

	const { screenToFlowPosition, getViewport } = useSvelteFlow()

	let isDrawing = $state(false)
	let startPosition: XYPosition | null = $state(null)
	let endPosition: XYPosition | null = $state(null)
	let rect: DOMRect | null = $state(null)

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
		console.log('NoteTool: pointer move', event)
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

		const position = screenToFlowPosition({
			x: Math.min(absoluteStartPosition.x, absoluteEndPosition.x),
			y: Math.min(absoluteStartPosition.y, absoluteEndPosition.y)
		})

		const zoom = getViewport().zoom
		const size = {
			width: Math.abs(absoluteEndPosition.x - absoluteStartPosition.x) / zoom,
			height: Math.abs(absoluteEndPosition.y - absoluteStartPosition.y) / zoom
		}

		// Create the actual note with the calculated size and position
		onNoteAdded?.({
			position,
			size
		})

		// Reset state
		isDrawing = false
		startPosition = null
	}

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

<div
	class="tool-overlay"
	onpointerdown={onPointerDown}
	onpointermove={onPointerMove}
	onpointerup={onPointerUp}
	role="button"
	tabindex="0"
	aria-label="Click and drag to create a note"
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			if (isDrawing) {
				// Cancel current drawing
				isDrawing = false
				startPosition = null
			} else {
				// Exit note mode
				onNoteAdded?.(null)
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
