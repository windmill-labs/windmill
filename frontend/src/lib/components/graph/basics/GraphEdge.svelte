<script lang="ts">
	import { getContext } from 'svelte'
	import { path } from "d3"
	import { GRAPH_CONTEXT_KEY, type GraphContext } from '..'
	
	const ADJUSTING = 6
	const COLOR = '#676b73'

	export let from: DOMPoint | undefined
	export let to: DOMPoint | undefined
	const nodeGap = getContext<GraphContext>(GRAPH_CONTEXT_KEY).gap.vertical
	let edge: SVGPathElement
	let d: string = ''

	$: if(from && to) drawEdge(from, to)

	function drawEdge(from: DOMPoint, to: DOMPoint) {
		const e = path()
		e.moveTo(from.x, from.y + 1)
		e.bezierCurveTo(from.x, from.y + nodeGap, to.x, to.y - nodeGap, to.x, to.y - ADJUSTING)
		
		d = e.toString()
	}
</script>

{#if d && to}
	<g>
		<path
			bind:this={edge}
			stroke={COLOR}
			fill="transparent"
			{d}
		></path>
		<polygon
			points="{to.x},{to.y - 1} {to.x - 4},{to.y - ADJUSTING} {to.x + 4},{to.y - ADJUSTING}"
			fill={COLOR}
		/>
	</g>
{/if}