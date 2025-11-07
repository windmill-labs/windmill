<script lang="ts">
	import { useSvelteFlow, type XYPosition } from '@xyflow/svelte'
	import { NODE } from './util'

	interface Props {
		selectionMode: 'normal' | 'rect-select'
		onNodesSelected: (nodeIds: string[], addToExisting: boolean) => void
		nodes: any[]
	}

	let { selectionMode, onNodesSelected, nodes }: Props = $props()

	const { screenToFlowPosition } = useSvelteFlow()

	let isDrawing = $state(false)
	let startPosition: XYPosition | null = $state(null)
	let endPosition: XYPosition | null = $state(null)
	let rect: DOMRect | null = $state(null)

	// Global handler to handle middle-click panning in rect mode
	function handleGlobalPointerDown(event: PointerEvent) {
		if (selectionMode === 'rect-select' && event.button === 1) {
			// Find the SvelteFlow element and dispatch the event to it
			const flowElement = document.querySelector('.svelte-flow')
			if (flowElement) {
				const syntheticEvent = new PointerEvent('pointerdown', {
					bubbles: true,
					pointerId: event.pointerId,
					button: event.button,
					buttons: event.buttons,
					clientX: event.clientX,
					clientY: event.clientY,
					screenX: event.screenX,
					screenY: event.screenY
				})
				flowElement.dispatchEvent(syntheticEvent)
			}
		}
	}

	function onPointerDown(event: PointerEvent) {
		if (selectionMode !== 'rect-select') return

		// Allow middle-click (button 1) to pass through for graph panning
		if (event.button === 1) {
			// Stop the event from being handled by this overlay
			event.stopPropagation()
			// Re-dispatch to SvelteFlow
			handleGlobalPointerDown(event)
			return
		}

		// Only handle left-click (button 0) for rectangle selection
		if (event.button !== 0) return

		// Capture pointer to continue tracking outside the element
		const target = event.currentTarget as Element
		target?.setPointerCapture?.(event.pointerId)

		// Use page coordinates as reference
		rect = target.getBoundingClientRect()
		startPosition = {
			x: event.pageX - rect.left,
			y: event.pageY - rect.top
		}
		endPosition = startPosition
		isDrawing = true
		event.preventDefault()
	}

	function onPointerMove(event: PointerEvent) {
		if (!isDrawing || !rect) return

		// Use page coordinates as reference
		endPosition = {
			x: event.pageX - rect.left,
			y: event.pageY - rect.top
		}
	}

	function onPointerUp(event: PointerEvent) {
		if (!isDrawing || !startPosition || !endPosition || !rect) return

		// Only proceed if we have a meaningful selection area
		const deltaX = Math.abs(endPosition.x - startPosition.x)
		const deltaY = Math.abs(endPosition.y - startPosition.y)

		if (deltaX > 5 || deltaY > 5) {
			// Convert the start and end positions to absolute positions
			const absoluteStartPosition = {
				x: startPosition.x + rect.left,
				y: startPosition.y + rect.top
			}
			const absoluteEndPosition = {
				x: endPosition.x + rect.left,
				y: endPosition.y + rect.top
			}

			// Convert to flow coordinates
			const flowStart = screenToFlowPosition({
				x: Math.min(absoluteStartPosition.x, absoluteEndPosition.x),
				y: Math.min(absoluteStartPosition.y, absoluteEndPosition.y)
			})

			const flowEnd = screenToFlowPosition({
				x: Math.max(absoluteStartPosition.x, absoluteEndPosition.x),
				y: Math.max(absoluteStartPosition.y, absoluteEndPosition.y)
			})

			// Find nodes within the selection rectangle
			const selectedNodeIds = getNodesInFlowRectangle({
				x1: flowStart.x,
				y1: flowStart.y,
				x2: flowEnd.x,
				y2: flowEnd.y
			})

			if (selectedNodeIds.length > 0) {
				onNodesSelected(selectedNodeIds, event.shiftKey)
			}
		}

		// Reset state
		isDrawing = false
		startPosition = null
		endPosition = null
		rect = null
	}

	function getNodesInFlowRectangle(rect: {
		x1: number
		y1: number
		x2: number
		y2: number
	}): string[] {
		const minX = Math.min(rect.x1, rect.x2)
		const maxX = Math.max(rect.x1, rect.x2)
		const minY = Math.min(rect.y1, rect.y2)
		const maxY = Math.max(rect.y1, rect.y2)

		return nodes
			.filter((node) => {
				const nodeMinX = node.position.x
				const nodeMaxX = node.position.x + NODE.width
				const nodeMinY = node.position.y
				const nodeMaxY = node.position.y + NODE.height

				// Check if node intersects with selection rectangle
				return !(nodeMaxX < minX || nodeMinX > maxX || nodeMaxY < minY || nodeMinY > maxY)
			})
			.map((node) => node.id)
	}

	const previewNote = $derived.by(() => {
		if (!startPosition || !endPosition) return null
		return {
			position: {
				x: Math.min(startPosition.x, endPosition.x),
				y: Math.min(startPosition.y, endPosition.y)
			},
			size: {
				width: Math.abs(endPosition.x - startPosition.x),
				height: Math.abs(endPosition.y - startPosition.y)
			}
		}
	})
</script>

{#if selectionMode === 'rect-select'}
	<div
		class="selection-overlay"
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		role="button"
		tabindex="0"
		aria-label="Click and drag to select nodes"
	>
		<!-- Preview selection rectangle while drawing -->
		{#if previewNote && isDrawing}
			<div
				class="absolute border-2 border-dashed border-accent bg-accent/10 rounded pointer-events-none"
				style="
					width: {previewNote.size.width}px;
					height: {previewNote.size.height}px;
					transform: translate({previewNote.position.x}px, {previewNote.position.y}px);
				"
			></div>
		{/if}
	</div>
{/if}

<style>
	.selection-overlay {
		pointer-events: auto;
		position: absolute;
		top: 0;
		left: 0;
		z-index: 5;
		height: 100%;
		width: 100%;
		cursor: crosshair;
		touch-action: none;
	}
</style>
