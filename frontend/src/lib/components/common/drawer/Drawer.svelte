<script lang="ts">
	import { onMount, createEventDispatcher, untrack } from 'svelte'
	import { BROWSER } from 'esm-env'
	import Disposable from './Disposable.svelte'
	import ConditionalPortal from './ConditionalPortal.svelte'
	import { chatState } from '$lib/components/copilot/chat/sharedChatState.svelte'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { getOverlayHost } from '../overlayHost.svelte'

	interface Props {
		open?: boolean
		duration?: number
		placement?: string
		size?: string
		/** Pixel floor for a percentage `size`, so the drawer stays usable in a narrow
		 *  container. Never grows past the container itself. */
		minSize?: string
		alwaysOpen?: boolean
		shouldUsePortal?: boolean
		offset?: number
		preventEscape?: boolean
		disableChatOffset?: boolean
		class?: string | undefined
		positionClass?: string | undefined
		name?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		open = $bindable(undefined),
		duration: _duration = 0.3,
		placement = 'right',
		size = '600px',
		minSize = '600px',
		alwaysOpen = false,
		shouldUsePortal = true,
		offset = 0,
		preventEscape = false,
		disableChatOffset = false,
		class: clazz = '',
		positionClass = undefined,
		name = undefined,
		children: children_render
	}: Props = $props()

	if (open === undefined) {
		open = false
	}

	let disposable: Disposable | undefined = $state(undefined)

	let reducedMotion = useReducedMotion()
	let duration = $derived(reducedMotion.val ? 0 : _duration)
	let durationMs = $derived(duration * 1000)

	export function toggleDrawer() {
		disposable?.toggleDrawer()
	}

	export function openDrawer() {
		disposable?.openDrawer()
	}

	export function closeDrawer() {
		if (open) {
			setTimeout(() => {
				dispatch('afterClose')
			}, durationMs)
		}
		disposable?.closeDrawer()
	}

	export function isOpen() {
		return open
	}

	let mounted = false
	const dispatch = createEventDispatcher()

	// A percentage size follows its container, which — anchored in a pane rather than the
	// viewport — can leave the drawer too narrow to use. Pixel sizes already state their
	// intent, so only percentages take the floor.
	const floor = $derived(size.trim().endsWith('%') ? `min(${minSize}, 100%)` : '0px')

	let style = $derived(`--duration: ${duration}s; --size: ${size}; --min-size: ${floor};`)

	function scrollLock(open: boolean) {
		if (BROWSER) {
			const body = document.querySelector('body')

			if (mounted && body) {
				body.style.overflowY = open ? 'hidden' : 'auto'
			}
		}
	}

	$effect(() => {
		scrollLock(open ?? false)
	})

	$effect(() => {
		open
		untrack(() => {
			open ? openDrawer() : closeDrawer()
		})
	})

	let timeout = $state(true)
	$effect(() => {
		!open ? setTimeout(() => (timeout = true), durationMs) : (timeout = false)
	})
	onMount(() => {
		mounted = true
	})

	// An enclosing pane can claim the drawer (see overlayHost): it is then portalled
	// into that element and positioned against it rather than the viewport. The
	// global-chat offset is the viewport's business, so it doesn't apply there.
	const overlayHost = getOverlayHost()
	const host = $derived(shouldUsePortal ? overlayHost?.el() : undefined)
	const posClass = $derived(positionClass ?? (host ? '!absolute' : undefined))

	const aiChatOpen = $derived(chatState.size > 0 && !host)
</script>

<!-- `contents` keeps the portal's wrapper from becoming a stray flex item of the
     host it is appended to; the drawer inside positions against the host itself. -->
<ConditionalPortal condition={shouldUsePortal} target={host} class={host ? 'contents' : undefined}>
	<Disposable
		initialOffset={offset}
		bind:open
		bind:this={disposable}
		onOpen={() => dispatch('open')}
		onClose={() => dispatch('close')}
		{preventEscape}
	>
		{#snippet children({ handleClickAway, zIndex, isTop })}
			<aside
				class="drawer windmill-app windmill-drawer {name ? `windmill-drawer-${name}` : ''} {clazz ??
					''} {posClass ?? ''} {aiChatOpen ? 'respect-global-chat' : ''}"
				class:open
				class:close={!open && timeout}
				class:global-chat-open={aiChatOpen}
				style={`${style}; --zIndex: ${zIndex}; --adjusted-offset: calc(${aiChatOpen && placement === 'right' && !disableChatOffset ? chatState.size : 0}% + 4px)`}
			>
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="overlay {posClass ?? ''}" onclick={handleClickAway}></div>
				<div class="panel {placement} {posClass ?? ''}" class:size>
					{#if open || !timeout || alwaysOpen}
						{@render children_render?.({ open, isTop })}
					{/if}
				</div>
			</aside>
		{/snippet}
	</Disposable>
</ConditionalPortal>

<style lang="postcss">
	.drawer {
		position: fixed;
		top: 0;
		left: 0;
		height: 100%;
		width: 100%;
		z-index: -1;
		transition: z-index var(--duration) step-end;
		overflow: clip;
		pointer-events: none;
	}

	.drawer.open {
		height: 100%;
		z-index: var(--zIndex);
		right: 0;
		width: calc(100% - var(--adjusted-offset));
		transition: z-index var(--duration) step-start;
		pointer-events: auto;
	}

	.overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.5);
		opacity: 0;
		z-index: 2;
		transition: opacity var(--duration) ease;
	}

	.drawer.respect-global-chat.global-chat-open > .overlay {
		width: 100%;
		right: var(--adjusted-offset);
		left: auto;
	}

	.drawer.open > .overlay {
		opacity: 1;
	}

	.drawer.close > .panel {
		height: 0;
		/* The size floor must not keep a closed panel expanded. */
		min-height: 0;
		overflow: hidden;
	}

	.panel {
		position: fixed;
		width: 100%;
		@apply bg-surface;
		z-index: 3;
		transition:
			transform var(--duration) ease,
			max-width var(--duration) ease,
			max-height var(--duration) ease;
		height: 100%;
	}

	.panel.left {
		left: 0;
		transform: translate(-100%, 0);
	}

	.panel.right {
		right: 0;
		transform: translate(100%, 0);
	}

	.drawer.respect-global-chat.global-chat-open > .panel.right {
		right: var(--adjusted-offset);
		width: calc(100vw - var(--adjusted-offset));
	}

	.panel.top {
		top: 0;
		transform: translate(0, -100%);
	}

	.panel.bottom {
		bottom: 0;
		transform: translate(0, 100%);
	}

	.panel.left.size,
	.panel.right.size {
		max-width: var(--size);
		min-width: var(--min-size);
	}

	.panel.top.size,
	.panel.bottom.size {
		max-height: var(--size);
		min-height: var(--min-size);
	}

	.drawer.open > .panel {
		transform: translate(0, 0);
	}
</style>
