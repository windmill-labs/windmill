<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import { isLoop, type Progress } from './model'
	import ProgressBarGeneralPart from './ProgressBarGeneralPart.svelte'
	import ProgressBarLoopPart from './ProgressBarLoopPart.svelte'

	export let steps: Progress = []
	export let startIndex = 0
	const length = 100 / steps.length
	const percent = tweened(+(length * startIndex).toFixed(0), {
		duration: 400,
		easing: cubicOut
	})
	let currentIndex = startIndex
	let loopIndex = 0
	let isDone = false

	function back() {
		isDone = false
		if (isLoop(currStep)) {
			if (loopIndex <= 0) loopIndex = 0
			else return loopIndex--
		}

		series[currentIndex].isDone = false
		loopIndex = 0
		if (currentIndex <= 0) currentIndex = 0
		else currentIndex--
		series[currentIndex].isDone = false
	}

	function next() {
		if (isLoop(currStep)) {
			const max = series[currentIndex].kind.length - 1
			if (loopIndex >= max) loopIndex = max
			else return loopIndex++
		}

		series[currentIndex].isDone = true
		loopIndex = 0
		if (currentIndex >= steps.length - 1) {
			currentIndex = steps.length - 1
			isDone = true
		} else currentIndex++
	}

	$: currStep = steps[currentIndex]
	$: series = steps.map((step, index) => ({ isDone: index < startIndex, kind: step }))
	$: percent.set(series.map((s) => (s.isDone ? length : 0)).reduce((acc, curr) => acc + curr))
</script>

<div>
	<div class="flex justify-between font-medium text-blue-700 mb-1">
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
	<div class="flex w-full bg-gray-200 rounded-full h-3 overflow-hidden">
		{#each series as step, index}
			{@const isStepDone = isDone || step.isDone}
			<div
				class="h-full relative border-white {index === 0 ? '' : 'border-l'}"
				style="width: {length}%;"
			>
				{#if isLoop(step.kind)}
					<ProgressBarLoopPart isDone={isStepDone} {loopIndex} loopLength={step.kind.length} />
				{:else}
					<ProgressBarGeneralPart isDone={isStepDone} />
				{/if}
			</div>
		{/each}
	</div>
</div>
<div class="flex justify-between">
	<button on:click={back}>back</button>
	<button on:click={next}>next</button>
</div>
