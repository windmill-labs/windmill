<script lang="ts">
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		Background,
		Controls,
		MiniMap,
		type Node,
		type Edge,
		MarkerType
	} from '@xyflow/svelte'
	import AssetNode from './AssetNode.svelte'
	import RunnableNode from './RunnableNode.svelte'
	import { layoutAssetGraph } from './assetGraphLayout'
	import type { AssetGraphResponse } from './types'

	interface Props {
		graph: AssetGraphResponse
	}
	let { graph }: Props = $props()

	// Build the node/edge model. Producers (w/rw) point runnable → asset;
	// consumers (r) point asset → runnable. This gives a left-to-right DAG
	// where assets sit between their producers and consumers.
	function build(g: AssetGraphResponse) {
		const nodes: Array<{
			id: string
			type: 'asset' | 'runnable'
			data: any
		}> = []
		const edges: Array<{
			id: string
			source: string
			target: string
			access: string | null
		}> = []

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
				data: n.data
			}
		})
	})

	let flowEdges = $derived.by(() =>
		model.edges.map<Edge>((e) => ({
			id: e.id,
			source: e.source,
			target: e.target,
			type: 'smoothstep',
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
</script>

<div class="w-full h-full">
	<SvelteFlow
		{nodes}
		{edges}
		{nodeTypes}
		fitView
		nodesDraggable
		nodesConnectable={false}
		elementsSelectable
		proOptions={{ hideAttribution: true }}
	>
		<Background />
		<Controls />
		<MiniMap pannable zoomable class="!bg-surface" />
	</SvelteFlow>
</div>

<style>
	:global(.svelte-flow) {
		--xy-background-color: transparent;
	}
</style>
