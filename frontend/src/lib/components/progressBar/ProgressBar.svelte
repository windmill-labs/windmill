<script lang="ts">
	import { Tween } from 'svelte/motion'
	import { linear } from 'svelte/easing'
	import { twMerge } from 'tailwind-merge'
	import { Loader2 } from 'lucide-svelte'
	import { fade } from 'svelte/transition'

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
		// Makes the progress bar slimmer with smaller text
		slim?: boolean
		length: number
		class?: string
		textPosition?: 'top' | 'bottom'
		// Optional step ID to display when available
		stepId?: string
		// Whether to show the step ID
		showStepId?: boolean
		// Special waiting state display
		isWaitingForEvents?: boolean
		// Whether the job was canceled
		isCanceled?: boolean
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
		slim = false,
		length,
		class: className = '',
		textPosition = 'top',
		stepId,
		showStepId = false,
		isWaitingForEvents = false,
		isCanceled = false
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

	const status: 'error' | 'done' | 'running' = $derived(
		error != undefined ? 'error' : finished ? 'done' : 'running'
	)
</script>

<div class={twMerge('flex flex-col gap-1', className)}>
	{#if textPosition == 'top'}
		{@render text('top')}
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
			compact ? 'rounded-none h-3' : slim ? 'rounded-full h-2.5' : 'rounded-full h-4'
		)}
	>
		{#each new Array(length) as _, partIndex (partIndex)}
			<div class="h-full relative border-white {partIndex === 0 ? '' : 'border-l'} w-full">
				{#if partIndex == index && nextInProgress}
					<div
						class={twMerge(
							'absolute left-0 bottom-0 h-full bg-blue-400/50',
							isCanceled ? '' : ' animate-pulse'
						)}
						style="width: 100%"
						transition:fade={{ duration: 200 }}
					></div>
				{/if}
				{#if partIndex < index - 1}
					<div
						class="absolute left-0 bottom-0 h-full w-full bg-blue-400 transition-colors duration-300 ease-in-out"
					></div>
				{:else if partIndex == index - 1 || (partIndex == index && subIndex !== undefined) || error == partIndex}
					<div
						class="absolute left-0 bottom-0 h-full transition-all duration-300 ease-in-out {error ==
						partIndex
							? 'bg-red-400'
							: 'bg-blue-400'}"
						style="width: {getPercent(partIndex, percent.current)}%"
					></div>
				{/if}
			</div>
		{/each}
	</div>

	{#if textPosition == 'bottom'}
		{@render text('bottom')}
	{/if}
</div>

{#snippet text(position: 'top' | 'bottom')}
	{#if !compact}
		<div
			class="flex justify-between items-end font-medium transition-colors duration-300 ease-in-out {error !=
				undefined || isCanceled
				? 'text-red-700 dark:text-red-200'
				: 'text-blue-700 dark:text-blue-200'}"
		>
			<div class={twMerge(slim ? 'text-xs' : 'text-sm', 'flex items-center gap-1')}>
				{#if status == 'running' && !isCanceled}
					<Loader2 class="animate-spin" size={14} />
				{/if}
				{#key status + isWaitingForEvents + stepId + isCanceled}
					<span in:fade={{ duration: 150 }}>
						{#if status == 'error'}
							Error occurred
						{:else if status == 'done' && isCanceled}
							Canceled
						{:else if status == 'done'}
							Done
						{:else if isWaitingForEvents}
							Waiting to be resumed
						{:else if showStepId}
							{stepId
								? `${isCanceled ? 'Canceled at' : 'Running'} step ${stepId}`
								: `${isCanceled ? 'Canceled' : ''}`}
						{:else if hideStepTitle}
							{isCanceled ? 'Canceled' : 'Running'}
						{:else if subIndexIsPercent}
							{`${isCanceled ? 'Canceled at' : ''} Step ${index + 1} (${subIndex !== undefined ? `${subIndex}%)` : ''}`}
						{:else}
							{`${isCanceled ? 'Canceled at' : ''} Step ${index + 1}${subIndex !== undefined ? `.${subIndex + 1}` : ''}`}
						{/if}
					</span>
				{/key}
			</div>
			<span
				class={twMerge(slim ? 'text-xs' : 'text-sm', 'transition-all duration-200 ease-in-out')}
			>
				{percent.current.toFixed(0)}%
			</span>
		</div>
	{/if}
{/snippet}
