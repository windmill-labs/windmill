<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements'
	import { twMerge } from 'tailwind-merge'

	type Props = {
		children: import('svelte').Snippet
		innerClass?: string
		class?: string
		horizontal?: boolean
		vertical?: boolean
		outerDivProps?: HTMLAttributes<HTMLDivElement>
	}
	let {
		children,
		innerClass,
		class: className = '',
		horizontal,
		vertical,
		outerDivProps
	}: Props = $props()

	let innerContainer: HTMLElement | null = $state(null)
	let outerContainer: HTMLElement | null = $state(null)

	let width: number | null = $state(null)
	let height: number | null = $state(null)

	let style = $derived.by(() => {
		let s = 'transition: width 0.2s, height 0.2s; overflow: clip;'
		if (width !== null) s += `width: ${width}px;`
		if (height !== null) s += `height: ${height}px;`
		return s
	})

	$effect(() => {
		if (!innerContainer || !outerContainer) return
		let observer = new ResizeObserver((entries) => {
			for (const entry of entries) {
				if (horizontal) width = entry.contentRect.width
				if (vertical) height = entry.contentRect.height
			}
		})
		observer.observe(innerContainer)
		return () => {
			if (innerContainer) observer.unobserve(innerContainer)
			observer.disconnect()
		}
	})
</script>

<div bind:this={outerContainer} {...outerDivProps} class={className} {style}>
	<div class={twMerge('absolute', innerClass)} bind:this={innerContainer}>
		{@render children()}
	</div>
</div>
