<script lang="ts">
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import type { FlowModule, RawScript } from '$lib/gen'
	import {
		Graph,
		GraphNodeClass,
		FlowLoopNode,
		FlowModuleNode,
		GRAPH_CONTEXT_KEY,
		isFlowLoopNode,
		isFlowModuleNode,
		type FlowItem,
		type FlowItemsGraph,
		type GraphContext,
		type IFlowLoopNode,
		type IFlowModuleNode,
		type ModuleHost
	} from './'

	export let modules: FlowModule[] = []
	const graph = writable<FlowItemsGraph>([])
	const context: GraphContext = {
		graph,
		node: {
			width: 260,
			height: 40,
		},
		loop: {
			width: 260,
			padding: 26,
			scale: 0.8
		},
		gap: {
			vertical: 100,
			horizontal: 40
		}
	} as const
	
	setContext<GraphContext>(GRAPH_CONTEXT_KEY, context)

	$: if(modules) {
		const g: FlowItemsGraph = []
		for (let i = 0; i < modules.length; i++) {
			const module = getConvertedFlowModule(i === 0 ? undefined : g[i - 1], g[i], modules[i])
			if(module) g.push([module])
		}
		context.graph.set(g)
	}

	function getConvertedFlowModule(parentLevel: FlowItem[] | undefined, currentLevel: FlowItem[], module: FlowModule) {
		const type = module.value.type
		if(type === 'rawscript') {
			const lang = module.value.language
			return flowModuleToNode(parentLevel, currentLevel, module.summary || 'Inline ' + lang, 'inline', lang)
		} else if(type === 'script') {
			return flowModuleToNode(parentLevel, currentLevel, module.summary || module.value.path, 'hub')
		} else if(type === 'forloopflow') {
			return flowModuleToLoop(parentLevel, module.value.modules, module.summary)
		}
	}

	function flowModuleToNode(
		parentLevel: FlowItem[] | undefined,
		currentLevel: FlowItem[],
		title: string,
		host: ModuleHost,
		lang?: RawScript.language
	): IFlowModuleNode {
		return {
			node: createNodeClass(parentLevel, currentLevel),
			title,
			host,
			lang
		}
	}

	function flowModuleToLoop(parentLevel: FlowItem[] | undefined, modules: FlowModule[], title?: string): IFlowLoopNode {
		const items: FlowItem[] = []
		for (let i = 0; i < modules.length; i++) {
			const module = getConvertedFlowModule(parentLevel, items, modules[i])
			if(module) items.push(module)
		}
		const node = createNodeClass(parentLevel, items, true)
		items[0] && items[0].node.setParent(undefined, false)
		items.forEach(({node}, i) => node.updateBox({ y: getIncrementalYPos(i) }))
		return {
			node,
			title: title || 'For loop',
			modules: items
		}
	}

	function createNodeClass(parentLevel: FlowItem[] | undefined, currentLevel: FlowItem[], isLoopRoot = false) {
		const parent = currentLevel?.length && !isLoopRoot ? currentLevel.at(-1) : parentLevel?.at(-1)
		return new GraphNodeClass(
			{
				y: context.gap.vertical + (parent ? parent.node.botAnchor.y : 0),
				width: context[isLoopRoot ? 'loop' : 'node'].width,
				height: isLoopRoot ? getIncrementalYPos(currentLevel.length) : context.node.height
			},
			parent?.node
		)
	}

	function getIncrementalYPos(i: number) {
		return i * (context.node.height + context.gap.vertical) + context.gap.vertical
	}
</script>

<Graph {...$$props.gridProps}>
	{#each $graph as row}
		{#each row as item}
			{#if isFlowModuleNode(item)}
				<FlowModuleNode {...item} />
			{:else if isFlowLoopNode(item)}
				<FlowLoopNode {...item} depth={0} />
			{/if}
		{/each}
	{/each}
</Graph>
