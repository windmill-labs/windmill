<script lang="ts">
	import { afterUpdate } from 'svelte'

	import { findStore } from '../../store/controllers/storeApi'
	import type { NodeType } from '../../store/types/types'

	import { forceCssHeightAndWidth } from '../../customCss/controllers/getCss'

	export let node: NodeType
	export let canvasId: string

	export let nodeId: string
	export let isCustom = false

	const store = findStore(canvasId)

	const { lockedOption } = store

	// forceCssHeightAndWidth forces the size of the node to be defined by CSS
	afterUpdate(() => {
		if (node.className) forceCssHeightAndWidth(store, node)
	})

	const mousedown = (e) => {
		if (node.clickCallback) node.clickCallback(node)
	}
</script>

<!-- on:wheel prevents page scroll when using mousewheel in the Node -->
<div
	on:mousedown={mousedown}
	on:touchstart={mousedown}
	class="{isCustom ? 'CustomNode' : 'Node'} {node.className}"
	style="left: {node.positionX}px;
    top: {node.positionY}px;
    width: {node.width}px;
    height: {node.height}px;
    {isCustom
		? ''
		: `
    background-color: ${node.bgColor ?? 'white'};
    border-color: ${node.borderColor};
    border-radius: ${node.borderRadius}px;
    color: ${node.textColor};
    cursor: ${$lockedOption ? 'default' : 'grab'};`}"
	id="svelvet-{node.id}"
>
	<!-- This executes if node.image is present without node.label -->
	{#if node.image}
		<img
			src={node.src}
			alt=""
			style="width: {node.width * 0.75}px;
			 height: {node.height * 0.75}px;
       overflow: hidden;"
		/>
	{/if}
	<slot />
</div>

<style>
	.CustomNode {
		position: absolute;
		display: grid;
		justify-content: center;
		align-items: center;
		font-size: 14px;
		text-align: center;
		pointer-events: auto;
		box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.2);
	}

	.Node {
		position: absolute;
		display: grid;
		user-select: none;
		justify-content: center;
		overscroll-behavior: auto;
		align-items: center;
		font-size: 14px;
		text-align: center;
		border: solid 1px black;
		border-radius: 5px;
		box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.2);
		pointer-events: auto; /* this is needed for pointer events to work since we disable them in graphview */
	}
	/* the default behavior when click/dragging an image is to move it. This interferes with node dragging. We disable pointer events on img to prevent this */
	/* Alternatively, we could use e.preventDefault() on mouseDown. However, this interferes with embedded Svelvet forms */
	img {
		pointer-events: none;
	}
</style>
