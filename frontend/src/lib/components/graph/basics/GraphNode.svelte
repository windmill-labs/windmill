<script lang="ts">
	import { GraphEdge, isParent, isParentArray, type GraphNodeClass } from '..'

	export let node: GraphNodeClass
	export let fill = '#ffffff'
	export let stroke = '#000000'
	export let noEdge = false

	function getSingleParentAnchor(node: GraphNodeClass) {
		return {
			from: node.getParentAnchor() as DOMPoint,
			to: node.topAnchor
		}
	}

	function getMultiParentAnchors(node: GraphNodeClass) {
		const parents = <DOMPoint[]>node.getParentAnchor()
		return parents.map(from => ({from, to: node.topAnchor}))
	}
</script>

{#if !noEdge}
	{#if isParent(node.parent)}
		<GraphEdge {...getSingleParentAnchor(node)} />
	{:else if isParentArray(node.parent)}
		{#each getMultiParentAnchors(node) as points}
			<GraphEdge {...points} />
		{/each}
	{/if}
{/if}
<g transform={`translate(${node.box.x} ${node.box.y})`}>
	<rect {fill} {stroke} stroke-width="2" rx="4" width={node.box.width} height={node.box.height}>
		<slot name="background" />
	</rect>
	<slot />
</g>
