<script lang="ts">
	import type { Placement } from '@floating-ui/core'
	import { InfoIcon } from 'lucide-svelte'
	import { zIndexes } from '$lib/zIndexes'

	import { createTooltip, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import TooltipInner from '../TooltipInner.svelte'

	export let light = false
	export let placement: Placement | undefined = 'bottom'
	export let documentationLink: string | undefined = undefined
	export let small = false
	export let markdownTooltip: string | undefined = undefined
	export let disablePopup: boolean = false
	export let openDelay: number = 300
	export let closeDelay: number = 0
	export let portal: string | undefined | null = 'body'

	const {
		elements: { trigger, content },
		states: { open }
	} = createTooltip({
		positioning: {
			placement
		},
		openDelay,
		closeDelay,
		group: true,
		portal
	})
</script>

<span class={$$props.class} use:melt={$trigger}>
	<slot />
</span>
{#if !$$slots.default}
	<div
		class="inline-flex w-3 mx-0.5 h-3 {light
			? 'text-tertiary-inverse'
			: 'text-tertiary'} {$$props.class} "
		use:melt={$trigger}
	>
		<InfoIcon size={small ? 12 : 14} />
	</div>
{/if}

{#if $open && !disablePopup}
	<div use:melt={$content} transition:fade={{ duration: 100 }} style="z-index: {zIndexes.tooltip}">
		<TooltipInner {documentationLink} {markdownTooltip}>
			<slot name="text" />
		</TooltipInner>
	</div>
{/if}
