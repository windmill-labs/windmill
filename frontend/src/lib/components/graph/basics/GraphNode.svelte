<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { ZoomTransform } from 'd3'
	import type { GraphNodeClass, NodeSizeContext } from '..'

	export let node: GraphNodeClass
	const transform = getContext<Writable<ZoomTransform>>('transform')
	const { w: WIDTH, h: HEIGHT } = getContext<NodeSizeContext>('nodeSize')
</script>

<g transform={`translate(${$transform.x} ${$transform.y}) scale(${$transform.k} ${$transform.k})`}>
	<g transform={`translate(${node.box.x} ${node.box.y})`}>
		<rect fill="#ffffff" stroke="#111827" rx="4" width={WIDTH} height={HEIGHT}>
			<slot name="background" />
		</rect>
		<slot />
	</g>
</g>
