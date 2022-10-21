<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import {
		type ProgressStep,
		type ProgressState,
		type LoopState,
		isLoopStep,
		isLoopState
	} from './model'
	import ProgressBarGeneralPart from './ProgressBarGeneralPart.svelte'
	import ProgressBarLoopPart from './ProgressBarLoopPart.svelte'

	export let steps: ProgressStep[]
	export let finished = false
	export let duration = 200
	const percent = tweened(0, {
		duration,
		easing: cubicOut
	})
	let state: ProgressState[] = []

	$: state = steps.map((step, i) => {
		if (isLoopStep(step)) {
			return {
				type: step.type,
				isDone: step.isDone,
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
	$: stepIndex = state.findIndex(({ isDone }) => !isDone)
	$: lastStep = state.at(-1)
	$: if (lastStep?.isDone) finished = true
	$: subStepIndex = lastStep ? lastStep['index'] : undefined
	$: length = 100 / (state.length || 1)
	$: percent.set(finished ? 100 : length * stepIndex)
</script>

<div class={$$props.class}>
	<div class="flex justify-between items-end font-medium text-blue-700 mb-1">
		<span class="text-base">
			{finished
				? 'Done'
				: `Step ${stepIndex + 1}${subStepIndex !== undefined ? `.${subStepIndex + 1}` : ''}`}
		</span>
		<span class="text-sm">
			{state.length ? $percent.toFixed(0) : 0}%
		</span>
	</div>
	<div class="flex w-full bg-gray-200 rounded-full h-4 overflow-hidden">
		{#each state as step, index}
			<div
				class="h-full relative border-white {index === 0 ? '' : 'border-l'}"
				style="width: {length}%;"
			>
				{#if isLoopState(step)}
					<ProgressBarLoopPart {step} {finished} {duration} />
				{:else}
					<ProgressBarGeneralPart {step} {finished} {duration} />
				{/if}
			</div>
		{/each}
	</div>
</div>
