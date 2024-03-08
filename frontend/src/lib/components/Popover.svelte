<script lang="ts">
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from 'svelte-portal'
	import { ExternalLink } from 'lucide-svelte'

	export let placement: PopoverPlacement = 'bottom-end'
	export let notClickable = false
	export let popupClass = ''
	export let disablePopup = false
	export let disappearTimeout = 100
	export let appearTimeout = 300
	export let documentationLink: string | undefined = undefined
	export let style: string | undefined = undefined

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
	let timeout: NodeJS.Timeout | undefined = undefined
	let inTimeout: NodeJS.Timeout | undefined = undefined

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
	<Portal>
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			use:popperContent={popperOptions}
			on:mouseenter={open}
			on:mouseleave={close}
			class="z-[5001] py-2 px-3 rounded-md text-sm font-normal !text-gray-300 bg-gray-800 whitespace-normal text-left {popupClass}"
		>
			<div class="max-w-sm">
				<slot name="text" />
				{#if documentationLink}
					<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
						<div class="flex flex-row gap-2 mt-4">
							See documentation
							<ExternalLink size="16" />
						</div>
					</a>
				{/if}
			</div>
		</div>
	</Portal>
{/if}
