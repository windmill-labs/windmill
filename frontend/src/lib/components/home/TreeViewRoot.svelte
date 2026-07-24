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
		sortCompare?: (a: ItemType, b: ItemType) => number
		// The server has further pages beyond the loaded items; `onLoadMore` fetches
		// the next one (grouping only reorders what's already loaded).
		hasMoreServer?: boolean
		onLoadMore?: () => void
		// Lazy per-owner loading: every folder and every user shows as a top-level node
		// regardless of the loaded window; expanding one loads its items on demand,
		// paginated within it. `ownerLoad` keys are full path prefixes (`f/<name>` /
		// `u/<name>`).
		allFolders?: string[]
		allUsers?: string[]
		ownerLoad?: Record<
			string,
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean }
		>
		onExpandOwner?: (owner: string, more?: boolean) => void
		onCollapseOwner?: (owner: string) => void
	}

	let {
		collapseAll,
		showCode,
		nbDisplayed = $bindable(),
		items,
		isSearching = false,
		pipelineFolders,
		sortCompare,
		hasMoreServer = false,
		onLoadMore,
		allFolders = [],
		allUsers = [],
		ownerLoad,
		onExpandOwner,
		onCollapseOwner
	}: Props = $props()

	let groupedItems: ReturnType<typeof groupItems> | 'loading' = $state('loading')
	$effect(() => {
		items
		pipelineFolders
		isSearching
		sortCompare
		allFolders
		allUsers
		untrack(() => {
			// While searching, `items` is already relevance-ranked and the sort
			// selector is disabled, so keep that order: a no-op leaf comparator
			// preserves insertion order within each group (Array.sort is stable).
			const grouped = groupItems(items, isSearching ? () => 0 : sortCompare)
			// Ensure every pipeline folder is present at the top level so its
			// "Pipeline" entry shows even when it has no listed items — a bundle-phase
			// pipeline (only a draft so far) or a folder whose only scripts are
			// pipeline members (folded into the pipeline, hidden from the list).
			// Skip while searching: pipelines aren't part of the text filter (list view
			// hides them on `filter !== ''`), so injecting them would surface unrelated
			// folders in the results.
			if (!isSearching) {
				// Inject a top-level node for every pipeline folder (so its Pipeline entry
				// shows even with no listed items), every workspace folder, and every user
				// — so an owner whose items sit outside the loaded window still appears;
				// expanding one loads its items on demand (see onExpandOwner). Injecting
				// users too is what stops a user node from vanishing under a name sort whose
				// first page is all folder rows.
				const presentFolders = new Set(
					grouped
						.filter((g) => 'folderName' in g)
						.map((g) => (g as { folderName: string }).folderName)
				)
				const missingFolders: { folderName: string; items: [] }[] = []
				for (const folderName of [...(pipelineFolders ?? []), ...allFolders]) {
					if (presentFolders.has(folderName)) continue
					presentFolders.add(folderName)
					missingFolders.push({ folderName, items: [] })
				}
				const presentUsers = new Set(
					grouped.filter((g) => 'username' in g).map((g) => (g as { username: string }).username)
				)
				const missingUsers: { username: string; items: [] }[] = []
				for (const username of allUsers) {
					if (presentUsers.has(username)) continue
					presentUsers.add(username)
					missingUsers.push({ username, items: [] })
				}
				if (missingFolders.length || missingUsers.length) {
					// `groupItems` returns user groups first, then folders alphabetically.
					// Append the missing nodes and sort each section once (O(n log n)) rather
					// than splicing each in with findIndex (O(n²) — at 10k owners that was
					// ~50M comparisons on every page merge).
					const users = grouped.filter((g) => 'username' in g) as { username: string }[]
					const folders = grouped.filter((g) => 'folderName' in g) as { folderName: string }[]
					users.push(...missingUsers)
					folders.push(...missingFolders)
					users.sort((a, b) => a.username.localeCompare(b.username))
					folders.sort((a, b) => a.folderName.localeCompare(b.folderName))
					grouped.length = 0
					grouped.push(
						...(users as unknown as typeof grouped),
						...(folders as unknown as typeof grouped)
					)
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
					{ownerLoad}
					{onExpandOwner}
					{onCollapseOwner}
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
	{#if nbDisplayed < groupedItems.length || hasMoreServer}
		<span class="text-xs font-normal text-secondary"
			>{Math.min(nbDisplayed, groupedItems.length)} root nodes{hasMoreServer
				? ''
				: ` out of ${groupedItems.length}`}
			<button
				class="ml-4 text-xs font-normal text-primary hover:text-emphasis"
				onclick={() => {
					if (nbDisplayed < groupedItems.length) nbDisplayed += 30
					else onLoadMore?.()
				}}>load 30 more</button
			></span
		>
	{/if}
{/if}
