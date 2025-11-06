<script lang="ts" module>
	let CURRENT_RESIZE_TRANSITION_WRAPPER_CONTEXT = 'CURRENT_RESIZE_TRANSITION_WRAPPER_CONTEXT'
</script>

<script lang="ts">
	import { getContext, setContext } from 'svelte'

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

	let currentResizeTransitionWrapper =
		getContext<boolean>(CURRENT_RESIZE_TRANSITION_WRAPPER_CONTEXT) ?? false
	setContext(CURRENT_RESIZE_TRANSITION_WRAPPER_CONTEXT, true)

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

<!-- Prevent nesting this component to avoid weird behavior -->
{#if currentResizeTransitionWrapper}
	{@render children()}
{:else}
	<div bind:this={outerContainer} {...outerDivProps} class={twMerge('relative', className)} {style}>
		<div class={twMerge('absolute', innerClass)} bind:this={innerContainer}>
			{@render children()}
		</div>
	</div>
{/if}
