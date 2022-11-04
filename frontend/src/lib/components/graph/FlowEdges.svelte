<script lang="ts">
	import {
		GraphEdge,
		isParent,
		isParentArray,
		type FlowItem,
	} from "."

	export let items: FlowItem[]

	function getSingleParentEdgePoints(module: FlowItem) {
		return {
			from: module.node.getParentAnchor() as DOMPoint,
			to: module.node.topAnchor
		}
	}

	function getMultiParentEdgePoints(module: FlowItem) {
		const parents = <DOMPoint[]>module.node.getParentAnchor()
		return parents.map(from => ({from, to: module.node.topAnchor}))
	}
</script>

{#each items as item}
	{#if isParent(item.node.parent)}
		<GraphEdge {...getSingleParentEdgePoints(item)} />
	{:else if isParentArray(item.node.parent)}
		{#each getMultiParentEdgePoints(item) as points}
			<GraphEdge {...points} />
		{/each}
	{/if}
{/each}