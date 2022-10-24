<script lang="ts">
	import { getContext } from 'svelte'
	import { get, type Writable } from 'svelte/store'
	import ProgressBarLoopAccessor from './ProgressBarLoopAccessor.svelte'
	import { getTween, type LoopState, type ProgressStateStoreValue } from './model'

	export let step: LoopState
	export let index: number
	export let duration = 200
	const state = getContext<Writable<ProgressStateStoreValue>>('state')
	let progresses = getTweenArray()

	$: finishedAndNotLast = $state.finished && $state.length - 1 !== index
	$: if (finishedAndNotLast && !$state.error) {
		progresses = getTweenArray(100)
	}
	$: if (step.indexChanged && !$state.error) {
		progresses.filter((p, i) => get(p) === 0 && i < step.index).forEach((p) => p.set(100))
	}
	$: if ($state.finished && !$state.error) progresses.forEach((p) => p.set(100))

	function getTweenArray(initial = 0) {
		return Array.from({ length: step.length }, () => getTween(initial, duration))
	}
</script>

{#each progresses as progress, i}
	<ProgressBarLoopAccessor {progress} index={i} length={step.length} />
{/each}
