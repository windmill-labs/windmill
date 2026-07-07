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
		// 'cursor' anchors the popup to the pointer position where hover started,
		// instead of the trigger element's box. Useful for wide/full-width triggers
		// where an element-anchored tooltip lands far from the cursor.
		anchor?: 'element' | 'cursor'
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
		anchor = 'element',
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

	// Cursor anchoring: floating-ui positions against `reference.getBoundingClientRect()`.
	// melt uses the trigger element as that reference, so we override its rect to a
	// zero-size box at the pointer. The coords are frozen while the tooltip is open so
	// it stays put (letting the pointer travel into the popup to reach the copy button);
	// they only track the pointer while closed, capturing where the next open will land.
	let triggerEl = $state<HTMLElement | undefined>(undefined)
	let cursorX = 0
	let cursorY = 0
	// Until a pointer is seen, fall back to the element rect so keyboard-focus opens
	// don't land the popup at (0, 0).
	let hasCursor = false
	$effect(() => {
		if (anchor !== 'cursor' || !triggerEl) return
		const el = triggerEl
		// Listeners added imperatively (not `onpointermove` attrs) so the static span
		// keeps no interaction handlers, avoiding an a11y_no_static_element warning.
		const track = (e: PointerEvent) => {
			if ($open) return
			cursorX = e.clientX
			cursorY = e.clientY
			hasCursor = true
		}
		el.addEventListener('pointerenter', track)
		el.addEventListener('pointermove', track)
		const original = el.getBoundingClientRect.bind(el)
		el.getBoundingClientRect = () =>
			hasCursor
				? ({
						width: 0,
						height: 0,
						x: cursorX,
						y: cursorY,
						top: cursorY,
						left: cursorX,
						right: cursorX,
						bottom: cursorY,
						toJSON() {}
					} as DOMRect)
				: original()
		return () => {
			el.removeEventListener('pointerenter', track)
			el.removeEventListener('pointermove', track)
			el.getBoundingClientRect = original
		}
	})
</script>

<span bind:this={triggerEl} class={className} {style} use:melt={$trigger}>
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
