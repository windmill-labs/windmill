<script lang="ts">
	import { onMount, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { zoom, zoomIdentity, select } from 'd3'

	// Grid defaults
	const GRID_SIZE = 40
	const GRID_DOT_SIZE = 1
	const GRID_COLOR = '#707070'

	export let width: string | number = 400
	export let height: string | number = 400
	let svg: Element
	let transform = writable(zoomIdentity)

	setContext('transform', transform)

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
</script>

<div class="relative h-[400px] w-full rounded overflow-hidden my-2 border {$$props.class}">
	<svg
		bind:this={svg}
		{width}
		{height}
		class="w-full h-full cursor-[grab] active:!cursor-[grabbing]"
	>
		<slot />
	</svg>
</div>
