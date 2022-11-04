<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { ZoomTransform } from 'd3-zoom'
	import {
		FlowLoopInner,
		GraphNode,
		GraphNodeClass,
		GRAPH_CONTEXT_KEY,
		type FlowItem,
		type GraphContext
	} from '.'
	import { ellipsize } from './utils'

	const LABEL_HEIGHT = 30

	export let node: GraphNodeClass
	export let title: string
	export let modules: FlowItem[]
	export let depth: number
	const transform: Writable<ZoomTransform> = getContext('transform')
	const { scale } = getContext<GraphContext>(GRAPH_CONTEXT_KEY).loop
	let topText: SVGTextElement, botText: SVGTextElement

	$: label = {
		top: {
			node: new GraphNodeClass(new DOMRect(0, 0, node.box.width, LABEL_HEIGHT), undefined),
			text: title
		},
		bottom: {
			node: new GraphNodeClass(new DOMRect(0, node.box.height - LABEL_HEIGHT, node.box.width, LABEL_HEIGHT), undefined),
			text: 'Output list'
		}
	}
	$: points = {
		x: label.top.node.topAnchor.x,
		y: label.top.node.box.height * 0.5
	}
	$: maxLabelWidth = (node.box.width - 10) * $transform.k * (scale ** depth)
</script>

<GraphNode {node} stroke="#6e6e6e" fill="transparent">
	<svelte:fragment slot="background">
		<title>{title}</title>
	</svelte:fragment>
	<FlowLoopInner {modules} depth={depth + 1} />
	<GraphNode node={label.top.node} stroke="#6e6e6e">
		<text
			bind:this={topText}
			{...points}
			text-anchor="middle"
			dominant-baseline="middle"
		>
			{ellipsize(label.top.text, topText, maxLabelWidth)}
		</text>
	</GraphNode>
	<GraphNode node={label.bottom.node} stroke="#6e6e6e">
		<text
			bind:this={botText}
			{...points}
			text-anchor="middle"
			dominant-baseline="middle"
		>
			{ellipsize(label.bottom.text, botText, maxLabelWidth)}
		</text>
	</GraphNode>
</GraphNode>
