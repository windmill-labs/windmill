<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { ZoomTransform } from 'd3'
	import type { NodeSizeContext } from '..'
	import { path } from "d3"
	
	const ADJUSTING = 6
	const COLOR = '#676b73'

	export let from: DOMPoint
	export let to: DOMPoint
	const transform = getContext<Writable<ZoomTransform>>('transform')
	const nodeGap = getContext<NodeSizeContext>('nodeSize').h * 1.5
	let edge: SVGPathElement
	let d: string = ''

	$: if(from && to) drawEdge(from, to)

	function drawEdge(from: DOMPoint, to: DOMPoint) {
		const e = path()
		e.moveTo(from.x, from.y)
		e.bezierCurveTo(from.x, from.y + nodeGap, to.x, to.y - nodeGap, to.x, to.y - ADJUSTING)
		
		d = e.toString()
	}
</script>

<circle
	cx={from.x}
	cy={from.y + 1}
	r="4"
	fill={COLOR}
	transform={`translate(${$transform.x} ${$transform.y}) scale(${$transform.k} ${$transform.k})`}
></circle>
<path
	bind:this={edge}
	stroke={COLOR}
	fill="transparent"
	transform={`translate(${$transform.x} ${$transform.y}) scale(${$transform.k} ${$transform.k})`}
	{d}
></path>
<polygon
	points="{to.x},{to.y - 1} {to.x - 4},{to.y - ADJUSTING} {to.x + 4},{to.y - ADJUSTING}"
	fill={COLOR}
	transform={`translate(${$transform.x} ${$transform.y}) scale(${$transform.k} ${$transform.k})`}
/>