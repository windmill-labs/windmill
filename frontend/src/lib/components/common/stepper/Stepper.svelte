<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let tabs: string[]
	export let selectedIndex: number = 0
	export let maxReachedIndex: number = -1
	export let statusByStep: Array<'success' | 'error' | 'pending'> = []
	export let hasValidations: boolean = false
	export let allowStepNavigation: boolean = false

	const dispatch = createEventDispatcher()

	function getStepColor(
		index: number,
		selectedIndex: number,
		statusByStep: Array<'success' | 'error' | 'pending'>,
		maxReachedIndex: number
	) {
		if (!hasValidations) {
			if (index === selectedIndex) {
				return 'bg-blue-500 text-white'
			} else if (index > maxReachedIndex) {
				return 'bg-gray-200'
			} else {
				return 'bg-blue-200'
			}
		}

		const current = index === selectedIndex
		if (statusByStep[index] === 'success') {
			return current
				? 'border-green-500 border bg-green-200 text-green-600'
				: 'border-green-500 border'
		} else if (statusByStep[index] === 'error') {
			return current
				? 'border-red-500 border bg-red-200 text-red-600'
				: 'border-red-500 bg-red-100 border'
		} else {
			if (index <= maxReachedIndex) {
				return current
					? 'border-blue-500 border bg-blue-200 text-blue-600'
					: 'border-blue-500 border'
			} else {
				return current
					? 'border-gray-500 border bg-gray-200 text-tertiary'
					: 'border-gray-500 border'
			}
		}
	}
</script>

<div class="flex justify-between">
	<ol class="relative z-20 flex justify-between items-centers text-sm font-medium text-tertiary">
		{#each tabs ?? [] as step, index}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
			<li
				class={classNames(
					'flex items-center gap-2 px-2 py-1 hover:bg-gray-1200 rounded-md m-0.5',
					index <= maxReachedIndex || allowStepNavigation ? 'cursor-pointer' : 'cursor-not-allowed'
				)}
				on:click={() => {
					dispatch('click', { index })
				}}
			>
				{#if statusByStep[index] === 'pending'}
					<Loader2 class="h-6 w-6 animate-spin" />
				{:else}
					<span
						class={classNames(
							'h-6 w-6 rounded-full flex items-center justify-center text-xs',
							getStepColor(index, selectedIndex, statusByStep, maxReachedIndex)
						)}
						class:font-bold={selectedIndex === index}
					>
						{index + 1}
					</span>
				{/if}

				<span
					class={classNames(
						'hidden sm:block',
						selectedIndex === index ? 'font-semibold text-primary' : 'font-normal text-tertiary'
					)}
				>
					{step}
				</span>
			</li>
			{#if index !== (tabs ?? []).length - 1}
				<li class="flex items-center">
					<div class="h-0.5 w-4 bg-blue-200"></div>
				</li>
			{/if}
		{/each}
	</ol>
</div>
