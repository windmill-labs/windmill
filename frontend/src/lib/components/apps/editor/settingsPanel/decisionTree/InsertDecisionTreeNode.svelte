<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Cross, GitBranchPlus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	export let canAddBranch: boolean = true
</script>

<div class="relative flex flex-row gap-1">
	<button
		title="Add step"
		on:pointerdown={() => {
			dispatch('node')

			close()
		}}
		type="button"
		class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
	>
		<Cross class="mx-[5px]" size={15} />
	</button>

	{#if canAddBranch}
		<button
			title="Add branch"
			type="button"
			on:click={() => dispatch('addBranch')}
			class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
		>
			<GitBranchPlus class="mx-[5px] rotate-180" size={15} />
		</button>
	{/if}
</div>
