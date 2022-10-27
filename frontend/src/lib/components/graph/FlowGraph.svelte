<script lang="ts">
	import { onMount, setContext } from 'svelte'
	import { zoom, zoomIdentity, select } from 'd3'
	import { GraphNodeClass, type GraphItems, type NodeSizeContext } from './'
	import { writable } from 'svelte/store'
	import { FlowModule, type IFlowModule } from './'

	// Grid
	const GRID_SIZE = 40
	const GRID_DOT_SIZE = 1
	const GRID_COLOR = '#707070'
	// Nodes
	const NODE_WIDTH = 260
	const NODE_HEIGHT = 40

	let svg: Element
	let width = 400,
		height = 400
	let transform = writable(zoomIdentity)

	setContext('transform', transform)
	setContext<NodeSizeContext>('nodeSize', {w: NODE_WIDTH, h: NODE_HEIGHT})

	onMount(() => {
		select(svg)
			.append('pattern')
			.attr('id', 'dot-pattern')
			.attr('patternUnits', 'userSpaceOnUse')
			.attr('x', 0)
			.attr('y', 0)
			.attr('width', GRID_SIZE)
			.attr('height', GRID_SIZE)
			.append('rect')
			.attr('width', GRID_DOT_SIZE)
			.attr('height', GRID_DOT_SIZE)
			.attr('fill', GRID_COLOR)
			.attr('x', GRID_SIZE / 2 - GRID_DOT_SIZE / 2)
			.attr('y', GRID_SIZE / 2 - GRID_DOT_SIZE / 2)

		select(svg)
			.insert('rect', ':first-child')
			.attr('fill', 'url(#dot-pattern)')
			.attr('width', '100%')
			.attr('height', '100%')

		select(svg).call(
			zoom()
				.scaleExtent([1 / 10, 8])
				.on('zoom', zoomed)
		)
		resize()
	})

	function updateGrid(zoomEvent) {
		select(svg)
			.select('#dot-pattern')
			.attr('x', zoomEvent.transform.x)
			.attr('y', zoomEvent.transform.y)
			.attr('width', GRID_SIZE * zoomEvent.transform.k)
			.attr('height', GRID_SIZE * zoomEvent.transform.k)
			.selectAll('rect')
			.attr('x', (GRID_SIZE * zoomEvent.transform.k) / 2 - GRID_DOT_SIZE / 2)
			.attr('y', (GRID_SIZE * zoomEvent.transform.k) / 2 - GRID_DOT_SIZE / 2)
			.attr('opacity', Math.min(zoomEvent.transform.k, 1)) // Lower opacity as the pattern gets more dense.
	}

	function zoomed(currentEvent) {
		transform.set(currentEvent.transform)
		updateGrid(currentEvent)
	}

	function resize() {
		width = svg.getBoundingClientRect().width
		height = svg.getBoundingClientRect().height
	}

	const graph: IFlowModule[][] = [
		[
			{
				node: new GraphNodeClass(new DOMRect(50, 50, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Local deno',
				lang: 'deno',
				host: 'local'
			},
			{
				node: new GraphNodeClass(new DOMRect(350, 50, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Remote deno',
				lang: 'deno',
				host: 'hub'
			},
		],
		[
			{
				node: new GraphNodeClass(new DOMRect(50, 150, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Local go',
				lang: 'go',
				host: 'local'
			},
			{
				node: new GraphNodeClass(new DOMRect(350, 150, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Remote go',
				lang: 'go',
				host: 'hub'
			},
		],
		[
			{
				node: new GraphNodeClass(new DOMRect(50, 250, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Local python',
				lang: 'python3',
				host: 'local'
			},
			{
				node: new GraphNodeClass(new DOMRect(350, 250, NODE_WIDTH, NODE_HEIGHT), undefined),
				title: 'Remote python',
				lang: 'python3',
				host: 'hub'
			},
		],
	]
</script>

<div class="relative h-[400px] w-full rounded overflow-hidden my-2 border">
	<svg
		bind:this={svg}
		{width}
		{height}
		class="bg-base-blue-100 w-full h-full cursor-[grab] active:!cursor-[grabbing]"
	>
		{#each graph as row}
			{#each row as module}
				<FlowModule {...module} />
			{/each}
		{/each}
	</svg>
</div>
