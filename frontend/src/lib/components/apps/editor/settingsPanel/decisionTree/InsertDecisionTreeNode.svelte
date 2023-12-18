<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Cross, GitBranchPlus } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	const dispatch = createEventDispatcher()

	export let canAddBranch: boolean = true
	export let canAddNode: boolean = true
</script>

<div class="relative flex flex-row gap-1">
	{#if canAddNode}
		<button
			title="Add step"
			on:pointerdown={() => {
				dispatch('node')
			}}
			type="button"
			class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
		>
			<Cross class="mx-[5px]" size={15} />
		</button>
	{/if}
	{#if canAddBranch}
		<button
			title="Add branch"
			type="button"
			on:click={() => dispatch('addBranch')}
			class={twMerge(
				'text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center',
				!canAddNode && 'ml-16 mb-2'
			)}
		>
			<GitBranchPlus class="mx-[5px] rotate-180" size={15} />
		</button>
	{/if}
</div>
