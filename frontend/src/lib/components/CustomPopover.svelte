<script lang="ts">
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from '$lib/components/Portal.svelte'

	export let placement: PopoverPlacement = 'bottom-end'
	export let notClickable = false
	export let disablePopup = false
	export let disappearTimeout = 100
	export let appearTimeout = 300
	export let style: string | undefined = undefined
	export let noPadding = false

	export let focusEl: HTMLElement | undefined = undefined

	const [popperRef, popperContent] = createPopperActions({ placement })

	const popperOptions: PopperOptions<{}> = {
		placement,
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
	let timeout: number | undefined = undefined
	let inTimeout: number | undefined = undefined

	function open() {
		clearTimeout(timeout)
		if (appearTimeout == 0) {
			showTooltip = true
		} else {
			inTimeout = setTimeout(() => (showTooltip = true), appearTimeout)
		}
	}
	function close() {
		inTimeout && clearTimeout(inTimeout)
		inTimeout = undefined
		timeout = setTimeout(() => (showTooltip = false), disappearTimeout)
	}

	$: focusEl && focusEl?.focus()
</script>

{#if notClickable}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<span {style} use:popperRef on:mouseenter={open} on:mouseleave={close} class={$$props.class}>
		<slot />
	</span>
{:else}
	<button
		{style}
		use:popperRef
		on:mouseenter={open}
		on:mouseleave={close}
		on:click
		class={$$props.class}
	>
		<slot />
	</button>
{/if}
{#if showTooltip && !disablePopup}
	<Portal name="custom-popover">
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			use:popperContent={popperOptions}
			on:mouseenter={open}
			on:mouseleave={close}
			class="z-[5001] border rounded-lg shadow-lg {noPadding ? '' : 'p-4'} bg-surface"
		>
			<slot name="overlay" />
		</div>
	</Portal>
{/if}
