<script lang="ts">
	import { untrack } from 'svelte'
	import TreeView from './TreeView.svelte'
	import { groupItems, type ItemType } from './treeViewUtils'

	interface Props {
		collapseAll: boolean
		showCode: (path: string, summary: string) => void
		nbDisplayed: number
		items: ItemType[] | undefined
		isSearching?: boolean
		pipelineFolders?: Set<string>
	}

	let {
		collapseAll,
		showCode,
		nbDisplayed = $bindable(),
		items,
		isSearching = false,
		pipelineFolders
	}: Props = $props()

	let groupedItems: ReturnType<typeof groupItems> | 'loading' = $state('loading')
	$effect(() => {
		items
		pipelineFolders
		isSearching
		untrack(() => {
			const grouped = groupItems(items)
			// Ensure every pipeline folder is present at the top level so its
			// "Pipeline" entry shows even when it has no listed items — a bundle-phase
			// pipeline (only a draft so far) or a folder whose only scripts are
			// pipeline members (folded into the pipeline, hidden from the list).
			// Skip while searching: pipelines aren't part of the text filter (list view
			// hides them on `filter !== ''`), so injecting them would surface unrelated
			// folders in the results.
			if (!isSearching) {
				const present = new Set(
					grouped
						.filter((g) => 'folderName' in g)
						.map((g) => (g as { folderName: string }).folderName)
				)
				// Insert each missing pipeline folder among the existing folders in name
				// order — `groupItems` already sorts user groups first then folders
				// alphabetically, so inserting before the first greater-named folder
				// keeps that ordering (rather than prepending out of order).
				for (const folderName of [...(pipelineFolders ?? [])]
					.filter((f) => !present.has(f))
					.sort()) {
					const item = { folderName, items: [] }
					const idx = grouped.findIndex(
						(g) =>
							'folderName' in g &&
							(g as { folderName: string }).folderName.localeCompare(folderName) > 0
					)
					if (idx < 0) grouped.push(item)
					else grouped.splice(idx, 0, item)
				}
			}
			groupedItems = grouped
		})
	})
</script>

{#if groupedItems === 'loading'}
	<div class="flex flex-row items-center justify-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 dark:border-gray-100"
		></div>
	</div>
{:else if groupedItems.length === 0}
	<div class="flex flex-row items-center justify-center">
		<div class="text-xs font-normal text-hint">No items</div>
	</div>
{:else}
	<div class="border rounded-md bg-surface-tertiary">
		{#each groupedItems.slice(0, nbDisplayed) as item ('folderName' in item ? `f__${item.folderName}` : 'username' in item ? `u__${item.username}` : `i__${item.type}__${item.path}`)}
			{#if item}
				<TreeView
					{isSearching}
					{collapseAll}
					{item}
					{pipelineFolders}
					on:scriptChanged
					on:flowChanged
					on:appChanged
					on:rawAppChanged
					on:reload
					{showCode}
				/>
			{/if}
		{/each}
	</div>
	{#if groupedItems.length > 15 && nbDisplayed < groupedItems.length}
		<span class="text-xs font-normal text-secondary"
			>{nbDisplayed} root nodes out of {groupedItems.length}
			<button
				class="ml-4 text-xs font-normal text-primary hover:text-emphasis"
				onclick={() => (nbDisplayed += 30)}>load 30 more</button
			></span
		>
	{/if}
{/if}
