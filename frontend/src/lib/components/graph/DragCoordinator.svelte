<script lang="ts">
	import { useSvelteFlow, type Edge, type Node } from '@xyflow/svelte'
	import { onMount } from 'svelte'
	import type { DragManager } from './dragManager.svelte'
	import type { GraphEventHandlers } from './graphBuilder.svelte'
	import DragGhost from './DragGhost.svelte'

	let {
		dragManager,
		eventHandlers,
		edges,
		nodes
	}: {
		dragManager: DragManager
		eventHandlers: GraphEventHandlers
		edges: Edge[]
		nodes: Node[]
	} = $props()

	const { screenToFlowPosition } = useSvelteFlow()

	onMount(() => {
		dragManager.setScreenToFlowPosition(screenToFlowPosition)
	})

	$effect(() => {
		if (!dragManager.dragging) return

		function onPointerMove(e: PointerEvent) {
			dragManager.updateDrag(e.clientX, e.clientY, edges, nodes)
		}

		function onPointerUp(_e: PointerEvent) {
			const moduleId = dragManager.dragging?.moduleId
			const zone = dragManager.endDrag()
			if (zone && moduleId) {
				// First set the move (toggles $moving store in FlowModuleSchemaMap)
				eventHandlers.move({ id: moduleId })
				// Then trigger the insert, which detects $moving and performs the splice
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
				dragManager.cancelDrag()
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

<DragGhost {dragManager} />
