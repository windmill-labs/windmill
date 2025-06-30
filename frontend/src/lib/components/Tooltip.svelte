<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { ExternalLink, InfoIcon } from 'lucide-svelte'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { getContext, hasContext } from 'svelte'
	interface Props {
		light?: boolean
		wrapperClass?: string
		placement?: PopoverPlacement | undefined
		documentationLink?: string | undefined
		small?: boolean
		markdownTooltip?: string | undefined
		customSize?: string
		class?: string
		children?: import('svelte').Snippet
	}

	let {
		light = false,
		wrapperClass = '',
		placement = undefined,
		documentationLink = undefined,
		small = false,
		markdownTooltip = undefined,
		customSize = '100%',
		class: classNames = '',
		children
	}: Props = $props()
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
				: 'text-tertiary'} {classNames} relative"
		>
			<InfoIcon class="{small ? 'bottom-0' : '-bottom-0.5'} absolute" size={small ? 12 : 14} />
		</div>
		{#snippet text()}
			{#if markdownTooltip}
				<div class="prose-sm">
					<Markdown md={markdownTooltip} {plugins} />
				</div>
			{:else}
				{@render children?.()}
			{/if}

			{#if documentationLink}
				<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
					<div class="flex flex-row gap-2 mt-4">
						See documentation
						<ExternalLink size="16" />
					</div>
				</a>
			{/if}
		{/snippet}
	</Popover>
{/if}
