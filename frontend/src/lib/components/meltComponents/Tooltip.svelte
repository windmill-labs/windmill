<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import type { Placement } from '@floating-ui/core'
	import { ExternalLink, InfoIcon } from 'lucide-svelte'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { zIndexes } from '$lib/zIndexes'

	import { createTooltip, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'

	export let light = false
	export let placement: Placement | undefined = 'bottom'
	export let documentationLink: string | undefined = undefined
	export let small = false
	export let markdownTooltip: string | undefined = undefined
	export let disablePopup: boolean = false
	export let openDelay: number = 300
	export let closeDelay: number = 0
	export let portal: string | undefined | null = 'body'

	const plugins = [gfmPlugin()]

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
	<div
		use:melt={$content}
		transition:fade={{ duration: 100 }}
		class="shadow max-w-sm break-words py-2 px-3 rounded-md text-sm font-normal !text-gray-300 bg-gray-800 whitespace-normal text-left"
		style="z-index: {zIndexes.tooltip}"
	>
		{#if markdownTooltip}
			<div class="prose-sm">
				<Markdown md={markdownTooltip} {plugins} />
			</div>
		{:else}
			<slot name="text" />
		{/if}

		{#if documentationLink}
			<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
				<div class="flex flex-row gap-2 mt-4">
					See documentation
					<ExternalLink size="16" />
				</div>
			</a>
		{/if}
	</div>
{/if}
