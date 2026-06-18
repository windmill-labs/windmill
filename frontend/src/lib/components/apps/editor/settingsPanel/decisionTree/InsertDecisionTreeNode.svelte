<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Cross, GitBranchPlus } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	const dispatch = createEventDispatcher()

	interface Props {
		canAddBranch?: boolean
		canAddNode?: boolean
	}

	let { canAddBranch = true, canAddNode = true }: Props = $props()
</script>

<div class="relative flex flex-row gap-1">
	{#if canAddNode}
		<button
			title="Add step"
			onpointerdown={() => {
				dispatch('node')
			}}
			type="button"
			class="text-primary bg-surface outline-[1px] outline dark:outline-gray-500 outline-gray-300 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
		>
			<Cross class="mx-[5px]" size={15} />
		</button>
	{/if}
	{#if canAddBranch}
		<button
			title="Add branch"
			type="button"
			onclick={() => dispatch('addBranch')}
			class={twMerge(
				'text-secondary bg-surface outline-[1px] outline dark:outline-gray-500 outline-gray-300 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center',
				!canAddNode && 'ml-16 mb-2'
			)}
		>
			<GitBranchPlus class="mx-[5px] rotate-180" size={15} />
		</button>
	{/if}
</div>
