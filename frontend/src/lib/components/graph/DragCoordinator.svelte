<script lang="ts">
	import { useSvelteFlow, type Edge, type Node } from '@xyflow/svelte'
	import { onMount } from 'svelte'
	import type { MoveManager } from './moveManager.svelte'
	import type { GraphEventHandlers } from './graphBuilder.svelte'
	import DragGhost from './DragGhost.svelte'

	let {
		moveManager,
		eventHandlers,
		edges,
		nodes
	}: {
		moveManager: MoveManager
		eventHandlers: GraphEventHandlers
		edges: Edge[]
		nodes: Node[]
	} = $props()

	const { screenToFlowPosition } = useSvelteFlow()

	onMount(() => {
		moveManager.setScreenToFlowPosition(screenToFlowPosition)
	})

	$effect(() => {
		if (!moveManager.dragging) return

		function onPointerMove(e: PointerEvent) {
			moveManager.updateDrag(e.clientX, e.clientY)
		}

		function onPointerUp(_e: PointerEvent) {
			const moduleId = moveManager.dragging?.moduleId
			const zone = moveManager.endDrag()
			if (zone && moduleId) {
				// First set the move (sets movingModuleId on MoveManager)
				eventHandlers.move({ id: moduleId })
				// Then trigger the insert, which detects movingModuleId and performs the splice
				eventHandlers.insert({
					sourceId: zone.sourceId,
					targetId: zone.targetId,
					branch: zone.branch,
					index: zone.index
				})
			}
		}

		function onKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape') {
				moveManager.cancelDrag()
			}
		}

		document.addEventListener('pointermove', onPointerMove)
		document.addEventListener('pointerup', onPointerUp)
		document.addEventListener('keydown', onKeyDown)

		return () => {
			document.removeEventListener('pointermove', onPointerMove)
			document.removeEventListener('pointerup', onPointerUp)
			document.removeEventListener('keydown', onKeyDown)
		}
	})
</script>

<DragGhost {moveManager} {nodes} {edges} />
