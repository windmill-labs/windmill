<script lang="ts">
	import { untrack } from 'svelte'
	import { useSvelteFlow, type Node, type Edge } from '@xyflow/svelte'
	import { getSubflowNodeIds, type MoveManager } from './moveManager.svelte'
	import { NODE } from './util'
	import MiniFlowGraph from './MiniFlowGraph.svelte'
	import { Move } from 'lucide-svelte'

	let { moveManager, nodes, edges }: { moveManager: MoveManager; nodes: Node[]; edges: Edge[] } =
		$props()

	const { getViewport } = useSvelteFlow()
	const PADDING = 10
	/** Approximate position of the drag handle (move button) on the node,
	 *  used to offset the ghost so the dragged node aligns with the cursor.
	 *  The handle is at the top-right of the 275px-wide node (right-4 = 16px inset). */
	const DRAG_HANDLE_OFFSET = { x: -90, y: 10 }

	function nodeOffset(n: Node): number {
		return ((n.data as Record<string, unknown>)?.offset as number) ?? 0
	}

	function getSubflowNodesAndEdges(
		moduleId: string,
		allNodes: Node[],
		allEdges: Edge[]
	): { sfNodes: Node[]; sfEdges: Edge[] } {
		const nodeIds = getSubflowNodeIds(moduleId, allNodes, allEdges)
		const sfNodes = allNodes.filter((n) => nodeIds.has(n.id))
		const sfEdges = allEdges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
		return { sfNodes, sfEdges }
	}

	/** Resolve a node's position to absolute flow coordinates.
	 *  xyflow child nodes (with parentId) have positions relative to their parent. */
	function absolutePosition(n: Node, allNodes: Node[]): { x: number; y: number } {
		if (n.parentId) {
			const parent = allNodes.find((p) => p.id === n.parentId)
			if (parent) {
				const parentAbs = absolutePosition(parent, allNodes)
				return { x: parentAbs.x + n.position.x, y: parentAbs.y + n.position.y }
			}
		}
		return { x: n.position.x, y: n.position.y }
	}

	function computeGhost(moduleId: string, allNodes: Node[], allEdges: Edge[]) {
		const { sfNodes, sfEdges } = getSubflowNodesAndEdges(moduleId, allNodes, allEdges)
		if (sfNodes.length === 0) return undefined

		// Compute bounding box using absolute positions
		let minX = Infinity,
			minY = Infinity,
			maxX = -Infinity,
			maxY = -Infinity
		for (const n of sfNodes) {
			const abs = absolutePosition(n, allNodes)
			const x = abs.x + nodeOffset(n)
			const y = abs.y
			const w = n.measured?.width ?? NODE.width
			const h = n.measured?.height ?? NODE.height
			minX = Math.min(minX, x)
			minY = Math.min(minY, y)
			maxX = Math.max(maxX, x + w)
			maxY = Math.max(maxY, y + h)
		}

		const bbWidth = maxX - minX + PADDING * 2
		const bbHeight = maxY - minY + PADDING * 2
		const scale = getViewport().zoom
		const containerWidth = Math.round(bbWidth * scale)
		const containerHeight = Math.round(bbHeight * scale)

		// Compute the offset so the dragged node's center aligns with the cursor
		const mainNode = sfNodes.find((n) => n.id === moduleId)
		let offsetX = containerWidth / 2
		let offsetY = containerHeight / 2
		if (mainNode) {
			const mainAbs = absolutePosition(mainNode, allNodes)
			const mx = mainAbs.x + nodeOffset(mainNode) - minX + PADDING
			const my = mainAbs.y - minY + PADDING
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
	}

	let isNearDrop = $derived(moveManager.nearestDropZone != null)

	let ghost = $derived.by(() => {
		const moduleId = moveManager.dragging?.moduleId
		if (!moduleId) return undefined
		return untrack(() => computeGhost(moduleId, nodes, edges))
	})
</script>

{#if moveManager.dragging}
	<div
		class="fixed pointer-events-none z-[10001] flex items-center justify-center w-5 h-5 rounded-full shadow border border-border transition-colors duration-150 {isNearDrop
			? 'bg-surface-accent-primary text-white'
			: 'bg-surface text-secondary'}"
		style="left: {moveManager.ghostScreenX + 8}px; top: {moveManager.ghostScreenY + 8}px;"
	>
		<Move size={12} />
	</div>
	{#if ghost}
		<div
			class="fixed pointer-events-none z-[10000]"
			style="left: {moveManager.ghostScreenX +
				DRAG_HANDLE_OFFSET.x}px; top: {moveManager.ghostScreenY +
				DRAG_HANDLE_OFFSET.y}px; transform: translate({-ghost.offsetX}px, {-ghost.offsetY}px);"
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
