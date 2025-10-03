<script lang="ts">
	import { Tween } from 'svelte/motion'
	import { linear } from 'svelte/easing'
	import { twMerge } from 'tailwind-merge'

	// Remove padding/margin, border radius and titles

	interface Props {
		error?: number | undefined
		index: number
		subIndex: number | undefined
		subLength: number | undefined
		nextInProgress?: boolean
		// Used for displaying progress of subjob of flow
		subIndexIsPercent?: boolean
		// Used in individual job test runs
		compact?: boolean
		// Removes `Step 1` and replaces it with `Running`
		hideStepTitle?: boolean
		length: number
		class?: string
	}

	let {
		error = undefined,
		index,
		subIndex,
		subLength,
		nextInProgress = false,
		subIndexIsPercent = false,
		compact = false,
		hideStepTitle = false,
		length,
		class: className = ''
	}: Props = $props()
	let duration = 200

	let percent = new Tween(0, { duration, easing: linear })

	export function resetP() {
		percent.set(0, { duration: 0 })
	}

	$effect(() => {
		percent.set(
			(length
				? index / length + (subIndex && subLength ? subIndex / (subLength ?? 1) / length : 0)
				: 0) * 100
		)
	})

	function getPercent(partIndex: number, _pct: number) {
		if (!length) {
			return 0
		}

		const res = Math.min((percent.current - (partIndex / length) * 100) * length, 100)
		return res
	}

	let finished = $derived(index == length)
</script>

<div class={className}>
	{#if !compact}
		<div
			class="flex justify-between items-end font-medium mb-1 {error != undefined
				? 'text-red-700 dark:text-red-200'
				: 'text-blue-700 dark:text-blue-200'}"
		>
			<span class="text-base">
				{error != undefined
					? 'Error occurred'
					: finished
						? 'Done'
						: hideStepTitle
							? `Running`
							: subIndexIsPercent
								? `Step ${index + 1} (${subIndex !== undefined ? `${subIndex}%)` : ''}`
								: `Step ${index + 1}${subIndex !== undefined ? `.${subIndex + 1}` : ''}`}
			</span>
			<span class="text-sm">
				{percent.current.toFixed(0)}%
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
	<div
		class={twMerge(
			'flex w-full bg-gray-200 overflow-hidden',
			compact ? 'rounded-none h-3' : 'rounded-full h-4'
		)}
	>
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
						style="width: {getPercent(partIndex, percent.current)}%"
					></div>
				{/if}
			</div>
		{/each}
	</div>
</div>
