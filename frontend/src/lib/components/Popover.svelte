<script lang="ts">
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'

	export let placement: PopoverPlacement = 'auto'
	export let notClickable = false
	export let popupClass = ''
	export let disablePopup = false
	export let disapperTimoout = 100

	const [popperRef, popperContent] = createPopperActions({ placement })

	const popperOptions: PopperOptions<{}> = {
		placement: 'bottom-end',
		strategy: 'fixed',
		modifiers: [
			{ name: 'offset', options: { offset: [8, 8] } },
			{
				name: 'arrow',
				options: {
					padding: 10
				}
			}
		]
	}

	let showTooltip = false
	let timeout: NodeJS.Timeout

	function open() {
		clearTimeout(timeout)
		showTooltip = true
	}
	function close() {
		timeout = setTimeout(() => (showTooltip = false), disapperTimoout)
	}
</script>

{#if notClickable}
	<span use:popperRef on:mouseenter={open} on:mouseleave={close} class={$$props.class}>
		<slot />
	</span>
{:else}
	<button use:popperRef on:mouseenter={open} on:mouseleave={close} on:click class={$$props.class}>
		<slot />
	</button>
{/if}
{#if showTooltip && !disablePopup}
	<div
		use:popperContent={popperOptions}
		on:mouseenter={open}
		on:mouseleave={close}
		class="z-50 py-2 px-3 rounded-md text-sm font-normal !text-gray-300 bg-gray-800
		whitespace-normal text-left {popupClass}"
	>
		<div class="max-w-sm">
			<slot name="text" />
		</div>
	</div>
{/if}
