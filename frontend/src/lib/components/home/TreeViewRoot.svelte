<script lang="ts">
	import TreeView from './TreeView.svelte'
	import { groupItems } from './treeViewUtils'

	export let collapseAll: boolean
	export let showCode: (path: string, summary: string) => void
	export let nbDisplayed: number
	export let items: any[] | undefined
	export let isSearching: boolean = false

	let treeLoading = false

	$: groupedItems = grpItems(items)
	function grpItems(items: any[] | undefined): any[] {
		treeLoading = true
		let r
		try {
			r = groupItems(items)
		} finally {
			treeLoading = false
		}
		return r
	}
</script>

{#if treeLoading}
	<div class="flex flex-row items-center justify-center">
		<div
			class="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 dark:border-gray-100"
		></div>
	</div>
{:else if groupedItems.length === 0}
	<div class="flex flex-row items-center justify-center">
		<div class="text-gray-500 dark:text-gray-400">No items</div>
	</div>
{:else}
	<div class="border rounded-md">
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
		<span class="text-xs"
			>{nbDisplayed} root nodes out of {groupedItems.length}
			<button class="ml-4" on:click={() => (nbDisplayed += 30)}>load 30 more</button></span
		>
	{/if}
{/if}
