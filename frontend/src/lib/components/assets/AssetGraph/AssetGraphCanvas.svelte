<script lang="ts">
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		Background,
		Controls,
		MiniMap,
		ConnectionLineType,
		type Node,
		type Edge,
		MarkerType
	} from '@xyflow/svelte'
	import AssetNode from './AssetNode.svelte'
	import RunnableNode from './RunnableNode.svelte'
	import AssetGraphEdge from './AssetGraphEdge.svelte'
	import { layoutAssetGraph } from './assetGraphLayout'
	import type { AssetGraphResponse, AssetGraphSelection } from './types'

	interface Props {
		graph: AssetGraphResponse
		selection?: AssetGraphSelection | undefined
		onselect?: (selection: AssetGraphSelection | undefined) => void
	}
	let { graph, selection, onselect }: Props = $props()

	// Producers (w/rw) point runnable → asset; consumers (r) point asset →
	// runnable. The sugiyama layout (assetGraphLayout.ts) renders this as a
	// top-down DAG: producers above, assets in the middle, consumers below.
	function build(g: AssetGraphResponse) {
		const nodes: Array<{ id: string; type: 'asset' | 'runnable'; data: any }> = []
		const edges: Array<{ id: string; source: string; target: string; access: string | null }> = []

		for (const a of g.assets) {
			nodes.push({
				id: `asset:${a.kind}:${a.path}`,
				type: 'asset',
				data: { asset_kind: a.kind, path: a.path }
			})
		}
		for (const r of g.runnables) {
			nodes.push({
				id: `${r.usage_kind}:${r.path}`,
				type: 'runnable',
				data: { runnable_kind: r.usage_kind, path: r.path }
			})
		}

		for (const e of g.edges) {
			const runnableId = `${e.runnable_kind}:${e.runnable_path}`
			const assetId = `asset:${e.asset_kind}:${e.asset_path}`
			const access = e.access_type ?? 'r'
			if (access === 'w' || access === 'rw') {
				edges.push({
					id: `prod:${runnableId}->${assetId}`,
					source: runnableId,
					target: assetId,
					access
				})
			}
			if (access === 'r' || access === 'rw') {
				edges.push({
					id: `cons:${assetId}->${runnableId}`,
					source: assetId,
					target: runnableId,
					access
				})
			}
		}
		return { nodes, edges }
	}

	let model = $derived(build(graph))

	let selectedId = $derived.by(() => {
		if (!selection) return undefined
		return selection.kind === 'asset'
			? `asset:${selection.asset_kind}:${selection.path}`
			: `${selection.runnable_kind}:${selection.path}`
	})

	let positionedNodes = $derived.by(() => {
		const positions = layoutAssetGraph({
			nodes: model.nodes.map((n) => ({ id: n.id, data: n.data })),
			edges: model.edges.map((e) => ({ source: e.source, target: e.target }))
		})
		return model.nodes.map<Node>((n) => {
			const p = positions.get(n.id) ?? { x: 0, y: 0 }
			return {
				id: n.id,
				type: n.type,
				position: { x: p.x, y: p.y },
				data: n.data,
				selected: n.id === selectedId
			}
		})
	})

	let flowEdges = $derived.by(() =>
		model.edges.map<Edge>((e) => ({
			id: e.id,
			source: e.source,
			target: e.target,
			type: 'asset',
			animated: e.access === 'rw',
			style:
				e.access === 'w' || e.access === 'rw'
					? 'stroke: rgb(59 130 246); stroke-width: 1.5px;'
					: 'stroke: rgb(107 114 128); stroke-width: 1.25px;',
			markerEnd: { type: MarkerType.ArrowClosed, width: 14, height: 14 }
		}))
	)

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])
	$effect(() => {
		nodes = positionedNodes
	})
	$effect(() => {
		edges = flowEdges
	})

	const nodeTypes = {
		asset: AssetNode as any,
		runnable: RunnableNode as any
	}

	const edgeTypes = {
		asset: AssetGraphEdge as any
	}

	function handleNodeClick({ node }: { node: Node }) {
		if (!onselect) return
		const data = node.data as any
		if (node.type === 'asset') {
			onselect({ kind: 'asset', asset_kind: data.asset_kind, path: data.path })
		} else {
			onselect({ kind: 'runnable', runnable_kind: data.runnable_kind, path: data.path })
		}
	}
</script>

<div class="w-full h-full relative">
	<SvelteFlow
		{nodes}
		{edges}
		{nodeTypes}
		{edgeTypes}
		fitView
		minZoom={0.2}
		maxZoom={1.6}
		nodesDraggable={false}
		nodesConnectable={false}
		elementsSelectable
		zoomOnDoubleClick={false}
		connectionLineType={ConnectionLineType.SmoothStep}
		defaultEdgeOptions={{ type: 'asset' }}
		proOptions={{ hideAttribution: true }}
		onnodeclick={handleNodeClick}
		onpaneclick={() => onselect?.(undefined)}
		--background-color={false}
	>
		<div class="absolute inset-0 !bg-surface-secondary h-full -z-10"></div>
		<Background />
		<Controls />
		<MiniMap
			pannable
			zoomable
			class="!bg-surface"
			nodeColor={(n) => (n.type === 'asset' ? 'rgb(96 165 250 / 0.5)' : 'rgb(52 211 153 / 0.5)')}
			nodeStrokeColor="transparent"
			maskColor="rgb(0 0 0 / 0.2)"
		/>
	</SvelteFlow>
</div>

<style lang="postcss">
	:global(.svelte-flow__handle) {
		opacity: 0;
	}
	:global(.svelte-flow__controls-button) {
		@apply bg-surface border-0;
	}
	:global(.svelte-flow__controls-button:hover) {
		@apply bg-surface-hover;
	}
	:global(.svelte-flow__node.selected .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-500;
	}
</style>
