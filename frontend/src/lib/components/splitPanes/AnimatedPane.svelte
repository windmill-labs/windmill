<script lang="ts">
	import { type ComponentProps } from 'svelte'
	import { Pane } from 'svelte-splitpanes'
	import { cubicInOut } from 'svelte/easing'

	let {
		duration = 300,
		easing = cubicInOut,
		size,
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

	let userChangedSize: number | undefined = $state(undefined)

	let t = $state(opened ? 1 : 0)
	let computedSize = $derived(easing(t) * (userChangedSize ?? size))

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

<Pane {...props} bind:size={() => computedSize, (v) => (userChangedSize = v)}>
	{@render children()}
</Pane>
