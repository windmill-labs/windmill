<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import ProgressBarLoopAccessor from './ProgressBarLoopAccessor.svelte'
	import type { LoopState } from './model'
	import { get } from 'svelte/store'

	export let step: LoopState
	export let finished = false
	export let duration = 200
	const progresses = Array.from({ length: step.length }, () => {
		return tweened(0, {
			duration,
			easing: cubicOut
		})
	})

	$: if (step.indexChanged) {
		progresses.filter((p, i) => get(p) === 0 && i < step.index).forEach((p) => p.set(100))
	}
	$: if (finished) progresses.forEach((p) => p.set(100))
</script>

{#each progresses as progress, index}
	<ProgressBarLoopAccessor {progress} {index} length={step.length} />
{/each}
