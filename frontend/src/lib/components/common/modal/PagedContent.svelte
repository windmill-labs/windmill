<script lang="ts" module>
	import type { Snippet } from 'svelte'

	/** One level of a paginated dialog. Order is the order given: the page on screen sits at rest
	 *  and every other waits off the side it is listed on, so a deeper page arrives from the right
	 *  and the way back arrives from the left without anyone naming a direction. */
	export type ModalPage = { key: string; content: Snippet }
</script>

<script lang="ts">
	let {
		pages,
		current,
		class: c = ''
	}: {
		pages: ModalPage[]
		/** The `key` of the page on screen. */
		current: string
		class?: string
	} = $props()

	/** A page is mounted the first time it is opened and never unmounted again, which is the whole
	 *  point of this over an `{#if}`: a page keeps its state, its loaded data and its scroll
	 *  position while another is on screen, and returning to it costs nothing. A page never
	 *  visited is never built. */
	let visited = $state<string[]>([])
	$effect(() => {
		if (!visited.includes(current)) visited = [...visited, current]
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
<div class="relative overflow-hidden {c}">
	{#each pages as page, i (page.key)}
		{#if visited.includes(page.key)}
			<!-- Pages are laid over each other rather than laid out, so the dialog cannot change
			     height as one replaces another. That needs a definite height from the caller.
			     `inert` and not just opacity: an off-screen page keeps its DOM, so without it the
			     tab order and every screen reader still walk through it. -->
			<div
				class="absolute inset-0 flex flex-col transition duration-150 ease-out motion-reduce:transition-none"
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
