<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import { isLoop, type Progress } from './model'
	import ProgressBarGeneralPart from './ProgressBarGeneralPart.svelte'
	import ProgressBarLoopPart from './ProgressBarLoopPart.svelte'

	export let steps: Progress
	export let startIndex = 0
	export let duration = 200
	let currentIndex = startIndex
	let loopIndex = 0
	let isDone = false

	export function back() {
		isDone = false
		if (isLoop(currStep)) {
			if (loopIndex > 0) {
				loopIndex--
				return
			}
		}

		series[currentIndex].isDone = false
		loopIndex = 0
		if (currentIndex <= 0) {
			currentIndex = 0
		} else {
			currentIndex--
		}
		series[currentIndex].isDone = false
	}

	export function next() {
		if (isLoop(currStep)) {
			const max = series[currentIndex].kind.length - 1
			if (loopIndex < max) {
				loopIndex++
				return
			}
		}

		series[currentIndex].isDone = true
		loopIndex = 0
		if (currentIndex >= steps.length - 1) {
			currentIndex = steps.length - 1
			accumulation = 100
		} else {
			currentIndex++
		}
	}

	export function reset() {
		currentIndex = startIndex
		loopIndex = 0
		isDone = false
		steps = steps
	}

	$: length = 100 / (steps.length || 1)
	$: percent = tweened(+(length * startIndex).toFixed(0), {
		duration,
		easing: cubicOut
	})
	$: currStep = steps[currentIndex]
	$: series = steps.map((step, index) => ({ isDone: index < currentIndex, kind: step }))
	$: accumulation = series.map((s) => (s.isDone ? length : 0)).reduce((acc, curr) => acc + curr)
	$: isDone = accumulation >= 100
	$: percent.set(accumulation > 100 ? 100 : accumulation)
</script>

<div class={$$props.class}>
	<div class="flex justify-between items-end font-medium text-blue-700 mb-1">
		<span class="text-base">
			{isDone
				? 'Done'
				: `Step ${currentIndex + 1}${isLoop(currStep) ? `.${loopIndex + 1}` : ''}: ${
						isLoop(currStep) ? currStep[loopIndex] : currStep
				  }`}
		</span>
		<span class="text-sm">
			{$percent.toFixed(0)}%
		</span>
	</div>
	<div class="flex w-full bg-gray-200 rounded-full h-4 overflow-hidden">
		{#each series as step, index}
			{@const isStepDone = isDone || step.isDone}
			<div
				class="h-full relative border-white {index === 0 ? '' : 'border-l'}"
				style="width: {length}%;"
			>
				{#if isLoop(step.kind)}
					<ProgressBarLoopPart
						isDone={isStepDone}
						{duration}
						{loopIndex}
						loopLength={step.kind.length}
					/>
				{:else}
					<ProgressBarGeneralPart isDone={isStepDone} {duration} />
				{/if}
			</div>
		{/each}
	</div>
</div>
<div class="flex justify-between">
	<button on:click={back}>back</button>
	<button on:click={next}>next</button>
</div>
