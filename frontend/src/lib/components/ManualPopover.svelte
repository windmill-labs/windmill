<script lang="ts">
	import { createPopperActions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from 'svelte-portal'
	import { fade } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	export let placement: PopoverPlacement = 'bottom'

	const [popperRef, popperContent, getInstance] = createPopperActions({ placement })

	export async function refresh() {
		await getInstance()?.update()
	}

	export let showTooltip = false
	export function open() {
		showTooltip = true
	}
	export function close() {
		showTooltip = false
	}
</script>

<fragment use:popperRef>
	<slot />
</fragment>
{#if showTooltip}
	<Portal>
		<div
			use:popperContent
			class={twMerge('z-[901] rounded-lg shadow-md border p-4 bg-surface', $$props.class)}
			transition:fade={{ duration: 200 }}
		>
			<slot name="content" />
		</div>
	</Portal>
{/if}
