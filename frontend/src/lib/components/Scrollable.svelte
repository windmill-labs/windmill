<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	let isAtBottom: boolean = $state(false)
	let isScrollable = $state(false)

	interface Props {
		id?: string | null | undefined
		scrollableClass?: string
		shiftedShadow?: boolean
		children?: import('svelte').Snippet
	}

	let { id = undefined, scrollableClass = '', shiftedShadow = false, children }: Props = $props()
	let mutationObserver: MutationObserver
	let el: HTMLDivElement | undefined = $state()

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
		el?.scrollTo({ top, behavior: 'smooth' })
	}
	onMount(() => {
		observeScrollability(el)
	})

	onDestroy(() => {
		mutationObserver?.disconnect()
	})
</script>

<div {id} class={twMerge('relative pb-1', scrollableClass)}>
	<div bind:this={el} onscroll={handleScroll} class="w-full h-full overflow-y-auto">
		{@render children?.()}
	</div>
	{#if !isAtBottom && isScrollable}
		<div
			class="pointer-events-none absolute bottom-0 {shiftedShadow
				? 'left-2'
				: 'right-0'} h-14 w-full bg-gradient-to-t from-surface to-transparent"
		></div>
	{/if}
</div>
