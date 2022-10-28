<script lang="ts">
	import { setContext } from 'svelte'
	import type { FlowModule } from '$lib/gen'
	import { Graph, GraphEdge, GraphNodeClass } from './'
	import { FlowModule as FlowModuleComponent, type IFlowModule } from './'
	import { isParent, isParentArray, type NodeSizeContext } from './model'

	const NODE_WIDTH = 260
	const NODE_HEIGHT = 40
	setContext<NodeSizeContext>('nodeSize', {w: NODE_WIDTH, h: NODE_HEIGHT})

	export let modules: FlowModule[] = []
	let graph: IFlowModule[][] = []

	$: graph = modules.map((mod, i) => {
		const type = mod.value.type
		if(type === 'rawscript') {
			return [
				{
					node: new GraphNodeClass(new DOMRect(0, NODE_HEIGHT * 2 * i, NODE_WIDTH, NODE_HEIGHT), i === 0 ? undefined : {row: i - 1, col: 0}),
					title: mod.summary || 'Inline ' + mod.value.language,
					lang: mod.value.language,
					host: 'local'
				}
			]
		} else {
			return [
				{
					node: new GraphNodeClass(new DOMRect(0, NODE_HEIGHT * 2 * i, NODE_WIDTH, NODE_HEIGHT), i === 0 ? undefined : {row: i - 1, col: 0}),
					title: mod.summary || 'For loop',
					lang: 'deno',
					host: 'local'
				}
			]
		}
	})
	$: console.log(modules);
	$: console.log(graph);

	// const graph: IFlowModule[][] = [
	// 	[
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(50, 50, NODE_WIDTH, NODE_HEIGHT), undefined),
	// 			title: 'Inline deno',
	// 			lang: 'deno',
	// 			host: 'local'
	// 		},
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(350, 50, NODE_WIDTH, NODE_HEIGHT), undefined),
	// 			title: 'Hub deno',
	// 			lang: 'deno',
	// 			host: 'hub'
	// 		},
	// 	],
	// 	[
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(50, 150, NODE_WIDTH, NODE_HEIGHT), undefined),
	// 			title: 'Inline go',
	// 			lang: 'go',
	// 			host: 'local'
	// 		},
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(350, 150, NODE_WIDTH, NODE_HEIGHT), [{row: 0, col: 0}, {row: 0, col: 1}]),
	// 			title: 'Hub go',
	// 			lang: 'go',
	// 			host: 'hub'
	// 		},
	// 	],
	// 	[
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(50, 500, NODE_WIDTH, NODE_HEIGHT), {row: 1, col: 0}),
	// 			title: 'Inline python',
	// 			lang: 'python3',
	// 			host: 'local'
	// 		},
	// 		{
	// 			node: new GraphNodeClass(new DOMRect(350, 500, NODE_WIDTH, NODE_HEIGHT), {row: 1, col: 0}),
	// 			title: 'Hub python',
	// 			lang: 'python3',
	// 			host: 'hub'
	// 		},
	// 	],
	// ]

	function getSingleParentEdgePoints(module: IFlowModule) {
		return {
			from: module.node.getParentAnchor(graph.map(r => r.map(m => m.node))) as DOMPoint,
			to: module.node.topAnchor
		}
	}

	function getMultiParentEdgePoints(module: IFlowModule) {
		const parents = <DOMPoint[]>module.node.getParentAnchor(graph.map(r => r.map(m => m.node)))
		return parents.map(from => ({from, to: module.node.topAnchor}))
	}
</script>

<Graph {...$$props.gridProps}>
	{#each graph as row}
		{#each row as module}
			{#if isParent(module.node.parent)}
				<GraphEdge {...getSingleParentEdgePoints(module)} />
			{:else if isParentArray(module.node.parent)}
				{#each getMultiParentEdgePoints(module) as points}
					<GraphEdge {...points} />
				{/each}
			{/if}
		{/each}
	{/each}
	{#each graph as row}
		{#each row as module}
			<FlowModuleComponent {...module} />
		{/each}
	{/each}
</Graph>
