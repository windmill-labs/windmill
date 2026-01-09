<script lang="ts">
	import { type ComponentProps } from 'svelte'
	import { Pane } from 'svelte-splitpanes'
	import { cubicOut } from 'svelte/easing'

	let {
		duration = 300,
		easing = cubicOut,
		size = $bindable(),
		opened,
		children,
		...props
	}: ComponentProps<typeof Pane> & {
		duration?: number
		easing?: (t: number) => number
		opened: boolean
		children?: import('svelte').Snippet
		size: number
	} = $props()

	let t = $state(opened ? 1 : 0)
	let computedSize = $derived((opened ? easing(t) : 1 - easing(1 - t)) * size)

	let lastTValue = 0
	let loopIsRunning = false

	function animationCallback(tValue: number) {
		let delta = tValue - lastTValue
		if (opened) t = Math.min(1, t + delta / duration)
		else t = Math.max(0, t - delta / duration)

		if ((t === 0 && !opened) || (t === 1 && opened)) {
			loopIsRunning = false
			return
		}

		lastTValue = tValue
		requestAnimationFrame(animationCallback)
	}

	$effect(() => {
		opened
		if (!loopIsRunning) {
			loopIsRunning = true
			requestAnimationFrame((tValue) => {
				lastTValue = tValue
				requestAnimationFrame(animationCallback)
			})
		}
	})
</script>

{#if computedSize > 1}
	<Pane {...props} bind:size={() => computedSize, (v) => (size = v)}>
		{@render children()}
	</Pane>
{/if}
