<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	let isAtBottom: boolean = false
	let isScrollable = false

	export let id: string | null | undefined = undefined
	export let scrollableClass: string = ''
	export let shiftedShadow: boolean = false
	let mutationObserver: MutationObserver
	let el: HTMLDivElement

	function handleScroll(event) {
		const scrollableElement = event.target

		isAtBottom =
			scrollableElement.scrollTop + scrollableElement.offsetHeight >=
			scrollableElement.scrollHeight - 2
	}

	function checkIfScrollable(el) {
		return el.scrollHeight > el.clientHeight
	}

	function observeScrollability(el) {
		isScrollable = checkIfScrollable(el)

		mutationObserver = new MutationObserver(() => {
			isScrollable = checkIfScrollable(el)
		})
		mutationObserver?.observe(el, { childList: true, subtree: true, characterData: true })
	}

	export function scrollIntoView(top: number) {
		el.scrollTo({ top, behavior: 'smooth' })
	}
	onMount(() => {
		observeScrollability(el)
	})

	onDestroy(() => {
		mutationObserver?.disconnect()
	})
</script>

<div {id} class={twMerge('relative pb-1', scrollableClass)}>
	<div bind:this={el} on:scroll={handleScroll} class="w-full h-full overflow-y-auto">
		<slot />
	</div>
	{#if !isAtBottom && isScrollable}
		<div
			class="pointer-events-none absolute bottom-0 {shiftedShadow
				? 'left-2'
				: 'right-0'} h-14 w-full bg-gradient-to-t from-surface to-transparent"
		></div>
	{/if}
</div>
