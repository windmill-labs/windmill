<script lang="ts">
	import type { Node, Edge } from '@xyflow/svelte'
	import type { DragManager } from './dragManager.svelte'
	import { NODE } from './util'
	import MiniFlowGraph from './MiniFlowGraph.svelte'

	let {
		dragManager,
		nodes,
		edges
	}: { dragManager: DragManager; nodes: Node[]; edges: Edge[] } = $props()

	const GHOST_MAX_WIDTH = 300
	const PADDING = 10

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

	let subflow = $derived.by(() => {
		const dragging = dragManager.dragging
		if (!dragging?.isSubflow) return undefined

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
		const scale = Math.min(1, GHOST_MAX_WIDTH / bbWidth)
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
	{#if subflow}
		<div
			class="fixed pointer-events-none z-[10000]"
			style="left: {dragManager.ghostScreenX}px; top: {dragManager.ghostScreenY}px; transform: translate({-subflow.offsetX}px, {-subflow.offsetY}px);"
		>
			<div style="opacity: 0.4; width: {subflow.containerWidth}px; height: {subflow.containerHeight}px;">
				<MiniFlowGraph
					nodes={subflow.ghostNodes}
					edges={subflow.ghostEdges}
					width={subflow.containerWidth}
					height={subflow.containerHeight}
				/>
			</div>
		</div>
	{:else}
		<div
			class="fixed pointer-events-none z-[10000]"
			style="left: {dragManager.ghostScreenX}px; top: {dragManager.ghostScreenY}px; transform: translate(-50%, -50%);"
		>
			<div
				class="rounded-md bg-surface-tertiary shadow-lg border border-border-selected opacity-70 px-3 py-1.5 flex items-center gap-2 text-sm text-primary truncate"
				style="width: {NODE.width}px;"
			>
				<span class="font-medium truncate">{dragManager.dragging.label}</span>
				<span
					class="ml-auto text-2xs bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 px-1.5 py-0.5 rounded font-mono shrink-0"
				>
					{dragManager.dragging.moduleId}
				</span>
			</div>
		</div>
	{/if}
{/if}
