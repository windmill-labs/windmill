<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { getTween, type GeneralState, type ProgressStateStoreValue } from './model'

	export let step: GeneralState
	export let index: number
	export let duration = 200
	const state = getContext<Writable<ProgressStateStoreValue>>('state')
	let progress = getTween(0, duration)

	$: finishedAndNotLast = $state.finished && $state.length - 1 !== index
	$: if (finishedAndNotLast || $state.index > index) progress = getTween(100, duration)
	$: if ($state.finished || step.isDone) {
		if (index > $state.index) {
			state.update((prev) => ({ ...prev, index }))
		}
		progress.set(100)
	}
</script>

<div
	class="absolute left-0 bottom-0 h-full {$state.error ? 'bg-red-400' : 'bg-blue-400'}"
	style="width: {$progress}%"
/>
