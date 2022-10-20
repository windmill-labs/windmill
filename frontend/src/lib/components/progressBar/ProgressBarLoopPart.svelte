<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import ProgressBarLoopAccessor from './ProgressBarLoopAccessor.svelte'

	export let loopLength: number
	export let loopIndex = 0
	export let isDone: boolean = false
	export let duration = 200

	$: progresses = new Array(loopLength).fill(undefined).map(() => {
		return tweened(isDone ? 100 : 0, {
			duration,
			easing: cubicOut
		})
	})
	$: if (loopIndex) {
		// progresses[loopIndex - 1].set(100)
	}
	$: isDone ? progresses.forEach((p) => p.set(100)) : progresses.forEach((p) => p.set(0))
</script>

{#each progresses as progress, index}
	<ProgressBarLoopAccessor {progress} {index} length={loopLength} />
{/each}
