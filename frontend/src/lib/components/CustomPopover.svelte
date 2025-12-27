<script lang="ts">
	import { run, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from '$lib/components/Portal.svelte'

	interface Props {
		placement?: PopoverPlacement
		notClickable?: boolean
		disablePopup?: boolean
		disappearTimeout?: number
		appearTimeout?: number
		style?: string | undefined
		noPadding?: boolean
		focusEl?: HTMLElement | undefined
		class?: string
		children?: import('svelte').Snippet
		overlay?: import('svelte').Snippet
	}

	let {
		placement = 'bottom-end',
		notClickable = false,
		disablePopup = false,
		disappearTimeout = 100,
		appearTimeout = 300,
		style = undefined,
		noPadding = false,
		focusEl = undefined,
		class: clazz = '',
		children,
		overlay
	}: Props = $props()
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

	let showTooltip = $state(false)
	let timeout: number | undefined = undefined
	let inTimeout: number | undefined = undefined

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

	run(() => {
		focusEl && focusEl?.focus()
	})
</script>

{#if notClickable}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span {style} use:popperRef onmouseenter={open} onmouseleave={close} class={clazz}>
		{@render children?.()}
	</span>
{:else}
	<button
		{style}
		use:popperRef
		onmouseenter={open}
		onmouseleave={close}
		onclick={bubble('click')}
		class={clazz}
	>
		{@render children?.()}
	</button>
{/if}
{#if showTooltip && !disablePopup}
	<Portal name="custom-popover">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			use:popperContent={popperOptions}
			onmouseenter={open}
			onmouseleave={close}
			class="z-[5001] border rounded-lg shadow-lg {noPadding ? '' : 'p-4'} bg-surface"
		>
			{@render overlay?.()}
		</div>
	</Portal>
{/if}
