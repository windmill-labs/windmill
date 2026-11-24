<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { NODE, wrapperBounds } from './util'
	import { SUBFLOW_END_SUFFIX } from './compoundLayout'
	import type { SubflowBoundN } from './graphBuilder.svelte'

	interface Props {
		allNodes: (Node & { type: string })[]
		hoveredNodeId: string | undefined
		selectedIds: string[]
	}

	let { allNodes, hoveredNodeId, selectedIds }: Props = $props()

	let boxes = $derived.by(() => {
		const nodeMap = new Map(allNodes.map((n) => [n.id, n]))
		const result: { id: string; x: number; y: number; width: number; height: number }[] = []
		for (const start of allNodes) {
			if (start.type !== 'subflowBound') continue
			const d = start.data as SubflowBoundN['data']
			// The end bound carries the same subflowId under another id
			if (d.subflowId !== start.id) continue
			const endId = start.id + SUBFLOW_END_SUFFIX
			// Inner node ids are `subflow:<path>:<id>`, where <path> already carries the
			// parent subflows of a nested one (see buildPrefix in graphBuilder)
			const innerPrefix =
				'subflow:' + (start.id.startsWith('subflow:') ? start.id.slice(8) : start.id) + ':'
			const active =
				hoveredNodeId === start.id ||
				hoveredNodeId === endId ||
				selectedIds.some((id) => id === start.id || id === endId || id.startsWith(innerPrefix))
			const end = nodeMap.get(endId)
			if (!active || !end) continue
			result.push({
				id: start.id,
				...wrapperBounds(start, end, d.wrapperWidth, {
					head: NODE.height,
					end: NODE.height
				})
			})
		}
		return result
	})
</script>

{#each boxes as box (box.id)}
	<ViewportPortal target="back">
		<div
			class="absolute rounded-lg outline outline-1 -outline-offset-1 outline-border-selected bg-surface-selected/10 pointer-events-none"
			style:transform="translate({box.x}px, {box.y}px)"
			style:width="{box.width}px"
			style:height="{box.height}px"
			style:z-index={-10 + box.id.split(':').length}
		></div>
	</ViewportPortal>
{/each}
