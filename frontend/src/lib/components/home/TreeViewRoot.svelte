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
	}

	let {
		collapseAll,
		showCode,
		nbDisplayed = $bindable(),
		items,
		isSearching = false
	}: Props = $props()

	let groupedItems: ReturnType<typeof groupItems> | 'loading' = $state('loading')
	$effect(() => {
		items
		untrack(() => (groupedItems = groupItems(items)))
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
		{#each groupedItems.slice(0, nbDisplayed) as item (item['folderName'] ?? 'user__' + item['username'])}
			{#if item}
				<TreeView
					{isSearching}
					{collapseAll}
					{item}
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
			<button class="ml-4 text-xs font-normal text-primary hover:text-emphasis" onclick={() => (nbDisplayed += 30)}>load 30 more</button></span
		>
	{/if}
{/if}
