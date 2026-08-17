<script lang="ts">
	import type { Snippet } from 'svelte'

	let {
		color,
		class: className = '',
		children
	}: {
		// Rail background, so the overflow fades blend into it rather than sitting
		// as visible bars.
		color: string
		class?: string
		children: Snippet
	} = $props()

	let viewport = $state<HTMLElement | undefined>(undefined)
	let overflowTop = $state(false)
	let overflowBottom = $state(false)

	function measure() {
		const el = viewport
		if (!el) return
		overflowTop = el.scrollTop > 1
		overflowBottom = Math.ceil(el.scrollTop + el.clientHeight) < el.scrollHeight - 1
	}

	$effect(() => {
		const el = viewport
		if (!el) return
		measure()
		// Re-measure when the viewport or its content changes height — menus
		// expanding, sessions loading, the rail being resized — not just on scroll.
		const ro = new ResizeObserver(measure)
		ro.observe(el)
		for (const child of Array.from(el.children)) ro.observe(child)
		return () => ro.disconnect()
	})
</script>

<div class={'relative flex-1 min-h-0 ' + className}>
	<div
		bind:this={viewport}
		onscroll={measure}
		class="sidebar-scroll h-full overflow-y-auto overflow-x-hidden scrollbar-hidden flex flex-col gap-3"
	>
		{@render children()}
	</div>
	<!-- Overflow hints: a short fade to the rail colour that only shows when there
	     is content past that edge. pointer-events-none so it never eats clicks. -->
	<div
		class="pointer-events-none absolute inset-x-0 top-0 h-4 transition-opacity duration-150"
		style:opacity={overflowTop ? 1 : 0}
		style:background="linear-gradient(to bottom, {color}, transparent)"
	></div>
	<div
		class="pointer-events-none absolute inset-x-0 bottom-0 h-6 transition-opacity duration-150"
		style:opacity={overflowBottom ? 1 : 0}
		style:background="linear-gradient(to top, {color}, transparent)"
	></div>
</div>

<style>
	/* Sections keep their natural height so the column scrolls as one block
	   instead of flex-shrinking each section (which would overlap their rows). */
	.sidebar-scroll > :global(*) {
		flex-shrink: 0;
	}
</style>
