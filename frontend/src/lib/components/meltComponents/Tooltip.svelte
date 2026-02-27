<script lang="ts">
	import { untrack } from 'svelte'
	import type { Placement } from '@floating-ui/core'
	import { InfoIcon } from 'lucide-svelte'
	import { zIndexes } from '$lib/zIndexes'

	import { createTooltip, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import TooltipInner from '../TooltipInner.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		light?: boolean
		placement?: Placement | undefined
		documentationLink?: string | undefined
		small?: boolean
		markdownTooltip?: string | undefined
		disablePopup?: boolean
		/** When true, the trigger renders as a non-interactive span instead of a button */
		notClickable?: boolean
		openDelay?: number
		closeDelay?: number
		/** @deprecated Use `closeDelay` instead. Controls the delay before tooltip disappears. */
		disappearTimeout?: number
		portal?: string | undefined | null
		customBgClass?: string | undefined
		/** Additional classes for the tooltip content container */
		containerClasses?: string | undefined
		/** Show a close button on the tooltip */
		closeButton?: boolean
		class?: string
		style?: string
		onclick?: (e: MouseEvent) => void
		children?: Snippet
		text?: Snippet
	}

	let {
		light = false,
		placement = 'bottom',
		documentationLink = undefined,
		small = false,
		markdownTooltip = undefined,
		disablePopup = false,
		notClickable: _notClickable = false,
		openDelay = 300,
		closeDelay: closeDelayProp = 0,
		disappearTimeout: disappearTimeoutProp = undefined,
		portal = 'body',
		customBgClass = undefined,
		containerClasses = undefined,
		closeButton: _closeButton = false,
		class: className = undefined,
		style: styleProp = undefined,
		onclick: onclickProp = undefined,
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
		closeDelay: untrack(() => disappearTimeoutProp) ?? untrack(() => closeDelayProp),
		group: true,
		portal: untrack(() => portal)
	})
</script>

{#if children}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span class={className} style={styleProp} use:melt={$trigger} onclick={onclickProp}>
		{@render children()}
	</span>
{:else}
	<div
		class="inline-flex w-3 mx-0.5 h-3 {light
			? 'text-primary-inverse'
			: 'text-primary'} {className ?? ''} "
		use:melt={$trigger}
	>
		<InfoIcon size={small ? 12 : 14} />
	</div>
{/if}

{#if $open && !disablePopup}
	<div use:melt={$content} transition:fade={{ duration: 100 }} style="z-index: {zIndexes.tooltip}">
		<TooltipInner {documentationLink} {markdownTooltip} customBgClass={containerClasses ?? customBgClass}>
			{@render text?.()}
		</TooltipInner>
	</div>
{/if}
