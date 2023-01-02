<script lang="ts">
	import { createPopperActions } from 'svelte-popperjs'

	const SIDE = ['auto', 'top', 'bottom', 'left', 'right'] as const
	const ALIGN = ['start', 'end'] as const

	export let placement: `${typeof SIDE[number]}` | `${typeof SIDE[number]}-${typeof ALIGN[number]}` = 'auto'
	export let notClickable = false
	export let popupClass = ''

	const [popperRef, popperContent] = createPopperActions({ placement })
	const betterPreventOverflow = (options) => ({
		name: 'preventOverflow',
		options,
		effect: ({ state }) => {
			const { padding = 0 } = options

			state.elements.popper.style.maxWidth = `calc(100vw - ${padding * 2}px)`
		}
	})
	const extraOpts = {
		modifiers: [
			betterPreventOverflow({ padding: 10 }),
			{ name: 'offset', options: { offset: [8, 8] } },
			{
				name: 'arrow',
				options: {
					padding: 10 // 5px from the edges of the popper
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
		timeout = setTimeout(() => (showTooltip = false), 100)
	}
</script>

<button
	class:cursor-default={notClickable}
	use:popperRef
	on:mouseenter={open}
	on:mouseleave={close}
	on:click
	class={$$props.class}
>
	<slot />
</button>
{#if showTooltip}
	<div
		use:popperContent={extraOpts}
		on:mouseenter={open}
		on:mouseleave={close}
		class="z-50  py-2 px-3 rounded-md  text-sm font-normal !text-gray-300 bg-gray-800
		whitespace-normal text-left {popupClass}"
	>
		<div class="max-w-sm">
			<slot name="text" />
		</div>
	</div>
{/if}
