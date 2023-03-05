<script lang="ts">
	import { findStore } from '../../../store/controllers/storeApi'
	import { getEdgeById } from '../../../edges/controllers/util'
	import EdgeText from '../Edges/EdgeText.svelte'
	import { get } from 'svelte/store'
	import type { EdgeProps } from '../Edges/types'
	export let baseEdgeProps: EdgeProps
	export let canvasId
	// destructuring the props passed in from the parent component
	$: ({ path, animate, arrow, label, labelBgColor, labelTextColor, edgeColor, centerX, centerY } =
		baseEdgeProps)

	// setting edge text props
	$: edgeTextProps = {
		label: label,
		labelBgColor: labelBgColor,
		labelTextColor: labelTextColor,
		centerX: centerX,
		centerY: centerY
	}

	// Click event handlers
	// At some point in the future, it would be good to refactor event handling to use the flux architecture
	//  ie, events will create an action that will be dispatched to some centralized reducer.
	//  or in other words, the creators of Redux knew what they were doing.
	// The advantage of this re-design would be greater modularity; views would be agnostic to the exact features implmemented,
	//   and they would be only responsible to detecting events and dispatch actions.
	const edgeId = baseEdgeProps.id
	const store = findStore(canvasId)
	const edge = getEdgeById(store, edgeId)
	const highlightEdgesOption = get(store.highlightEdgesOption)
	const handleRightClick = () => {
		const store = findStore(canvasId)
		const { editableOption } = store
		// handles edgeEdit feature
		if (get(editableOption)) store.edgeEditModal.set(edgeId)
	}
	const handleClick = () => {
		const store = findStore(canvasId)
		const edge = getEdgeById(store, edgeId)
		// handles edge clickCallback feature
		if (edge.clickCallback) edge.clickCallback(edge)
		console.log(edge.className)
	}

	const defaultArrow = `0 0, 9 4.5, 0 9`
</script>

<defs>
	<marker id="arrow" markerWidth="9" markerHeight="9" refX="8" refY="4" orient="auto">
		<polygon points={defaultArrow} fill="gray" />
	</marker>
</defs>

<!-- This is an invisible edge that is used to implement event events, because the visible edge is thin and hard to click on. It 
highlights on hover -->
{#if highlightEdgesOption}
	<path
		id={`edgeSelector`}
		d={path}
		fill="transparent"
		stroke={'red'}
		stroke-opacity="0"
		stroke-width="10"
		on:contextmenu={handleRightClick}
		on:click={handleClick}
		on:keypress={() => {}}
	/>
{/if}

{#if arrow}
	<path
		class={animate ? `animate ${edge.className}` : `${edge.className}`}
		d={path}
		fill="transparent"
		stroke={edgeColor ? edgeColor : 'gray'}
		marker-end="url(#arrow)"
		aria-label="svg-path"
	/>
{:else}
	<path
		class={animate ? `animate ${edge.className}` : `${edge.className}`}
		d={path}
		fill="transparent"
		stroke={edgeColor ? edgeColor : 'gray'}
		aria-label="svg-path"
	/>
{/if}

{#if edgeTextProps.label}
	<EdgeText {edgeTextProps} />
{/if}

<style>
	.animate {
		stroke-dasharray: 5;
		animation: dash 50000s linear;
	}
	@keyframes dash {
		from {
			stroke-dashoffset: 1000000;
		}
	}

	#edgeSelector:hover {
		stroke: 'red';
		stroke-opacity: 0.5;
	}
</style>
