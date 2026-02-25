<script lang="ts">
	import { useSvelteFlow, type Edge, type Node } from '@xyflow/svelte'
	import { onMount } from 'svelte'
	import { getSubflowNodeIds, type MoveManager } from './moveManager.svelte'
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

	// Populate draggedNodeIds for both legacy move and drag-and-drop
	$effect(() => {
		const moduleId = moveManager.movingModuleId ?? moveManager.dragging?.moduleId
		if (!moduleId) {
			moveManager.setDraggedNodeIds(new Set())
			return
		}
		const ids = getSubflowNodeIds(moduleId, nodes, edges)
		moveManager.setDraggedNodeIds(ids)
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
				// Set movingModuleId directly (non-toggle) so the insert handler knows which module to relocate
				moveManager.forceSetMoving(moduleId)
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
