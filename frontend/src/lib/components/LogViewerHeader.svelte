<script lang="ts">
	import { FoldVertical, UnfoldVertical } from 'lucide-svelte'

	interface Props {
		showResultsInputs: boolean | undefined
		toggleExpandAll: () => void
		allExpanded: boolean | undefined
	}

	let { showResultsInputs = $bindable(), toggleExpandAll, allExpanded }: Props = $props()
</script>

<div class="flex justify-end gap-4 items-center p-2 bg-surface-secondary border-b">
	<div class="flex items-center gap-2 whitespace-nowrap">
		<label for="showResultsInputs" class="text-xs text-primary hover:text-primary transition-colors"
			>Show inputs/results</label
		>
		<div class="flex-shrink-0">
			<input
				type="checkbox"
				name="showResultsInputs"
				id="showResultsInputs"
				bind:checked={showResultsInputs}
				class="w-3 h-4 accent-primary -my-1"
			/>
		</div>
	</div>
	<button
		onclick={toggleExpandAll}
		class="text-xs text-primary hover:text-primary transition-colors flex items-center gap-2 min-w-24 justify-end"
	>
		{allExpanded ? 'Collapse All' : 'Expand All'}
		{#if allExpanded}
			<FoldVertical size={16} />
		{:else}
			<UnfoldVertical size={16} />
		{/if}
	</button>
</div>
