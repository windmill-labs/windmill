<script lang="ts">
	import { getContext } from 'svelte'
	import {
		FlowModuleNode,
		FlowLoopNode,
		isFlowModuleNode,
		isFlowLoopNode,
		GRAPH_CONTEXT_KEY,
		type GraphContext,
		type FlowItem,
	} from '.'

	export let modules: FlowItem[]
	export let depth: number
	const { padding, scale } = getContext<GraphContext>(GRAPH_CONTEXT_KEY).loop
	let wrapper: SVGGElement
	let halfWidth: number, fullHeight: number

	$: if(wrapper && modules) {
		halfWidth = wrapper.getBoundingClientRect().width * 0.5
	}
	$: if(wrapper && modules) {
		fullHeight = wrapper.getBoundingClientRect().height
	}
</script>

<g transform={`translate(${padding})`}>
	<g bind:this={wrapper} transform={`scale(${scale ** depth})`}>
		{#each modules as item}
			{#if isFlowModuleNode(item)}
				<FlowModuleNode {...item} />
			{:else if isFlowLoopNode(item)}
				<FlowLoopNode {...item} {depth} />
			{/if}
		{/each}
	</g>
</g>
