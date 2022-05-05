<script lang="ts">
	import Icon from 'svelte-awesome';
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons';
	import { fade } from 'svelte/transition';

	export let position: ActionKind = 'above';
	export let direction: DirectionKind = 'default';
	type ActionKind = 'above' | 'below';
	type DirectionKind = 'default' | 'left';

	let viewTooltip = false;
	let mouseOnMessage = false;
	let mouseOver = false;

	// This function makes sure the tooltip does not disappear when moving the mouse to the tooltip
	// message, so that users can click links there for example
	function hideTooltip(): void {
		setTimeout(function () {
			if (!mouseOnMessage && !mouseOver) {
				viewTooltip = false;
			} else {
				hideTooltip();
			}
		}, 300);
	}
</script>

<div class="inline-block has-tooltip {$$props.class}">
	{#if viewTooltip}
		<span
			transition:fade
			class="tooltip rounded shadow-lg p-1 leading-4 {position === 'above' ? '-mt-8' : 'mt-6'} 
			{viewTooltip ? 'tooltip-visible' : 'tooltip'} {direction === 'left' ? 'right-0' : ''}
			text-2xs text-gray-700"
			on:mouseover={() => {
				mouseOnMessage = true;
			}}
			on:focus={() => {}}
			on:mouseout={() => {
				mouseOnMessage = false;
			}}
			on:blur={() => {}}
			><slot />
		</span>
	{/if}
	<Icon class="text-gray-500 font-thin inline-block align-middle" data={faInfoCircle} scale={0.6} />
	<!-- Hovering on this (invisible) area triggers the apparition of the tooltip. Needed because the icon is too small-->
	<div
		class="relative w-4 h-5 -mt-5 -ml-1"
		on:mouseover={() => {
			viewTooltip = true;
			mouseOver = true;
		}}
		on:focus={() => {
			viewTooltip = true;
		}}
		on:mouseout={() => {
			hideTooltip();
			mouseOver = false;
		}}
		on:blur={() => {}}
	/>
</div>

<style>
	.tooltip {
		@apply invisible absolute;
		@apply duration-100 transition-opacity;
	}

	.tooltip-visible {
		@apply absolute visible z-50 bg-white;
		@apply max-w-sm;
		@apply transition-opacity;
	}
</style>
