<script lang="ts">
	import { untrack } from 'svelte'
	import type { Placement } from '@floating-ui/core'
	import { InfoIcon } from 'lucide-svelte'
	import { zIndexes } from '$lib/zIndexes'

	import { createTooltip, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import TooltipInner from '../TooltipInner.svelte'

	interface Props {
		light?: boolean
		placement?: Placement | undefined
		documentationLink?: string | undefined
		small?: boolean
		disablePopup?: boolean
		openDelay?: number
		closeDelay?: number
		portal?: string | undefined | null
		customBgClass?: string | undefined
		style?: string
		class?: string
		children?: import('svelte').Snippet
		text?: import('svelte').Snippet
	}

	let {
		light = false,
		placement = 'bottom',
		documentationLink = undefined,
		small = false,
		disablePopup = false,
		openDelay = 300,
		closeDelay = 0,
		portal = 'body',
		customBgClass = undefined,
		style = '',
		class: className = '',
		children,
		text
	}: Props = $props()

	const {
		elements: { trigger, content },
		states: { open }
	} = createTooltip({
		positioning: {
			placement: untrack(() => placement)
		},
		openDelay: untrack(() => openDelay),
		closeDelay: untrack(() => closeDelay),
		group: true,
		portal: untrack(() => portal)
	})
</script>

<span class={className} {style} use:melt={$trigger}>
	{@render children?.()}
</span>
{#if !children}
	<div
		class="inline-flex w-3 mx-0.5 h-3 {light
			? 'text-primary-inverse'
			: 'text-primary'} {className} "
		use:melt={$trigger}
	>
		<InfoIcon size={small ? 12 : 14} />
	</div>
{/if}

{#if $open && !disablePopup}
	<div use:melt={$content} transition:fade={{ duration: 100 }} style="z-index: {zIndexes.tooltip}">
		<TooltipInner {documentationLink} {customBgClass}>
			{#snippet children()}
				{@render text?.()}
			{/snippet}
		</TooltipInner>
	</div>
{/if}
