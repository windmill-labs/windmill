<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { linear } from 'svelte/easing'
	import { twMerge } from 'tailwind-merge'

	function getTween(initialValue = 0, duration = 200) {
		return tweened(initialValue, {
			duration,
			easing: linear
		})
	}

	export let error: number | undefined = undefined
	export let index: number
	export let subIndex: number | undefined
	export let subLength: number | undefined
	export let nextInProgress: boolean = false
	// Used for displaying progress of subjob of flow
	export let subIndexIsPercent: boolean = false
	// Remove padding/margin, border radius and titles
	// Used in individual job test runs
	export let compact: boolean = false
	// Removes `Step 1` and replaces it with `Running` 
	export let hideStepTitle: boolean = false

	export let length: number
	let duration = 200

	let percent = getTween(0, duration)

	export function resetP() {
		percent = getTween(0, duration)
	}

	$: percent.set(
		(length
			? index / length + (subIndex && subLength ? subIndex / (subLength ?? 1) / length : 0)
			: 0) * 100
	)

	function getPercent(partIndex: number, _pct: number) {
		if (!length) {
			return 0
		}

		const res = Math.min(($percent - (partIndex / length) * 100) * length, 100)
		return res
	}

	$: finished = index == length
</script>

<div class={$$props.class}>
	{#if !compact}
	<div
		class="flex justify-between items-end font-medium mb-1 {error != undefined
			? 'text-red-700 dark:text-red-200'
			: 'text-blue-700 dark:text-blue-200'}"
	>
		<span class="text-base">
			{error != undefined
				? 'Error occured'
				: finished
				? 'Done'
				: hideStepTitle 
				? `Running`
				: subIndexIsPercent
				? `Step ${index + 1} (${subIndex !== undefined ? `${subIndex}%)` : ''}`
				: `Step ${index + 1}${subIndex !== undefined ? `.${subIndex + 1}` : ''}`}
		</span>
		<span class="text-sm">
			{$percent.toFixed(0)}%
		</span>
	</div>
	{/if}
	<!-- {#each state as step, index}
		{index} {JSON.stringify(step)}
	{/each} -->
	<!-- A: {index}, B: {length}
	{#each new Array(length) as _, index (index)}
		{index} -
		{getPercent(index)}
		|
	{/each} -->
	<div class={twMerge(
		"flex w-full bg-gray-200 overflow-hidden",
		(compact) ? "rounded-none h-3" : "rounded-full h-4",
	)}>
		{#each new Array(length) as _, partIndex (partIndex)}
			<div class="h-full relative border-white {partIndex === 0 ? '' : 'border-l'} w-full">
				{#if partIndex == index && nextInProgress}
					<div
						class="absolute left-0 bottom-0 h-full bg-blue-400/50 animate-pulse"
						style="width: 100%"
					></div>
				{/if}
				{#if partIndex < index - 1}
					<div class="absolute left-0 bottom-0 h-full w-full bg-blue-400"></div>
				{:else if partIndex == index - 1 || (partIndex == index && subIndex !== undefined) || error == partIndex}
					<div
						class="absolute left-0 bottom-0 h-full {error == partIndex
							? 'bg-red-400'
							: 'bg-blue-400'}"
						style="width: {getPercent(partIndex, $percent)}%"
					></div>
				{/if}
			</div>
		{/each}
	</div>
</div>
