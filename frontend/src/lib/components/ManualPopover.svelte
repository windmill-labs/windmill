<script lang="ts">
	import { createPopperActions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from 'svelte-portal'
	import { fade } from 'svelte/transition'

	export let placement: PopoverPlacement = 'bottom'

	const [popperRef, popperContent] = createPopperActions({ placement })

	let showTooltip = false
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
			class="z-[901] rounded-lg shadow-md border p-4 bg-surface"
			transition:fade={{ duration: 200 }}
		>
			<slot name="content" />
		</div>
	</Portal>
{/if}
