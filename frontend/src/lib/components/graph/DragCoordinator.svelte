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
		moveManager.setComputeDraggedNodeIds((moduleIds) => {
			const combined = new Set<string>()
			for (const id of moduleIds) {
				for (const nodeId of getSubflowNodeIds(id, nodes, edges)) {
					combined.add(nodeId)
				}
			}
			return combined
		})
	})

	$effect(() => {
		if (!moveManager.dragging) return

		function onPointerMove(e: PointerEvent) {
			moveManager.updateDrag(e.clientX, e.clientY)
		}

		function onPointerUp(_e: PointerEvent) {
			const moduleId = moveManager.dragging?.moduleId
			const selectedIds = moveManager.dragging?.selectedIds
			const zone = moveManager.endDrag()
			if (zone && moduleId) {
				// Set moving state so the insert handler knows which module(s) to relocate
				if (selectedIds && selectedIds.length > 1) {
					moveManager.movingModuleId = selectedIds[0]
					moveManager.movingIds = selectedIds
				} else {
					moveManager.setMoving(moduleId)
				}
				// Then trigger the insert, which detects movingModuleId/movingIds and performs the splice
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
		document.addEventListener('keydown', onKeyDown, true)

		return () => {
			document.removeEventListener('pointermove', onPointerMove)
			document.removeEventListener('pointerup', onPointerUp)
			document.removeEventListener('keydown', onKeyDown, true)
		}
	})
</script>

<DragGhost {moveManager} {nodes} {edges} />
