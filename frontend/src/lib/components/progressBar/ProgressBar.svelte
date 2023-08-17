<script lang="ts">
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import {
		type ProgressStep,
		type ProgressState,
		type LoopState,
		isLoopStep,
		isLoopState,
		getTween,
		type ProgressStateStoreValue
	} from './model'
	import ProgressBarGeneralPart from './ProgressBarGeneralPart.svelte'
	import ProgressBarLoopPart from './ProgressBarLoopPart.svelte'

	export let steps: ProgressStep[]
	export let error = false
	export let duration = 100
	let percent = getTween(0, duration)
	let finished = false
	let state: ProgressState[] = []

	const stateStore = writable<ProgressStateStoreValue>({
		length: state.length,
		index: 0,
		finished,
		error
	})
	setContext('state', stateStore)

	$: state = steps.map((step, i) => {
		if (isLoopStep(step)) {
			return {
				type: step.type,
				isDone: error ? false : step.isDone,
				isDoneChanged: !state[i]?.isDone && step.isDone,
				length: step.length,
				index: step.index,
				indexChanged: (state[i] as LoopState)?.index !== step.index
			}
		} else {
			return {
				type: step.type,
				isDone: step.isDone,
				isDoneChanged: !state[i]?.isDone && step.isDone
			}
		}
	})
	$: stateStore.update(({ index }) => ({ length: state.length, index, finished, error }))
	$: stepIndex = state.findIndex(({ isDone }) => !isDone)
	$: lastStep = state.at(-1)
	$: if (typeof lastStep?.isDone === 'boolean') {
		finished = lastStep.isDone
		if ($percent >= 100 && !lastStep.isDone) {
			percent = getTween(0, duration)
		}
	}
	$: subStepIndex = lastStep ? lastStep['index'] : undefined
	$: length = 100 / (state.length || 1)
	$: if (finished) {
		percent.set(100)
	} else {
		const product = length * stepIndex
		percent.set(product < 0 ? 0 : product)
	}
</script>

<div class={$$props.class}>
	<div
		class="flex justify-between items-end font-medium mb-1 {error
			? 'text-red-700 dark:text-red-200'
			: 'text-blue-700 dark:text-blue-200'}"
	>
		<span class="text-base">
			{error
				? 'Error occured'
				: finished
				? 'Done'
				: `Step ${stepIndex + 1}${subStepIndex !== undefined ? `.${subStepIndex + 1}` : ''}`}
		</span>
		<span class="text-sm">
			{state.length ? $percent.toFixed(0) : 0}%
		</span>
	</div>
	<!-- {#each state as step, index}
		{index} {JSON.stringify(step)}
	{/each} -->
	<div class="flex w-full bg-gray-200 rounded-full h-4 overflow-hidden">
		{#each state as step, index}
			<div
				class="h-full relative border-white {index === 0 ? '' : 'border-l'}"
				style="width: {length}%;"
			>
				{#if isLoopState(step)}
					<ProgressBarLoopPart {step} {index} {duration} />
				{:else}
					<ProgressBarGeneralPart {step} {index} {duration} />
				{/if}
			</div>
		{/each}
	</div>
</div>
