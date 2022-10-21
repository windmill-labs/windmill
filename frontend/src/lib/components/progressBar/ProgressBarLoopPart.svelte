<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import ProgressBarLoopAccessor from './ProgressBarLoopAccessor.svelte'

	export let loopLength: number
	export let loopIndex = 0
	export let isDone: boolean = false
	export let duration = 200

	$: progresses = Array.from({ length: loopLength }, (_, i) => {
		return tweened(isDone || i < loopIndex - 1 ? 100 : 0, {
			duration,
			easing: cubicOut
		})
	})
	$: if (loopIndex) {
		progresses[loopIndex - 1].set(100)
	}
</script>

{#each progresses as progress, index}
	<ProgressBarLoopAccessor {progress} {index} length={loopLength} />
{/each}
