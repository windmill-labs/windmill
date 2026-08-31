<script lang="ts" module>
	import type { Snippet } from 'svelte'

	/** One level of a paginated dialog. Order is the order given: the page on screen sits at rest
	 *  and every other waits off the side it is listed on, so a deeper page arrives from the right
	 *  and the way back arrives from the left without anyone naming a direction. */
	export type ModalPage = { key: string; content: Snippet }
</script>

<script lang="ts">
	import { overlayHostActive, topmostSurface } from '$lib/components/common/overlayHost.svelte'

	let {
		pages,
		current,
		onNavigate,
		warm = false,
		class: c = ''
	}: {
		pages: ModalPage[]
		/** The `key` of the page on screen. */
		current: string
		/**
		 * Go to a page. Given, the component answers the left and right arrow keys, stepping between
		 * pages the way the surface's own controls do. Escape is deliberately not taken: the dialog
		 * around this owns it, and a page component that swallowed it would stop the dialog from
		 * closing. Without this prop the pages are still navigable, just not from the keyboard —
		 * the caller owns `current` either way.
		 */
		onNavigate?: (key: string) => void
		/**
		 * Build every page once this one has settled, rather than each on its first visit. Turn it
		 * on where a page is heavy enough that building it costs a frame: otherwise that build
		 * lands on the first click, inside the transition, and only the first navigation stutters.
		 * The cost moves to a moment when nothing is animating and nobody is waiting.
		 */
		warm?: boolean
		class?: string
	} = $props()

	/** A page is mounted the first time it is opened and never unmounted again, which is the whole
	 *  point of this over an `{#if}`: a page keeps its state, its loaded data and its scroll
	 *  position while another is on screen, and returning to it costs nothing. */
	let visited = $state<string[]>([])
	$effect(() => {
		if (!visited.includes(current)) visited = [...visited, current]
	})

	/** Whether the rest of the pages have been built. Deferred a frame past mount rather than done
	 *  outright: the page being opened *now* is what the user is waiting on, and building its
	 *  neighbours in the same frame would make opening the dialog pay for them. */
	let warmed = $state(false)
	$effect(() => {
		if (!warm || warmed) return
		const frame = requestAnimationFrame(() => (warmed = true))
		return () => cancelAnimationFrame(frame)
	})

	let box: HTMLDivElement | undefined = $state()

	// A window listener answers keys aimed anywhere, so it has to ask two questions the DOM cannot:
	// is my host the visible one — session preview tabs stay mounted when hidden — and is my surface
	// still the one on top, rather than under a drawer or a dialog opened since.
	const hostActive = overlayHostActive()
	const onTop = topmostSurface()
	const listening = () => hostActive() && onTop()

	/** Whether the keyboard is ours to answer.
	 *
	 *  Deliberately not "focus is inside the pages". Navigating leaves focus wherever the page that
	 *  was left put it — the body, once `inert` lands on it, or a button in the surrounding dialog
	 *  chrome — so a containment test answers the first press and refuses every one after it. What
	 *  is actually being asked is whether the key is meant for something else, and only a control
	 *  that reads arrows itself qualifies. */
	function ownsKeyboard(target: EventTarget | null): boolean {
		if (!box || !box.isConnected) return false
		const el = target as HTMLElement | null
		return !el?.closest?.('input, textarea, select, [contenteditable="true"], [role="listbox"]')
	}

	function onKeydown(event: KeyboardEvent) {
		if (!onNavigate || !listening() || event.metaKey || event.ctrlKey || event.altKey) return
		if (!ownsKeyboard(event.target)) return
		const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
		if (step === 0) return
		const next = pages[index + step]
		if (!next) return
		event.preventDefault()
		// Asked before navigating: `inert` lands on the page being left and takes focus with it.
		restoreFocus = !!box?.contains(document.activeElement)
		onNavigate(next.key)
	}

	/** Whether the arriving page should take focus: set by a keypress that had focus inside the
	 *  pages, so a key aimed at the dialog's own chrome leaves focus where it was. Plain state, not
	 *  `$state`: the effect below is keyed on the page, and reading this must not key it too. */
	let restoreFocus = false

	/** Focus follows the page, or a keyboard user is dropped out of the dialog on the first step:
	 *  `inert` on the page being left resets focus to the body. Deferred a frame and unconditional,
	 *  because that reset lands *after* this effect runs — testing `activeElement` here reads the
	 *  element the user is about to lose and does nothing. */
	$effect(() => {
		current
		if (!restoreFocus) return
		restoreFocus = false
		const frame = requestAnimationFrame(() => {
			box?.querySelector<HTMLElement>('.paged-content-page:not(.is-hidden)')?.focus({
				preventScroll: true
			})
		})
		return () => cancelAnimationFrame(frame)
	})

	/** How far a page travels, as a share of the box. Deliberately a fraction and not the whole
	 *  width: at 100% a page has left the box before its opacity has gone anywhere, so the fade is
	 *  spent off-screen and all that is left to see is a slide. */
	const TRAVEL_PERCENT = 24

	let index = $derived(
		Math.max(
			pages.findIndex((p) => p.key === current),
			0
		)
	)
</script>

<!-- The clipper. `overflow` clips descendants and not an element's own transform, so the box that
     hides a page off the side can never be the page that travels. -->
<svelte:window onkeydown={onKeydown} />

<div bind:this={box} class="relative overflow-hidden {c}">
	{#each pages as page, i (page.key)}
		{#if visited.includes(page.key) || warmed}
			<!-- Pages are laid over each other rather than laid out, so the dialog cannot change
			     height as one replaces another. That needs a definite height from the caller.
			     `inert` and not just opacity: an off-screen page keeps its DOM, so without it the
			     tab order and every screen reader still walk through it. -->
			<!-- `tabindex="-1"`: focusable programmatically, never a tab stop of its own. -->
			<div
				tabindex="-1"
				class="paged-content-page absolute inset-0 flex flex-col"
				class:is-hidden={i !== index}
				class:opacity-0={i !== index}
				class:pointer-events-none={i !== index}
				style="transform: translateX({(i - index) * TRAVEL_PERCENT}%)"
				inert={i !== index}
			>
				{@render page.content()}
			</div>
		{/if}
	{/each}
</div>

<style>
	/* Arriving: slides in and fades up. A transition reads the duration of the state it is moving
	   *to*, which is what lets the two directions differ here. */
	.paged-content-page {
		transition-property: transform, opacity;
		transition-duration: 130ms, 90ms;
		transition-timing-function: ease-out;
	}

	/* Leaving: gone at once. Both pages share the box while one replaces the other, so an outgoing
	   page is painted over the incoming one for as long as it still has any opacity — any fade at
	   all is a ghost laid over the page you are trying to look at. It still travels, which costs
	   nothing to watch because it is already invisible. */
	.paged-content-page.is-hidden {
		transition-duration: 130ms, 0s;
	}

	@media (prefers-reduced-motion: reduce) {
		.paged-content-page {
			transition: none;
		}
	}
</style>
