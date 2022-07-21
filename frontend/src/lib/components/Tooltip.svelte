<script lang="ts">
	import Icon from 'svelte-awesome'
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons'
	import { createPopperActions } from 'svelte-popperjs'
	import { fade } from 'svelte/transition'
	const [popperRef, popperContent] = createPopperActions({
		placement: 'auto'
	})
	const betterPreventOverflow = (options) => ({
		name: 'preventOverflow',
		options,
		effect: ({ state }) => {
			const { padding = 0 } = options

			state.elements.popper.style.maxWidth = `calc(100vw - ${padding * 2}px)`
		}
	})
	const extraOpts = {
		modifiers: [betterPreventOverflow({ padding: 50 })]
	}

	let showTooltip = false
	let timeout: NodeJS.Timeout

	function open() {
		clearTimeout(timeout)
		showTooltip = true
	}
	function close() {
		timeout = setTimeout(() => (showTooltip = false), 200)
	}
</script>

<button use:popperRef on:mouseenter={open} on:mouseleave={close}>
	<Icon class="text-gray-500 font-thin inline-block align-middle" data={faInfoCircle} scale={0.8} />
</button>
{#if showTooltip}
	<div
		transition:fade
		id="tooltip"
		use:popperContent={extraOpts}
		on:mouseenter={open}
		on:mouseleave={close}
	>
		<slot />
		<div id="arrow" data-popper-arrow />
	</div>
{/if}

<style>
	#tooltip {
		@apply z-50 font-normal text-gray-300 bg-zinc-800 p-4 rounded-xl whitespace-normal;
	}
</style>
