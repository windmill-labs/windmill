<script lang="ts">
	import { useSvelteFlow, type Node, type Edge } from '@xyflow/svelte'
	import type { DragManager } from './dragManager.svelte'
	import { NODE } from './util'
	import MiniFlowGraph from './MiniFlowGraph.svelte'
	import { ArrowUpDown } from 'lucide-svelte'

	let { dragManager, nodes, edges }: { dragManager: DragManager; nodes: Node[]; edges: Edge[] } =
		$props()

	const { getViewport } = useSvelteFlow()
	const GHOST_ZOOM_FACTOR = 0.8
	const PADDING = 10
	/** Offset from the move button (drag handle) to the node center */
	const MOVE_BTN_OFFSET = { x: -90, y: 10 }

	function getSubflowNodesAndEdges(
		moduleId: string,
		allNodes: Node[],
		allEdges: Edge[]
	): { sfNodes: Node[]; sfEdges: Edge[] } {
		const nodeIdPrefix = moduleId + '-'
		const nodeIds = new Set<string>()

		for (const n of allNodes) {
			if (n.id === moduleId || n.id.startsWith(nodeIdPrefix)) {
				nodeIds.add(n.id)
			}
		}

		for (const e of allEdges) {
			const disableIds: string[] = (e.data as any)?.disableMoveIds ?? []
			if (disableIds.includes(moduleId)) {
				nodeIds.add(e.source)
				nodeIds.add(e.target)
			}
		}

		const sfNodes = allNodes.filter((n) => nodeIds.has(n.id))
		const sfEdges = allEdges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))

		return { sfNodes, sfEdges }
	}

	let isNearDrop = $derived(dragManager.nearestDropZone != null)

	let ghost = $derived.by(() => {
		const dragging = dragManager.dragging
		if (!dragging) return undefined

		const { sfNodes, sfEdges } = getSubflowNodesAndEdges(dragging.moduleId, nodes, edges)
		if (sfNodes.length === 0) return undefined

		// Compute bounding box
		let minX = Infinity,
			minY = Infinity,
			maxX = -Infinity,
			maxY = -Infinity
		for (const n of sfNodes) {
			const x = n.position.x + ((n.data as any).offset ?? 0)
			const y = n.position.y
			const w = n.measured?.width ?? NODE.width
			const h = n.measured?.height ?? NODE.height
			minX = Math.min(minX, x)
			minY = Math.min(minY, y)
			maxX = Math.max(maxX, x + w)
			maxY = Math.max(maxY, y + h)
		}

		const bbWidth = maxX - minX + PADDING * 2
		const bbHeight = maxY - minY + PADDING * 2
		const scale = getViewport().zoom * GHOST_ZOOM_FACTOR
		const containerWidth = Math.round(bbWidth * scale)
		const containerHeight = Math.round(bbHeight * scale)

		// Compute the offset so the dragged node's center aligns with the cursor
		const mainNode = sfNodes.find((n) => n.id === dragging.moduleId)
		let offsetX = containerWidth / 2
		let offsetY = containerHeight / 2
		if (mainNode) {
			const mx = mainNode.position.x + ((mainNode.data as any).offset ?? 0) - minX + PADDING
			const my = mainNode.position.y - minY + PADDING
			const mw = mainNode.measured?.width ?? NODE.width
			const mh = mainNode.measured?.height ?? NODE.height
			offsetX = (mx + mw / 2) * scale
			offsetY = (my + mh / 2) * scale
		}

		// Clone nodes/edges with interactive features disabled
		const ghostNodes = sfNodes.map((n) => ({
			...n,
			data: { ...n.data, insertable: false, editMode: false, moving: undefined }
		}))
		const ghostEdges = sfEdges.map((e) => ({
			...e,
			data: { ...e.data, insertable: false, editMode: false }
		}))

		return { containerWidth, containerHeight, ghostNodes, ghostEdges, offsetX, offsetY }
	})
</script>

{#if dragManager.dragging}
	<div
		class="fixed pointer-events-none z-[10001] flex items-center justify-center w-5 h-5 rounded-full shadow border border-border transition-colors duration-150 {isNearDrop
			? 'bg-surface-accent-primary text-white'
			: 'bg-surface text-secondary'}"
		style="left: {dragManager.ghostScreenX + 12}px; top: {dragManager.ghostScreenY + 12}px;"
	>
		<ArrowUpDown size={12} />
	</div>
	{#if ghost}
		<div
			class="fixed pointer-events-none z-[10000]"
			style="left: {dragManager.ghostScreenX + MOVE_BTN_OFFSET.x}px; top: {dragManager.ghostScreenY +
				MOVE_BTN_OFFSET.y}px; transform: translate({-ghost.offsetX}px, {-ghost.offsetY}px);"
		>
			<div
				style="opacity: {isNearDrop
					? 0.8
					: 0.25}; width: {ghost.containerWidth}px; height: {ghost.containerHeight}px; transition: opacity 150ms ease;"
			>
				<MiniFlowGraph
					nodes={ghost.ghostNodes}
					edges={ghost.ghostEdges}
					width={ghost.containerWidth}
					height={ghost.containerHeight}
				/>
			</div>
		</div>
	{/if}
{/if}
