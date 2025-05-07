<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { ExternalLink, InfoIcon } from 'lucide-svelte'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { getContext, hasContext } from 'svelte'
	export let light = false
	export let wrapperClass = ''
	export let placement: PopoverPlacement | undefined = undefined
	export let documentationLink: string | undefined = undefined
	export let small = false
	export let markdownTooltip: string | undefined = undefined
	export let customSize: string = '100%'
	const plugins = [gfmPlugin()]

	const disableTooltips = hasContext('disableTooltips')
		? getContext('disableTooltips') === true
		: false
</script>

{#if disableTooltips !== true}
	<Popover
		notClickable
		{placement}
		class={wrapperClass}
		style="transform: scale({parseFloat(customSize) / 100});"
	>
		<div
			class="inline-flex w-3 mx-0.5 h-3 {light
				? 'text-tertiary-inverse'
				: 'text-tertiary'} {$$props.class} relative"
		>
			<InfoIcon class="{small ? 'bottom-0' : '-bottom-0.5'} absolute" size={small ? 12 : 14} />
		</div>
		<svelte:fragment slot="text">
			{#if markdownTooltip}
				<div class="prose-sm">
					<Markdown md={markdownTooltip} {plugins} />
				</div>
			{:else}
				<slot />
			{/if}

			{#if documentationLink}
				<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
					<div class="flex flex-row gap-2 mt-4">
						See documentation
						<ExternalLink size="16" />
					</div>
				</a>
			{/if}
		</svelte:fragment>
	</Popover>
{/if}
